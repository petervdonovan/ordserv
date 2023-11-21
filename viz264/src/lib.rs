use std::{collections::HashMap, fs::File, path::PathBuf};

use plotters::{
    backend::BitMapBackend,
    chart::{ChartBuilder, LabelAreaPosition},
    coord::ranged1d::{AsRangedCoord, IntoSegmentedCoord, SegmentValue, ValueFormatter},
    drawing::IntoDrawingArea,
    series::Histogram,
    style::{Color, TextStyle, BLACK, RED, WHITE},
};
use protocol_test::{
    exec::Executable,
    state::TestId,
    testing::{AccumulatingTracesState, AtsDelta},
};

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
    let path = get_latest_ats_file(scratch);
    println!("reading {:?}...", path);
    rmp_serde::from_read(File::open(path).unwrap()).unwrap()
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
        let mut int2id = Vec::new();
        let mut int2exec = Vec::new();
        let mut sorted = executables.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|(_, exec)| exec.name());
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
    let root = BitMapBackend::new(file_name, (1024, 1024)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    // let (range, int2id, formatter) = TestFormatter::make(ats.kcs.executables());
    let mut chart = ChartBuilder::on(&root)
        .set_label_area_size(LabelAreaPosition::Left, 300)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
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
