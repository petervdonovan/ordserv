use std::{collections::HashMap, path::PathBuf};

use protocol_test::{
    outputvector::OutputVectorRegistry,
    state::TestMetadata,
    testing::{AccumulatingTracesState, TestRuns},
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use statrs::statistics::Statistics;
use streaming_transpositions::{OgRank2CurRank, Orderings, StreamingTranspositions};
use viz264::{get_latest_ats, histogram_by_test, TestFormatter};
fn compute_permutable_sets(
    runs: impl std::ops::Deref<Target = TestRuns>,
    metadata: &TestMetadata,
    ovr: &OutputVectorRegistry,
) -> StreamingTranspositions {
    let mut st = StreamingTranspositions::new(metadata.og_ov_length_rounded_up(), 32, 0.01);
    for trace in runs.raw_traces.iter().filter_map(|(_, result)| {
        if let Ok((trace, _, _)) = result {
            Some(trace)
        } else {
            None
        }
    }) {
        let unpacked = trace.unpack(ovr);
        st.record(OgRank2CurRank(&unpacked));
    }
    st
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
                    ordering_projection(permutable_sets_by_testid.get(tid).unwrap().orderings())
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
