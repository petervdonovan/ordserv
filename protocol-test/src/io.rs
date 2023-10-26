use std::{
  collections::HashMap,
  ffi::OsString,
  path::{Path, PathBuf},
  process::Command,
};

use csv::ReaderBuilder;
use rand::prelude::Distribution;
use regex::Regex;

use crate::{
  state::Crash, DelayParams, DelayVector, EnvironmentUpdate, HookId, InvocationCounts, Traces,
};

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
  pub fn random(ic: &InvocationCounts, rng: &mut rand::rngs::ThreadRng, dp: &DelayParams) -> Self {
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

pub fn get_lf_files_non_recursive(src_dir: &Path) -> Vec<PathBuf> {
  let mut ret = vec![];
  for entry in src_dir.read_dir().expect("failed to read source directory") {
    let path = entry.expect("failed to read dir entry").path();
    if path.is_file() && path.extension().unwrap_or_default() == "lf" {
      ret.push(path);
    }
  }
  ret
}

fn check_if_clean(src_dir: &Path) {
  let output = Command::new("git")
    .arg("status")
    .arg("--porcelain")
    .current_dir(src_dir)
    .output()
    .expect("failed to execute git");
  if !output.status.success() {
    panic!("failed to check if git repo is clean");
  }
  if !std::str::from_utf8(&output.stdout)
    .expect("expected output to be UTF-8")
    .is_empty()
  {
    panic!("git repo is not clean");
  }
}

pub fn get_commit_hash(src_dir: &Path) -> u128 {
  check_if_clean(src_dir);
  let output = Command::new("git")
    .arg("rev-parse")
    .arg("HEAD")
    .current_dir(src_dir)
    .output()
    .expect("failed to execute git");
  if !output.status.success() {
    panic!("failed to get commit hash");
  }
  let s = std::str::from_utf8(&output.stdout).expect("expected output to be UTF-8");
  u128::from_str_radix(&s.trim()[..32], 16).expect("failed to parse commit hash")
}

pub fn get_counts(executable: &PathBuf) -> InvocationCounts {
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

pub fn get_traces(
  executable: &Path,
  scratch: &Path,
  evars: EnvironmentUpdate,
) -> Result<Traces, Crash> {
  let run = Command::new(
    executable
      .canonicalize()
      .expect("failed to resolve executable path")
      .as_os_str(),
  )
  .envs(evars.0)
  .current_dir(scratch.clone())
  .output()
  .expect("failed to execute program to get trace");
  if !run.status.success() {
    return Err(Crash {
      exit_code: run.status.code().expect("failed to get exit code"),
      stdout: String::from_utf8(run.stdout).expect("expected stdout to be UTF-8"),
      stderr: String::from_utf8(run.stderr).expect("expected stderr to be UTF-8"),
    });
  }
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
  Ok(Traces(ret))
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

pub fn run_with_parameters(
  executable: &Path,
  scratch: &Path,
  ic: &InvocationCounts,
  dvec: &DelayVector,
) -> Result<Traces, Crash> {
  assert_compatible(ic, dvec);
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

  use crate::TraceRecord;

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
      let counts = get_counts(&entry.path());
      println!("{counts:?}");
    }
  }

  #[test]
  fn test_get_traces() {
    for entry in test_progs() {
      println!("{entry:?}");
      let csvs: HashMap<String, Vec<String>> = get_traces(
        &entry.path(),
        &scratch_relpath(),
        EnvironmentUpdate::default(),
      )
      .unwrap()
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
