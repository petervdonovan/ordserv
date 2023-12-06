use std::path::PathBuf;

use viz264::{
    compare_permutable_sets, describe_permutable_sets, error_rate, get_atses, get_latest_ats,
    runs_over_time_chart,
};

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();
    std::env::set_current_dir("..").unwrap();
    let scratch = &PathBuf::from("scratch");
    let latest = get_latest_ats(scratch);
    println!("Data loaded.");
    describe_permutable_sets(&latest);
    // let ats_a = get_latest_ats(&PathBuf::from("scratcha"));
    // let ats_b = get_latest_ats(&PathBuf::from("scratchb"));
    // compare_permutable_sets(&ats_a, &ats_b);

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
    runs_over_time_chart(&atses, "plots/runs_over_time.png");
    error_rate(&latest, "plots/error_rate.png");
}
