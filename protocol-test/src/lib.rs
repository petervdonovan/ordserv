mod io;
mod state;

use std::{collections::HashMap, ffi::OsString, fs::File, path::PathBuf, process::Command};

use csv::{Reader, ReaderBuilder};
use rand::prelude::Distribution;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HookId(String);

#[derive(Debug, Serialize, Deserialize)]
pub struct InvocationCounts(HashMap<HookId, u32>);

#[derive(Debug, Default)]
pub struct EnvironmentUpdate(HashMap<OsString, OsString>);

#[derive(Debug)]
pub struct Traces(HashMap<String, Reader<File>>);

#[derive(Debug, Serialize, Deserialize)]
pub struct DelayVector(Vec<u64>);
#[derive(Debug, Serialize, Deserialize)]
pub struct DelayParams {
  pub max_expected_wallclock_overhead: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraceRecord {
  #[serde(rename = "Event")]
  event: String,
  #[serde(rename = "Reactor")]
  reactor: String,
  #[serde(rename = "Source")]
  source: i32,
  #[serde(rename = "Destination")]
  destination: i32,
  #[serde(rename = "Elapsed Logical Time")]
  elapsed_logical_time: i64,
  #[serde(rename = "Microstep")]
  microstep: i64,
  #[serde(rename = "Elapsed Physical Time")]
  elapsed_physical_time: i64,
  #[serde(rename = "Trigger")]
  trigger: String,
  #[serde(rename = "Extra Delay")]
  extra_delay: u64,
}
