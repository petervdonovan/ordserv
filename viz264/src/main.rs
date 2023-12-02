use std::path::PathBuf;

use viz264::{describe_permutable_sets, get_latest_ats};

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();
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
