use std::{collections::HashMap, fs::File, path::PathBuf, process::Command};

use csv::{Reader, ReaderBuilder};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct HookId(String);

fn get_counts(executable: PathBuf) -> HashMap<HookId, u32> {
  let output = Command::new(executable.as_os_str())
    .env("LF_LOGTRACE", "YES")
    .output()
    .expect("failed to execute subprocess");
  if !output.status.success() {
    println!("Failed to get correct initial counts for {executable:?}. Re-running.");
    return get_counts(executable);
  }
  let regex = Regex::new(r"<<< (?<HookId>.*) >>>").unwrap();
  let mut ret = HashMap::new();
  for line in std::str::from_utf8(&output.stdout)
    .expect("expected output to be UTF-8")
    .lines()
  {
    if let Some(caps) = regex.captures(line) {
      let hid = HookId(caps["HookId"].to_string());
      let next = ret.get(&hid).unwrap_or(&0) + 1;
      ret.insert(hid, next);
    }
  }
  ret
}

#[derive(Debug, Serialize, Deserialize)]
struct TraceRecord {
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

fn get_traces(executable: PathBuf, scratch: PathBuf) -> HashMap<String, Reader<File>> {
  Command::new(
    executable
      .canonicalize()
      .expect("failed to resolve executable path")
      .as_os_str(),
  )
  .current_dir(scratch.clone())
  .output()
  .expect("failed to execute program to get trace");
  for entry in scratch
    .read_dir()
    .expect("failed to read tracefiles from scratch")
    .flatten()
    .filter(|it| it.file_name().to_str().unwrap().ends_with(".lft"))
  {
    println!("{entry:?}");
    Command::new("trace_to_csv")
      .current_dir(scratch.clone())
      .arg(entry.file_name())
      .output()
      .expect("failed to execute trace_to_csv");
  }
  let mut ret = HashMap::new();
  for entry in scratch
    .read_dir()
    .expect("failed to read csvs from scratch")
    .flatten()
    .filter(|it| {
      let s = it.file_name();
      let s = s.to_str().unwrap();
      s.ends_with(".csv") && !s.contains("summary.csv")
    })
  {
    ret.insert(
      PathBuf::from(entry.file_name())
        .file_name()
        .expect("file name ends with .., which should be impossible")
        .to_str()
        .expect("file name is not valid UTF-8")
        .to_string(),
      ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(scratch.join(entry.file_name()))
        .expect("failed to open CSV reader"),
    );
  }
  ret
}

#[cfg(test)]
mod tests {

  use super::*;
  use std::{fs::DirEntry, path::PathBuf};

  fn tests_relpath() -> PathBuf {
    PathBuf::from("../../../../lingua-franca/test/C/bin")
  }

  fn scratch_relpath() -> PathBuf {
    let ret = PathBuf::from("./scratch");
    std::fs::create_dir_all(&ret).expect("failed to create scratch dir");
    ret
  }

  fn test_progs() -> impl Iterator<Item = DirEntry> {
    tests_relpath()
      .read_dir()
      .expect("read_dir call failed")
      .flatten()
  }

  #[test]
  fn test_get_counts() {
    for entry in test_progs() {
      let counts = get_counts(entry.path());
      println!("{counts:?}");
    }
  }

  #[test]
  fn test_get_traces() {
    for entry in test_progs() {
      println!("{entry:?}");
      let csvs: HashMap<String, Vec<String>> = get_traces(entry.path(), scratch_relpath())
        .iter_mut()
        .map(|(name, reader)| {
          println!("{name:?}");
          (
            name.clone(),
            reader
              .deserialize()
              .map(|r| {
                println!("{r:?}");
                let r: TraceRecord = r.expect("could not read record");
                r.event
              })
              .collect(),
          )
        })
        .collect();
      println!("{csvs:?}");
    }
  }
}
