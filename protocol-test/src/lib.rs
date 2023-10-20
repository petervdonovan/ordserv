use std::{collections::HashMap, ffi::OsString, fs::File, path::PathBuf, process::Command};

use csv::{Reader, ReaderBuilder};
use rand::prelude::Distribution;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct HookId(String);

#[derive(Debug)]
struct InvocationCounts(HashMap<HookId, u32>);

#[derive(Debug, Default)]
struct EnvironmentUpdate(HashMap<OsString, OsString>);

#[derive(Debug)]
struct Traces(HashMap<String, Reader<File>>);

#[derive(Debug)]
struct DelayVector(Vec<u64>);

struct DelayParams {
  pub max_expected_wallclock_overhead: u64,
}

impl InvocationCounts {
  fn len(&self) -> usize {
    self.0.values().map(|it| *it as usize).sum::<usize>()
  }
  fn to_vec(&self) -> Vec<(&HookId, &u32)> {
    let mut keys: Vec<_> = self.0.iter().collect();
    keys.sort();
    keys
  }
}

impl DelayVector {
  fn random(ic: &InvocationCounts, rng: &mut rand::rngs::ThreadRng, dp: DelayParams) -> Self {
    let mut v = vec![];
    v.reserve_exact(ic.len());
    for _ in 0..ic.len() {
      v.push(
        rand::distributions::Uniform::try_from(0..dp.max_expected_wallclock_overhead)
          .unwrap()
          .sample(rng),
      );
    }
    Self(v)
  }
}

fn get_counts(executable: PathBuf) -> InvocationCounts {
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
  InvocationCounts(ret)
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

fn get_traces(executable: PathBuf, scratch: PathBuf, evars: EnvironmentUpdate) -> Traces {
  Command::new(
    executable
      .canonicalize()
      .expect("failed to resolve executable path")
      .as_os_str(),
  )
  .envs(evars.0)
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
  Traces(ret)
}

fn stringify_dvec(dvec: &[u64]) -> OsString {
  OsString::from(dvec.iter().fold(String::from(""), |mut acc, x| {
    acc.push_str(&x.to_string());
    acc
  }))
}

fn assert_compatible(ic: &InvocationCounts, dvec: &DelayVector) {
  if ic.len() != dvec.0.len() {
    panic!("ic and dvec correspond to a different number of hook invocations");
  }
}

fn run_with_parameters(
  executable: PathBuf,
  scratch: PathBuf,
  ic: InvocationCounts,
  dvec: DelayVector,
) -> Traces {
  assert_compatible(&ic, &dvec);
  let mut ev = HashMap::new();
  let mut cumsum: usize = 0;
  for (hid, k) in ic.to_vec() {
    ev.insert(
      OsString::from(hid.0.clone()),
      stringify_dvec(&dvec.0[cumsum..cumsum + (*k as usize)]),
    );
    cumsum += *k as usize;
  }
  get_traces(executable, scratch, EnvironmentUpdate(ev))
}

#[cfg(test)]
mod tests {

  use super::*;
  use std::{fs::DirEntry, path::PathBuf};

  fn tests_relpath() -> PathBuf {
    PathBuf::from("../lf-264/test/C/bin")
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
      let csvs: HashMap<String, Vec<String>> = get_traces(
        entry.path(),
        scratch_relpath(),
        EnvironmentUpdate::default(),
      )
      .0
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
