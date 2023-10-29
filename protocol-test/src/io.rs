use std::{
  collections::HashMap,
  path::{Path, PathBuf},
  process::Command,
};

use csv::ReaderBuilder;
use rand::{
  distributions::{Alphanumeric, DistString},
  prelude::Distribution,
};
use regex::Regex;

use crate::{
  env::EnvironmentUpdate,
  exec::{ExecResult, Executable},
  state::CommitHash,
  DelayParams, DelayVector, HookId, InvocationCounts, Traces,
};

impl InvocationCounts {
  pub fn len(&self) -> usize {
    self.0.values().map(|it| *it as usize).sum::<usize>()
  }
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }
  pub fn to_vec(&self) -> Vec<(&HookId, &u32)> {
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
fn check_if_deps_up_to_date(src_dir: &Path) {
  let submodule_status = Command::new("git")
    .arg("submodule")
    .arg("status")
    .current_dir(src_dir)
    .output()
    .expect("failed to execute git");
  if !submodule_status.status.success() {
    panic!("failed to check git submodule versions");
  }
  let submodule_status =
    std::str::from_utf8(&submodule_status.stdout).expect("expected output to be UTF-8");
  let commits: Vec<_> = submodule_status
    .lines()
    .filter(|it| !it.is_empty())
    .map(|line| {
      let mut parts = line.split_whitespace();
      parts
        .next()
        .expect("expected submodule status to have at least one part")
    })
    .map(|it| {
      if let Some(s) = it.strip_prefix('+') {
        s
      } else {
        it
      }
    })
    .collect();
  let output = Command::new("RTI")
    .arg("--version")
    .output()
    .expect("failed to check RTI version");
  if !output.status.success() {
    panic!("failed to check RTI version");
  }
  let output = std::str::from_utf8(&output.stdout).expect("expected output to be UTF-8");
  if !commits.iter().any(|it| output.contains(it)) {
    panic!("RTI version is not up to date");
  }
  if !output.contains("dirty") {
    panic!("RTI was built from a dirty repo (i.e., without all changes commmitted)");
  }
}

pub fn get_commit_hash(src_dir: &Path) -> CommitHash {
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
  CommitHash::new(u128::from_str_radix(&s.trim()[..32], 16).expect("failed to parse commit hash"))
}

pub fn get_counts(executable: &Executable, scratch: &Path) -> InvocationCounts {
  let rand_subdir = get_random_subdir(scratch);
  println!("getting invocationcounts for {}...", executable);
  let mut output = executable.run(
    EnvironmentUpdate::new(&[("LF_LOGTRACE", "YES")]),
    &rand_subdir,
    Box::new(|s: &str| s.starts_with("<<<")), // Warning: must define superset of set defined by regex below
  );
  if !output.status.is_success() {
    output.retain_output(|s| s.to_lowercase().contains("fail"));
    println!("Failed to get correct initial counts for {executable:?}. Re-running.");
    println!("summary of failed run:\n{output}");
    std::fs::remove_dir_all(rand_subdir).expect("failed to remove garbage dir");
    return get_counts(executable, scratch);
  }
  let regex = Regex::new(r"<<< (?<HookId>.*) >>>").unwrap();
  let mut ret = HashMap::new();
  for line in output.selected_output.iter() {
    if let Some(caps) = regex.captures(line) {
      let hid = HookId(caps["HookId"].to_string());
      let next = ret.get(&hid).unwrap_or(&0) + 1;
      ret.insert(hid, next);
    }
  }
  std::fs::remove_dir_all(rand_subdir).expect("failed to remove garbage dir");
  InvocationCounts(ret)
}

fn get_random_subdir(scratch: &Path) -> PathBuf {
  let mut rand_subdir = String::from("rand");
  rand_subdir.push_str(&Alphanumeric.sample_string(&mut rand::thread_rng(), 16));
  let rand_subdir = scratch.join(rand_subdir);
  std::fs::create_dir_all(&rand_subdir).expect("failed to create random subdir");
  rand_subdir
    .canonicalize()
    .expect("failed to canonicalize random subdir")
}

pub fn get_traces(
  executable: &Executable,
  scratch: &Path,
  evars: EnvironmentUpdate,
) -> (PathBuf, Result<Traces, ExecResult>) {
  let rand_subdir = get_random_subdir(scratch);
  println!("getting traces for {}...", executable);
  let run = executable.run(
    evars,
    &rand_subdir,
    Box::new(|s: &str| s.to_lowercase().contains("fail")),
  );
  if !run.status.is_success() {
    println!("Failed to get correct traces for {executable}.");
    println!("summary of failed run:\n{run}");
    return (rand_subdir, Err(run));
  }
  for entry in rand_subdir
    .read_dir()
    .expect("failed to read tracefiles from scratch")
    .flatten()
    .filter(|it| it.file_name().to_str().unwrap().ends_with(".lft"))
  {
    Command::new("trace_to_csv")
      .current_dir(&rand_subdir)
      .arg(entry.file_name())
      .output()
      .expect("failed to execute trace_to_csv");
  }
  let mut ret = HashMap::new();
  for entry in rand_subdir
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
        .from_path(rand_subdir.join(entry.file_name()))
        .expect("failed to open CSV reader"),
    );
  }
  (rand_subdir, Ok(Traces(ret)))
}

fn assert_compatible(ic: &InvocationCounts, dvec: &DelayVector) {
  if ic.len() != dvec.0.len() {
    panic!("ic and dvec correspond to a different number of hook invocations");
  }
}

pub fn run_with_parameters(
  executable: &Executable,
  scratch: &Path,
  ic: &InvocationCounts,
  dvec: &DelayVector,
) -> (PathBuf, Result<Traces, ExecResult>) {
  assert_compatible(ic, dvec);
  get_traces(executable, scratch, EnvironmentUpdate::delayed(ic, dvec))
}

pub fn clean(scratch: &Path) {
  for entry in scratch.read_dir().expect("failed to read scratch dir") {
    let entry = entry.expect("failed to read scratch dir entry");
    if entry.file_type().expect("failed to get file type").is_dir()
      && entry.file_name().to_str().unwrap().starts_with("rand")
    {
      println!("removing {}", entry.path().to_str().unwrap());
      std::fs::remove_dir_all(entry.path()).expect("failed to remove scratch dir");
    }
  }
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
      let counts = get_counts(&Executable::new(entry.path()), &scratch_relpath());
      println!("{counts:?}");
    }
  }

  #[test]
  fn test_get_traces() {
    for entry in test_progs() {
      println!("{entry:?}");
      let csvs: HashMap<String, Vec<String>> = get_traces(
        &Executable::new(entry.path()),
        &scratch_relpath(),
        EnvironmentUpdate::default(),
      )
      .1
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
