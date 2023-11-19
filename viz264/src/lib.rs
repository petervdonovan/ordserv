use std::{collections::HashMap, fs::File, path::PathBuf};

use plotters::coord::ranged1d::{AsRangedCoord, SegmentValue, ValueFormatter};
use protocol_test::{
    exec::Executable,
    state::{TestId, TestMetadata},
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
