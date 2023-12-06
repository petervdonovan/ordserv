use std::{cmp::Ordering, collections::HashMap, fs::File, path::PathBuf};

const MAX_RUNS_TO_CONSIDER: usize = 5000;

pub mod stats;

use plotters::{
    backend::BitMapBackend,
    chart::{ChartBuilder, LabelAreaPosition},
    coord::ranged1d::{AsRangedCoord, IntoSegmentedCoord, SegmentValue, ValueFormatter},
    drawing::IntoDrawingArea,
    element::Rectangle,
    series::{Histogram, LineSeries},
    style::{Color, Palette, Palette99, TextStyle, BLACK, RED, WHITE},
};
use protocol_test::{
    exec::Executable,
    outputvector::OutputVectorRegistry,
    state::{TestId, TestMetadata},
    testing::{AccumulatingTracesState, AtsDelta, TestRuns},
};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use statrs::statistics::Statistics;
use stats::BasicStats;
use streaming_transpositions::{OgRank2CurRank, StreamingTranspositions};

pub fn get_atses(scratch: &PathBuf) -> Vec<AtsDelta> {
    let mut atses = Vec::new();
    for entry in std::fs::read_dir(scratch)
        .expect("failed to read scratch dir")
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.path().is_file())
        .filter(|entry| {
            entry
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("accumulating-traces")
        })
    {
        println!("reading {:?}...", entry.path());
        let ats: AtsDelta = rmp_serde::from_read(File::open(entry.path()).unwrap()).unwrap();
        atses.push(ats);
    }
    atses
}

fn get_latest_ats_file(scratch: &PathBuf) -> PathBuf {
    std::fs::read_dir(scratch)
        .expect("failed to read scratch dir")
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.path().is_file())
        .filter(|entry| {
            entry
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("accumulating-traces")
        })
        .max_by_key(|entry| entry.path().metadata().unwrap().modified().unwrap())
        .unwrap()
        .path()
}

pub fn get_latest_ats(scratch: &PathBuf) -> AccumulatingTracesState {
    let target = PathBuf::from("scratch");
    if !scratch.ends_with("scratch") {
        assert!(!target.exists());
        std::fs::rename(scratch, &target).unwrap();
    }
    let path = get_latest_ats_file(&target);
    println!("reading {:?}...", path);
    let ret = rmp_serde::from_read(File::open(path).unwrap()).unwrap();
    if !scratch.ends_with("scratch") {
        std::fs::rename(target, scratch).unwrap();
    }
    ret
}

pub fn get_n_runs_over_time(atses: &[AtsDelta]) -> Vec<(f64, usize)> {
    let mut ret = Vec::new();
    for ats in atses {
        ret.push((ats.dt.as_secs_f64(), ats.total_runs));
    }
    ret.sort_by_key(|(_, b)| *b);
    ret
}

pub struct TestFormatter(Vec<String>);
pub type TestsRepresentation = (plotters::coord::types::RangedCoordu32, TestFormatter);

impl TestFormatter {
    pub fn make(
        executables: &HashMap<TestId, Executable>,
    ) -> (Vec<TestId>, (plotters::coord::types::RangedCoordu32, Self)) {
        Self::make_with_sort(executables, |(_, exec)| exec.name())
    }
    pub fn make_with_sort<T>(
        executables: &HashMap<TestId, Executable>,
        sort_by: impl Fn(&(&TestId, &Executable)) -> T,
    ) -> (Vec<TestId>, (plotters::coord::types::RangedCoordu32, Self))
    where
        T: PartialOrd,
    {
        let mut int2id = Vec::new();
        let mut int2exec = Vec::new();
        let mut sorted = executables.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| {
            sort_by(a)
                .partial_cmp(&sort_by(b))
                .unwrap_or(Ordering::Equal)
        });
        for (id, exec) in sorted.into_iter().rev() {
            int2id.push(*id);
            int2exec.push(exec.name());
        }
        (
            int2id,
            ((0..executables.len() as u32).into(), Self(int2exec)),
        )
    }
    fn stringify(&self, value: &u32) -> String {
        self.0
            .get(*value as usize)
            .unwrap_or(&"".to_string())
            .clone()
    }
}

