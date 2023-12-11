use std::path::PathBuf;

use viz264::{
    compare_permutable_sets, describe_permutable_sets, error_rate, get_atses, get_latest_ats,
    get_trace_ords, runs_over_time_chart,
};

fn do_compare_permutable_sets() {
    let ats_a = get_latest_ats(&PathBuf::from("scratcha"));
    let ats_b = get_latest_ats(&PathBuf::from("scratchb"));
    compare_permutable_sets(&ats_a, &ats_b);
}

fn do_describe_permutable_sets() {
    let ats = get_latest_ats(&PathBuf::from("scratch"));
    describe_permutable_sets(&ats);
}

fn do_throughput_and_error_rate() {
    let scratch = &PathBuf::from("scratch");
    let atses = get_atses(scratch);
    let latest = get_latest_ats(scratch);
    runs_over_time_chart(&atses, "plots/runs_over_time.png");
    error_rate(&latest, "plots/error_rate.png");
}

fn do_computed_precedences() {
    let cp_path = PathBuf::from("../trace-ord/datasets/computed_precedences.mpk");
    let scratch = &PathBuf::from("scratch");
    let cp = get_trace_ords(&cp_path);
    let latest = get_latest_ats(scratch);
    for (tid, runs) in latest.runs {
        let strans_out = &runs.read().unwrap().strans_out;
    }
}

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();
    std::env::set_current_dir("..").unwrap();
    // do_throughput_and_error_rate();
    // do_describe_permutable_sets();
    // do_compare_permutable_sets();
}
