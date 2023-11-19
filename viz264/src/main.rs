use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

const THROUGHPUT_FILE_NAME: &str = "plots/throughput.png";
const ERRORS_FILE_NAME: &str = "plots/errors.png";

use plotters::{
    coord::ranged1d::{AsRangedCoord, ValueFormatter},
    prelude::*,
};
use protocol_test::{
    outputvector::{OutputVectorKey, OutputVectorRegistry},
    state::{TestMetadata, TracePointId},
    testing::{AccumulatingTracesState, AtsDelta, TestRuns},
};
use statrs::statistics::Statistics;
use viz264::{get_atses, get_latest_ats, get_n_runs_over_time, TestFormatter, TestsRepresentation};

fn runs_over_time_chart(atses: &[AtsDelta]) {
    let data = get_n_runs_over_time(atses);
    println!("{:?}", data);
    let root = BitMapBackend::new(THROUGHPUT_FILE_NAME, (1024, 768)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let (max_x, max_y) = data.iter().max_by_key(|it| it.0 as i64).unwrap();
    let mut chart = ChartBuilder::on(&root)
        .set_label_area_size(LabelAreaPosition::Left, 60)
        .set_label_area_size(LabelAreaPosition::Bottom, 60)
        .caption("Runs over time", ("sans-serif", 40))
        .build_cartesian_2d(0.0..*max_x, 0..*max_y)
        .unwrap();

    chart
        .configure_mesh()
        .x_desc("Time (s)")
        .y_desc("Number of runs")
        .draw()
        .unwrap();

    chart
        .draw_series(plotters::series::LineSeries::new(data, RED.mix(0.2)).point_size(20))
        .unwrap();
    root.present().unwrap();
}

fn histogram_by_test<X, XValue>(
    ats: &AccumulatingTracesState,
    data: impl Iterator<Item = (u32, XValue)>,
    file_name: &str,
    title: &str,
    x_desc: &str,
    x_spec: X,
    trep: &TestsRepresentation,
) where
    X: AsRangedCoord<Value = XValue>,
    <X as plotters::coord::ranged1d::AsRangedCoord>::Value: std::ops::AddAssign,
    <X as plotters::coord::ranged1d::AsRangedCoord>::Value: std::default::Default,
    <X as plotters::coord::ranged1d::AsRangedCoord>::CoordDescType: ValueFormatter<XValue>,
    <X as plotters::coord::ranged1d::AsRangedCoord>::Value: std::fmt::Debug,
{
    let root = BitMapBackend::new(file_name, (1024, 1024)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    // let (range, int2id, formatter) = TestFormatter::make(ats.kcs.executables());
    let mut chart = ChartBuilder::on(&root)
        .set_label_area_size(LabelAreaPosition::Left, 300)
        .set_label_area_size(LabelAreaPosition::Bottom, 20)
        .caption(title, ("serif", 40))
        .build_cartesian_2d(x_spec, trep.0.clone().into_segmented())
        .unwrap();
    chart
        .configure_mesh()
        .disable_x_mesh()
        .bold_line_style(WHITE.mix(0.3))
        .label_style(TextStyle::from(("serif", 14)).color(&BLACK))
        .x_desc(x_desc)
        .y_desc("Test")
        .y_label_formatter(&|i| trep.1.format_ext(i))
        .y_labels(ats.kcs.executables().len())
        .draw()
        .unwrap();
    chart
        .draw_series(
            Histogram::horizontal(&chart)
                .style(RED.mix(0.5).filled())
                .data(data),
        )
        .unwrap();
    root.present().unwrap()
}

fn error_rate(ats: &AccumulatingTracesState) {
    let (int2id, trep) = TestFormatter::make(ats.kcs.executables());
    let data = int2id.iter().enumerate().map(|(n, tid)| {
        let runs = ats.runs.get(tid).unwrap();
        let raw_traces = &runs.read().unwrap().raw_traces;
        let n_errors = raw_traces.iter().filter(|it| it.1.is_err()).count();
        (n as u32, n_errors as f64 / raw_traces.len() as f64)
    });
    histogram_by_test(
        ats,
        data,
        ERRORS_FILE_NAME,
        "Error Rate by Test",
        "Error rate",
        0.0..0.026,
        &trep,
    );
}
struct Orderings {
    imm_before: Vec<HashSet<u32>>,
    imm_after: Vec<HashSet<u32>>,
    before_and_after: Vec<HashSet<u32>>,
}
const SEARCH_RADIUS: i32 = 64;
fn compute_permutable_sets(
    runs: impl std::ops::Deref<Target = TestRuns>,
    metadata: &TestMetadata,
    ovr: &OutputVectorRegistry,
) -> Orderings {
    let mut imm_before = vec![HashSet::new(); metadata.hic.len()];
    let mut imm_after = vec![HashSet::new(); metadata.hic.len()];
    let mut before_and_after = vec![HashSet::new(); metadata.hic.len()];
    for (_, result) in &runs.raw_traces {
        if let Ok((trace, _, _)) = result {
            let mut ogrank_currank_pairs = trace
                .unpack(ovr)
                .into_iter()
                .enumerate()
                .collect::<Vec<_>>();
            ogrank_currank_pairs.sort_by_key(|it| it.1);
            for idx in 0..ogrank_currank_pairs.len() {
                let left_bound = (idx as i32 - SEARCH_RADIUS).max(0) as usize;
                let right_bound = (idx + (SEARCH_RADIUS as usize)).min(ogrank_currank_pairs.len());
                for (other_idx, _) in ogrank_currank_pairs[left_bound..idx].iter() {
                    imm_before[ogrank_currank_pairs[idx].0].insert(*other_idx as u32);
                    if imm_after[*other_idx].contains(&(idx as u32)) {
                        before_and_after[ogrank_currank_pairs[idx].0].insert(*other_idx as u32);
                    }
                }
                for (other_idx, _) in ogrank_currank_pairs[idx + 1..right_bound].iter() {
                    imm_after[ogrank_currank_pairs[idx].0].insert(*other_idx as u32);
                    if imm_before[*other_idx].contains(&(idx as u32)) {
                        before_and_after[ogrank_currank_pairs[idx].0].insert(*other_idx as u32);
                    }
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
        .iter()
        .map(|(testid, runs)| {
            let metadata = ats.kcs.metadata(testid);
            let orderings = compute_permutable_sets(runs.read().unwrap(), metadata, &ats.ovr);
            (testid, orderings)
        })
        .collect();
    let before_and_after_len_stats: Vec<_> = int2id
        .iter()
        .map(|tid| {
            BasicStats::new(
                permutable_sets_by_testid
                    .get(tid)
                    .unwrap()
                    .before_and_after
                    .iter()
                    .map(|it| it.len() as f64),
            )
        })
        .collect();
    for projection in BasicStats::projections() {
        histogram_by_test(
            ats,
            before_and_after_len_stats
                .iter()
                .map(|it| (0, projection.1(&it))),
            &format!("plots/before_and_after_{}.png", projection.0),
            &format!("Before and After {}", projection.0),
            &projection.0,
            0.0..Statistics::max(
                before_and_after_len_stats
                    .iter()
                    .take(10)
                    .map(|it| projection.1(it)),
            ),
            &trep,
        );
    }
}

fn main() {
    std::env::set_current_dir("..").unwrap();
    let scratch = &PathBuf::from("scratch");
    let latest = get_latest_ats(scratch);
    describe_permutable_sets(&latest);
    println!("total runs: {}", latest.total_runs());
    println!("total time: {}", latest.get_dt().as_secs_f64());
    println!(
        "throughput: {}",
        latest.total_runs() as f64 / latest.get_dt().as_secs_f64()
    );
    latest.runs.iter().for_each(|(testid, runs)| {
        println!("testid: {:?}", testid);
        let iomats = &runs.read().unwrap().iomats;
        println!(
            "runs: {} and {}",
            iomats.len(),
            iomats.values().map(|iomat| iomat.len()).sum::<usize>()
        );
    });
    let atses = get_atses(scratch);
    runs_over_time_chart(&atses);
    error_rate(&latest);
}
