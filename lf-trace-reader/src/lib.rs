use std::path::Path;

use csv::ReaderBuilder;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

pub fn trace_by_physical_time(trace_path: &Path) -> Vec<TraceRecord> {
    let mut records: Vec<_> = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(trace_path)
        .expect("failed to open CSV reader")
        .deserialize::<TraceRecord>()
        .map(|r| r.expect("failed to read record"))
        .collect();
    records.sort_by_key(|it| it.elapsed_physical_time);
    records
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TraceRecord {
    #[serde(rename = "Event")]
    pub event: String,
    #[serde(rename = "Reactor")]
    pub reactor: String,
    #[serde(rename = "Source")]
    pub source: i32,
    #[serde(rename = "Destination")]
    pub destination: i32,
    #[serde(rename = "Elapsed Logical Time")]
    pub elapsed_logical_time: i64,
    #[serde(rename = "Microstep")]
    pub microstep: i64,
    #[serde(rename = "Elapsed Physical Time")]
    pub elapsed_physical_time: i64,
    #[serde(rename = "Trigger")]
    pub trigger: String,
    #[serde(rename = "Extra Delay")]
    pub extra_delay: u64,
    #[serde(rename = "File Index")]
    pub file_index: u32,
    #[serde(rename = "Line Number")]
    pub line_number: u32,
    #[serde(rename = "Sequence Number for File and Line")]
    pub sequence_number_for_file_and_line: u32,
}

impl TraceRecord {
    pub fn mock() -> Self {
        let rng = &mut rand::thread_rng();
        Self {
            event: vec!["A", "B", "C", "D"].choose(rng).unwrap().to_string(),
            reactor: vec!["R", "S", "T", "U"].choose(rng).unwrap().to_string(),
            source: rand::random(),
            destination: rand::random(),
            elapsed_logical_time: rand::random(),
            microstep: rand::random(),
            elapsed_physical_time: rand::random(),
            trigger: vec!["W", "X", "Y", "Z"].choose(rng).unwrap().to_string(),
            extra_delay: rand::random(),
            file_index: rand::random::<u32>() % 10,
            line_number: rand::random::<u32>() % 10,
            sequence_number_for_file_and_line: rand::random::<u32>() % 100,
        }
    }
}
