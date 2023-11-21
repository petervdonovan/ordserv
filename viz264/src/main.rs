use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use protocol_test::{
    outputvector::OutputVectorRegistry,
    state::TestMetadata,
    testing::{AccumulatingTracesState, TestRuns},
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use statrs::statistics::Statistics;
use viz264::{get_latest_ats, histogram_by_test, TestFormatter};

struct Orderings {
    imm_before: Vec<HashSet<u32>>,
    imm_after: Vec<HashSet<u32>>,
    before_and_after: Vec<HashSet<u32>>,
}
type RelatedOgranksGiver<'a> = dyn Fn(&'a Orderings) -> &'a Vec<HashSet<u32>>;
impl Orderings {
    fn projections<'a>() -> Vec<(&'static str, Box<RelatedOgranksGiver<'a>>)> {
        vec![
            ("Before", Box::new(|it: &Self| &it.imm_before)),
            ("After", Box::new(|it| &it.imm_after)),
            ("Before and After", Box::new(|it| &it.before_and_after)),
        ]
    }
}
const SEARCH_RADIUS: i32 = 32;
fn compute_permutable_sets(
    runs: impl std::ops::Deref<Target = TestRuns>,
    metadata: &TestMetadata,
    ovr: &OutputVectorRegistry,
) -> Orderings {
    let mut imm_before = vec![HashSet::new(); metadata.og_ov_length_rounded_up()];
    let mut imm_after = vec![HashSet::new(); metadata.og_ov_length_rounded_up()];
    let mut before_and_after = vec![HashSet::new(); metadata.og_ov_length_rounded_up()];
    for (_, result) in &runs.raw_traces {
        if let Ok((trace, _, _)) = result {
            let ogrank2currank = trace.unpack(ovr);
            let mut ogrank_currank_pairs = ogrank2currank.iter().enumerate().collect::<Vec<_>>();
            ogrank_currank_pairs.sort_by_key(|it| it.1);
            for idx in 0..metadata.og_ov_length_rounded_up() {
                for before_ogrank in imm_before[idx].iter() {
                    if ogrank2currank[*before_ogrank as usize] > ogrank2currank[idx] {
                        before_and_after[idx].insert(*before_ogrank);
                    }
                }
                for after_ogrank in imm_after[idx].iter() {
                    if ogrank2currank[*after_ogrank as usize] < ogrank2currank[idx] {
                        before_and_after[idx].insert(*after_ogrank);
                    }
                }
            }
            for idx in 0..metadata.og_ov_length_rounded_up() {
                let left_bound = (idx as i32 - SEARCH_RADIUS).max(0) as usize;
                let right_bound = (idx + 1 + (SEARCH_RADIUS as usize))
                    .min(metadata.og_ov_length_rounded_up() - 1);
                for (other_idx, _currank) in ogrank_currank_pairs[left_bound..idx].iter() {
                    imm_before[ogrank_currank_pairs[idx].0].insert(*other_idx as u32);
                }
                if idx == metadata.og_ov_length_rounded_up() - 1 {
                    continue;
                }
                for (other_idx, _currank) in ogrank_currank_pairs[idx + 1..right_bound].iter() {
                    imm_after[ogrank_currank_pairs[idx].0].insert(*other_idx as u32);
                }
            }
        }
    }
    Orderings {
        imm_before,
        imm_after,
        before_and_after,
    }
}
#[derive(Debug)]
pub struct BasicStats {
    pub mean: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub upper_quartile: f64,
    pub lower_quartile: f64,
}
type StatProjection = (String, Box<dyn Fn(&BasicStats) -> f64>);
impl BasicStats {
    pub fn new(data: impl Iterator<Item = f64>) -> Self {
        let mut data: Vec<_> = data.collect();
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let len = data.len();
        let mean = data.iter().sum::<f64>() / len as f64;
        let median = data[len / 2];
        let min = data[0];
        let max = data[len - 1];
        let upper_quartile = data[(len * 3) / 4];
        let lower_quartile = data[len / 4];
        Self {
            mean,
            median,
            min,
            max,
            upper_quartile,
            lower_quartile,
        }
    }
    fn projections() -> Vec<StatProjection> {
        vec![
            ("Mean".to_string(), Box::new(|it: &Self| it.mean)),
            ("Median".to_string(), Box::new(|it| it.median)),
            ("Minimum".to_string(), Box::new(|it| it.min)),
            ("Maximum".to_string(), Box::new(|it| it.max)),
            (
                "Upper Quartile".to_string(),
                Box::new(|it| it.upper_quartile),
            ),
            (
                "Lower Quartile".to_string(),
                Box::new(|it| it.lower_quartile),
            ),
        ]
    }
}

fn describe_permutable_sets(ats: &AccumulatingTracesState) {
    let (int2id, trep) = TestFormatter::make(ats.kcs.executables());
    let permutable_sets_by_testid: HashMap<_, _> = ats
        .runs
        .par_iter()
        .map(|(testid, runs)| {
            let metadata = ats.kcs.metadata(testid);
            let orderings = compute_permutable_sets(runs.read().unwrap(), metadata, &ats.ovr);
            (testid, orderings)
        })
        .collect();
    for (ordering_name, ordering_projection) in Orderings::projections() {
        let projected_stats: Vec<_> = int2id
            .iter()
            .map(|tid| {
                BasicStats::new(
                    ordering_projection(permutable_sets_by_testid.get(tid).unwrap())
                        .iter()
                        .map(|it| {
                            it.len() as f64 / ats.kcs.metadata(tid).ovkey.n_tracepoints as f64
                        }),
                )
            })
            .collect();
        for projection in BasicStats::projections() {
            histogram_by_test(
                ats,
                projected_stats
                    .iter()
                    .enumerate()
                    .map(|(idx, stats)| (idx as u32, projection.1(stats))),
                &format!("plots/{}_{}.png", ordering_name, projection.0),
                &format!("{} {}", ordering_name, projection.0),
                &projection.0,
                0.0..Statistics::max(projected_stats.iter().take(10).map(|it| projection.1(it))),
                &trep,
            );
        }
    }
}

fn main() {
    std::env::set_current_dir("..").unwrap();
    let scratch = &PathBuf::from("scratch");
    let latest = get_latest_ats(scratch);
    println!("Data loaded.");
    describe_permutable_sets(&latest);
    // println!("total runs: {}", latest.total_runs());
    // println!("total time: {}", latest.get_dt().as_secs_f64());
    // println!(
    //     "throughput: {}",
    //     latest.total_runs() as f64 / latest.get_dt().as_secs_f64()
    // );
    // latest.runs.iter().for_each(|(testid, runs)| {
    //     println!("testid: {:?}", testid);
    //     let iomats = &runs.read().unwrap().iomats;
    //     println!(
    //         "runs: {} and {}",
    //         iomats.len(),
    //         iomats.values().map(|iomat| iomat.len()).sum::<usize>()
    //     );
    // });
    // let atses = get_atses(scratch);
    // runs_over_time_chart(&atses);
    // error_rate(&latest);
}