impl ValueFormatter<SegmentValue<u32>> for TestFormatter {
    fn format_ext(&self, value: &SegmentValue<u32>) -> String {
        match value {
            SegmentValue::CenterOf(ref value) => self.stringify(value),
            _ => "".to_string(),
        }
    }
}

pub fn runs_over_time_chart(atses: &[AtsDelta], throughput_file_name: &str) {
    let data = get_n_runs_over_time(atses);
    println!("{:?}", data);
    let root = BitMapBackend::new(throughput_file_name, (1024, 768)).into_drawing_area();
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

pub fn histogram_by_test<X, XValue>(
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
    std::fs::create_dir_all(PathBuf::from(file_name).parent().unwrap()).unwrap();
    let root = BitMapBackend::new(file_name, (2048, 2048)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    // let (range, int2id, formatter) = TestFormatter::make(ats.kcs.executables());
    let mut chart = ChartBuilder::on(&root)
        .set_label_area_size(LabelAreaPosition::Left, 640)
        .set_label_area_size(LabelAreaPosition::Bottom, 100)
        .caption(title, ("serif", 72))
        .build_cartesian_2d(x_spec, trep.0.clone().into_segmented())
        .unwrap();
    chart
        .configure_mesh()
        .max_light_lines(5)
        // .disable_y_mesh()
        .bold_line_style(BLACK.mix(0.5))
        .label_style(TextStyle::from(("serif", 28)).color(&BLACK))
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

pub struct AxesDescriptions<X, Y> {
    pub x_desc: &'static str,
    pub x_spec: X,
    pub y_desc: &'static str,
    pub y_spec: Y,
}

pub fn plot_by_test<X, XValue: Clone + 'static, Y, YValue: Clone + 'static>(
    data: impl Iterator<Item = impl Iterator<Item = (XValue, YValue)> + Clone>,
    file_name: &str,
    title: &str,
    ad: AxesDescriptions<X, Y>,
    trep: &TestsRepresentation,
) where
    Y: AsRangedCoord<Value = YValue>,
    <Y as plotters::coord::ranged1d::AsRangedCoord>::Value: std::ops::AddAssign,
    <Y as plotters::coord::ranged1d::AsRangedCoord>::Value: std::default::Default,
    <Y as plotters::coord::ranged1d::AsRangedCoord>::CoordDescType: ValueFormatter<YValue>,
    <Y as plotters::coord::ranged1d::AsRangedCoord>::Value: std::fmt::Debug,
    X: AsRangedCoord<Value = XValue>,
    <X as plotters::coord::ranged1d::AsRangedCoord>::Value: std::ops::AddAssign,
    <X as plotters::coord::ranged1d::AsRangedCoord>::Value: std::default::Default,
    <X as plotters::coord::ranged1d::AsRangedCoord>::CoordDescType: ValueFormatter<XValue>,
    <X as plotters::coord::ranged1d::AsRangedCoord>::Value: std::fmt::Debug,
{
    std::fs::create_dir_all(PathBuf::from(file_name).parent().unwrap()).unwrap();
    let root = BitMapBackend::new(file_name, (1024, 1024)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let mut chart = ChartBuilder::on(&root)
        .set_label_area_size(LabelAreaPosition::Left, 60)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .caption(title, ("serif", 40))
        .build_cartesian_2d(ad.x_spec, ad.y_spec)
        .unwrap();
    chart
        .configure_mesh()
        .bold_line_style(WHITE.mix(0.3))
        .label_style(TextStyle::from(("serif", 14)).color(&BLACK))
        .y_desc(ad.y_desc)
        .x_desc(ad.x_desc)
        .draw()
        .unwrap();
    for (idx, series) in data.enumerate() {
        let color = Palette99::pick(idx).mix(0.9);
        chart
            .draw_series(LineSeries::new(series, color.stroke_width(3)))
            .unwrap()
            .label(trep.1.stringify(&(idx as u32)))
            .legend(move |(x, y)| Rectangle::new([(x, y - 10), (x + 20, y + 10)], color.filled()));
    }
    chart
        .configure_series_labels()
        .border_style(BLACK)
        .draw()
        .unwrap();
    root.present().unwrap()
}

pub fn error_rate(ats: &AccumulatingTracesState, errors_file_name: &str) {
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
        errors_file_name,
        "Error Rate by Test",
        "Error rate",
        0.0..0.026,
        &trep,
    );
}

fn compute_permutable_sets(
    runs: impl std::ops::Deref<Target = TestRuns> + Sync,
    metadata: &TestMetadata,
    ovr: &OutputVectorRegistry,
) -> StreamingTranspositions {
    let stride = 128; // Relevant to performance
    let stop_at = runs.raw_traces.len().min(MAX_RUNS_TO_CONSIDER);
    StreamingTranspositions::new(metadata.og_ov_length_rounded_up(), 10000, 0.000001)
        .par_record_all((0..(stop_at / stride).max(1)).into_par_iter().map(|start| {
            runs.raw_traces[(start * stride)..(start * stride + stride).min(stop_at)]
                .iter()
                .filter_map(|(_, result)| {
                    if let Ok((trace, _, _)) = result {
                        Some(trace)
                    } else {
                        None
                    }
                })
                .map(|trace| OgRank2CurRank(trace.unpack(ovr)))
        }))
}

fn get_permutable_sets_by_testid(
    ats: &AccumulatingTracesState,
) -> HashMap<TestId, StreamingTranspositions> {
    ats.runs
        .par_iter()
        .map(|(testid, runs)| {
            let metadata = ats.kcs.metadata(testid);
            let orderings = compute_permutable_sets(runs.read().unwrap(), metadata, &ats.ovr);
            (*testid, orderings)
        })
        .collect()
}

fn get_projected_stats(
    ats: &AccumulatingTracesState,
    permutable_sets_by_testid: &HashMap<TestId, StreamingTranspositions>,
) -> HashMap<TestId, BasicStats> {
    ats.runs
        .keys()
        .map(|tid| {
            (
                *tid,
                BasicStats::new(
                    permutable_sets_by_testid
                        .get(tid)
                        .unwrap()
                        .orderings()
                        .iter()
                        .map(|it| {
                            it.len() as f64 / ats.kcs.metadata(tid).out_ovkey.n_tracepoints as f64
                        }),
                ),
            )
        })
        .collect()
}

fn get_comparison_stats(
    permutable_sets_by_testid_a: &HashMap<TestId, StreamingTranspositions>,
    permutable_sets_by_testid_b: &HashMap<TestId, StreamingTranspositions>,
) -> HashMap<TestId, BasicStats> {
    permutable_sets_by_testid_a
        .keys()
        .map(|tid| {
            (
                *tid,
                BasicStats::new(
                    permutable_sets_by_testid_a
                        .get(tid)
                        .unwrap()
                        .orderings()
                        .iter()
                        .zip(
                            permutable_sets_by_testid_b
                                .get(tid)
                                .unwrap()
                                .orderings()
                                .iter(),
                        )
                        .map(|(a, b)| {
                            let ret = (a.len() as f64).log2() - (b.len() as f64).log2();
                            print!("# {} / {} -> {}   ", a.len(), b.len(), ret);
                            ret
                        }),
                ),
            )
        })
        .collect()
}

pub fn compare_permutable_sets(ats_a: &AccumulatingTracesState, ats_b: &AccumulatingTracesState) {
    let permutable_sets_by_testid_a = get_permutable_sets_by_testid(ats_a);
    let permutable_sets_by_testid_b = get_permutable_sets_by_testid(ats_b);
    let comparison_stats =
        get_comparison_stats(&permutable_sets_by_testid_a, &permutable_sets_by_testid_b);
    let (int2id, trep) = TestFormatter::make_with_sort(ats_a.kcs.executables(), |(tid, _)| {
        comparison_stats.get(tid).unwrap().mean
    });
    for projection in BasicStats::projections() {
        let iterator = || {
            int2id
                .iter()
                .map(|tid| projection.1(&comparison_stats[tid]))
        };
        let width = iterator().abs_max() * 1.1;
        println!("plotting {}", projection.0);
        histogram_by_test(
            ats_a,
            iterator()
                .enumerate()
                .map(|(idx, value)| (idx as u32, value)),
            &format!("plots/permutable_pairs_{}.png", projection.0),
            &format!(
                "{} Log2 Ratio of Number of Known Unordered Pairs",
                projection.0.replace(' ', "")
            ),
            &projection.0,
            -width..width,
            &trep,
        );
        println!("done");
    }
}

// pub fn compare_permutable_sets(ats_a: &AccumulatingTracesState, ats_b: &AccumulatingTracesState) {
//     let permutable_sets_by_testid_a = get_permutable_sets_by_testid(ats_a);
//     let permutable_sets_by_testid_b = get_permutable_sets_by_testid(ats_b);
//     let projected_stats_a = get_projected_stats(ats_a, &permutable_sets_by_testid_a);
//     let projected_stats_b = get_projected_stats(ats_b, &permutable_sets_by_testid_b);
//     let (int2id, trep) = TestFormatter::make_with_sort(ats_a.kcs.executables(), |(tid, _)| {
//         projected_stats_a.get(tid).unwrap().mean - projected_stats_b.get(tid).unwrap().mean
//     });
//     for projection in BasicStats::projections() {
//         println!("plotting {}", projection.0);
//         histogram_by_test(
//             ats_a,
//             int2id
//                 .iter()
//                 .map(|tid| {
//                     (
//                         tid,
//                         projection.1(&projected_stats_a[tid]).log10()
//                             - projection.1(&projected_stats_b[tid]).log10(),
//                     )
//                 })
//                 .enumerate()
//                 .map(|(idx, (tid, value))| (idx as u32, value)),
//             &format!("plots/permutable_pairs_{}.png", projection.0),
//             &format!("{} Number of Permutable Pairs", projection.0),
//             &projection.0,
//             -1.0..1.0,
//             &trep,
//         );
//         println!("done");
//     }
// }

pub fn describe_permutable_sets(ats: &AccumulatingTracesState) {
    let permutable_sets_by_testid = get_permutable_sets_by_testid(ats);
    let maxes_by_testid = permutable_sets_by_testid
        .iter()
        .map(|(testid, st)| (testid, st.cumsums().last().unwrap().1))
        .collect::<HashMap<_, _>>();
    let projected_stats = get_projected_stats(ats, &permutable_sets_by_testid);
    let (int2id, trep) = TestFormatter::make_with_sort(ats.kcs.executables(), |(tid, _)| {
        -projected_stats.get(tid).unwrap().mean
    });
    for projection in BasicStats::projections() {
        println!("plotting {}", projection.0);
        histogram_by_test(
            ats,
            int2id
                .iter()
                .map(|tid| projected_stats[tid])
                .enumerate()
                .map(|(idx, stats)| (idx as u32, projection.1(&stats))),
            &format!(
                "plots/permutable_pairs_{}.png",
                projection.0.replace(' ', "")
            ),
            &format!("{} Number of Unordered Pairs", projection.0),
            &projection.0,
            0.0..Statistics::max(projected_stats.values().map(|it| projection.1(it))),
            &trep,
        );
        println!("done");
    }
    let (int2id, trep) = TestFormatter::make_with_sort(ats.kcs.executables(), |(tid, _)| {
        *maxes_by_testid.get(tid).unwrap()
    });
    let time_series = int2id.iter().map(|tid| {
        let st = permutable_sets_by_testid.get(tid).unwrap();
        let max = maxes_by_testid.get(&tid).unwrap().0 as f64;
        st.cumsums().map(move |(ntraces, cumsum)| {
            let remaining = (max - (cumsum.0 as f64)) / max;
            let logged = if remaining.abs() < 1e-6 {
                -6.0
            } else {
                remaining.log10()
            };
            (ntraces.0, logged)
        })
    });
    let xmax = permutable_sets_by_testid
        .values()
        .map(|st| st.traces_recorded().0)
        .max()
        .unwrap();
    // let ymax = permutable_sets_by_testid
    //     .values()
    //     .map(|st| st.cumsums().last().unwrap().1 .0)
    //     .max()
    //     .unwrap();
    println!("plotting cumsums");
    plot_by_test(
        time_series,
        "plots/cumsums.png",
        "Number of Pairs Shown to be Unordered by Test",
        AxesDescriptions {
            x_desc: "Number of Traces",
            x_spec: 0..xmax,
            y_desc: "Number of Pairs",
            y_spec: -5.05..0.05,
        },
        &trep,
    );
}
