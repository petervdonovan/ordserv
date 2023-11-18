use std::path::PathBuf;

const THROUGHPUT_FILE_NAME: &str = "plots/throughput.png";
const ERRORS_FILE_NAME: &str = "plots/errors.png";

use plotters::{coord::ranged1d::ValueFormatter, prelude::*};
use protocol_test::testing::{AccumulatingTracesState, AtsDelta};
use viz264::{get_atses, get_latest_ats, get_n_runs_over_time, TestFormatter};

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

fn error_rate(ats: AccumulatingTracesState) {
    let root = BitMapBackend::new(ERRORS_FILE_NAME, (1024, 1024)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let (range, int2id, formatter) = TestFormatter::make(ats.kcs.executables());
    let mut chart = ChartBuilder::on(&root)
        .set_label_area_size(LabelAreaPosition::Left, 200)
        .set_label_area_size(LabelAreaPosition::Bottom, 20)
        .caption("Error Rate by Test", ("serif", 40))
        .build_cartesian_2d(0.0..0.025, range.into_segmented())
        .unwrap();
    chart
        .configure_mesh()
        .disable_x_mesh()
        .bold_line_style(WHITE.mix(0.3))
        .label_style(TextStyle::from(("serif", 14)).color(&BLACK))
        .x_desc("Error rate")
        .y_desc("Test")
        .y_label_formatter(&|i| formatter.format_ext(i))
        .y_labels(int2id.len())
        .draw()
        .unwrap();
    chart
        .draw_series(
            Histogram::horizontal(&chart)
                .style(RED.mix(0.5).filled())
                .data(int2id.iter().enumerate().map(|(n, tid)| {
                    let runs = ats.runs.get(tid).unwrap();
                    let raw_traces = &runs.read().unwrap().raw_traces;
                    let n_errors = raw_traces.iter().filter(|it| it.1.is_err()).count();
                    (n as u32, n_errors as f64 / raw_traces.len() as f64)
                })),
        )
        .unwrap();
    root.present().unwrap()
}

fn main() {
    std::env::set_current_dir("..").unwrap();
    let scratch = &PathBuf::from("scratch");
    let latest = get_latest_ats(scratch);
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
    error_rate(latest);
}
