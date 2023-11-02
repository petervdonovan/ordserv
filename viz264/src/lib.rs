use std::{fs::File, path::PathBuf};

use protocol_test::{state::State, testing::AccumulatingTracesState};

pub fn get_atses(scratch: &PathBuf) -> Vec<AccumulatingTracesState> {
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
                .contains("4521") // FIXME
        })
    {
        println!("reading {:?}...", entry.path());
        let ats: State = rmp_serde::from_read(File::open(entry.path()).unwrap()).unwrap();
        match ats {
            State::AccumulatingTraces(ats) => atses.push(ats),
            _ => panic!("expected State::AccumulatingTraces"),
        }
    }
    atses
}

pub fn get_n_runs_over_time(atses: &Vec<AccumulatingTracesState>) -> Vec<(f64, usize)> {
    let mut ret = Vec::new();
    for ats in atses {
        ret.push((ats.get_dt().as_secs_f64(), ats.total_runs()));
    }
    ret.sort_by_key(|(_, b)| *b);
    ret
}
