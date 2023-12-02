use std::{
  collections::HashMap,
  path::{Path, PathBuf},
  process::Command,
  sync::{Arc, RwLock},
  time::Duration,
};

use csv::ReaderBuilder;
use log::{error, warn};
use ordering_server::{
  server::ServerSubHandle, FederateId, HookInvocation, Precedence, RunId,
  SequenceNumberByFileAndLine, ORDSERV_WAIT_TIMEOUT_MILLISECONDS_ENV_VAR,
};
use rand::distributions::{Alphanumeric, DistString};

use crate::{
  env::EnvironmentUpdate,
  exec::{ExecResult, Executable},
  state::CommitHash,
  testing::TestRuns,
  ConstraintList, HookId, HookInvocationCounts, ThreadId, TraceRecord, Traces,
};

const C_ORDERING_CLIENT_LIBRARY_PATH: &str = "../../target/release/libc_ordering_client.so";
const C_ORDERING_CLIENT_LIBRARY_PATH_ENV_VAR: &str = "C_ORDERING_CLIENT_LIBRARY_PATH";

const ORDSERV_WAIT_TIMEOUT_MILLISECONDS: &str = "25";

pub struct RunContext<'a> {
  pub scratch: &'a Path,
  pub tid: ThreadId,
  pub ordserv: &'a mut ServerSubHandle,
  pub run_id: u32,
}

impl HookInvocationCounts {
  pub fn len(&self) -> usize {
    self.hid2ic.values().map(|it| *it as usize).sum::<usize>()
  }
  pub fn is_empty(&self) -> bool {
    self.len() == 0
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
  CommitHash::new(s.trim()[..32].to_string())
}

fn trace_by_physical_time(trace_path: &PathBuf) -> Vec<TraceRecord> {
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

pub fn get_counts(hook_trace: &[TraceRecord]) -> HookInvocationCounts {
  let mut hid2ic = HashMap::new();
  let mut ogrank2hinvoc = Vec::new();
  let mut process_ids = vec![];
  for record in hook_trace {
    let hid = HookId::new(
      format!("{} {}", record.line_number, record.source),
      FederateId(record.source),
    ); // "source" is a misnomer. It actually means "local federate" regardless of whether it is the source or destination of the message.
    let next = hid2ic.get(&hid).unwrap_or(&0) + 1;
    if next != record.sequence_number_for_file_and_line + 1 {
      panic!(
        "sequence number mismatch at line {}: expected {}, got {}",
        record.line_number,
        next,
        record.sequence_number_for_file_and_line + 1
      );
    }
    hid2ic.insert(hid.clone(), next);
    ogrank2hinvoc.push(HookInvocation {
      hid: hid.clone(),
      seqnum: SequenceNumberByFileAndLine(record.sequence_number_for_file_and_line),
    });
    if !process_ids.contains(&record.source) {
      process_ids.push(record.source);
    }
  }
  let ogrank2hinvoc_len = ogrank2hinvoc.len();
  let ret = HookInvocationCounts {
    hid2ic,
    ogrank2hinvoc,
    n_processes: process_ids.len(),
  };
  assert!(ogrank2hinvoc_len == ret.len());
  ret
}

#[derive(Debug)]
pub struct TempDir(
  pub PathBuf,
  // std::marker::PhantomData<std::sync::MutexGuard<'static, ()>>,
);

impl TempDir {
  pub async fn new(scratch: &Path) -> Self {
    let mut rand_subdir = String::from("rand");
    rand_subdir.push_str(&Alphanumeric.sample_string(&mut rand::thread_rng(), 16));
    let rand_subdir = scratch.join(rand_subdir);
    tokio::fs::create_dir_all(&rand_subdir)
      .await
      .expect("failed to create random subdir");
    TempDir(
      rand_subdir
        .canonicalize()
        .expect("failed to canonicalize random subdir"),
      // std::marker::PhantomData,
    )
  }
  pub fn new_sync(scratch: &Path) -> Self {
    let mut rand_subdir = String::from("rand");
    rand_subdir.push_str(&Alphanumeric.sample_string(&mut rand::thread_rng(), 16));
    let rand_subdir = scratch.join(rand_subdir);
    std::fs::create_dir_all(&rand_subdir).expect("failed to create random subdir");
    TempDir(
      rand_subdir
        .canonicalize()
        .expect("failed to canonicalize random subdir"),
      // std::marker::PhantomData,
    )
  }
  pub fn rand_file(&self, prefix: &str) -> PathBuf {
    let mut rand_file = String::from(prefix);
    rand_file.push_str(&Alphanumeric.sample_string(&mut rand::thread_rng(), 16));
    self.0.join(rand_file)
  }
}

// impl Drop for TempDir {
//   fn drop(&mut self) {
//     if std::thread::panicking() {
//       return; // It is too dangerous to work on the file system while panicking because it may cause a double panic. It is OK to leak the directory.
//     }
//     std::fs::remove_dir_all(&self.0).expect("failed to remove random subdir");
//   }
// }

fn print_repro_instructions(
  executable: &Executable,
  evars: &HashMap<std::ffi::OsString, std::ffi::OsString>,
) {
  warn!("To reproduce, run:");
  warn!(
    "  {evars} {executable} ",
    executable = executable,
    evars = evars
      .iter()
      .map(|(k, v)| format!("{}={}; ", k.to_str().unwrap(), v.to_str().unwrap()))
      .collect::<Vec<_>>()
      .join(" ")
  );
}

pub async fn get_traces(
  executable: &Executable,
  tmp: &TempDir,
  evars: EnvironmentUpdate<'_>,
) -> Result<Traces, ExecResult> {
  let evarsc = evars.get_evars().clone();
  let run = executable
    .run(
      evars,
      tmp,
      // Box::new(|_: &_| true),
      // Box::new(|s: &str| s.to_lowercase().contains("fail")),
      Box::new(|_: &_| false),
    )
    .await;
  if !run.status.is_success() {
    warn!("Failed to get correct traces for {executable}.");
    warn!("summary of failed run:\n{run}");
    print_repro_instructions(executable, &evarsc);
    return Err(run);
  }
  for entry in tmp
    .0
    .read_dir()
    .expect("failed to read tracefiles from scratch")
    .flatten()
    .filter(|it| it.file_name().to_str().unwrap().ends_with(".lft"))
  {
    for retries in 0..5 {
      let result = tokio::process::Command::new("trace_to_csv")
        .current_dir(&tmp.0)
        .arg(entry.file_name())
        .output()
        .await;
      if let Err(e) = result {
        error!(
          "failed to execute trace_to_csv on {:?} in dir {:?} for reasons that I have not taken the time to understand due to being in a hurry. Error:\n    {:?}",
          entry.file_name(),
          &tmp.0,
          e
        );
        tokio::time::sleep(Duration::from_millis(10)).await; // FIXME: horrible hack
      } else {
        break;
      }
      if retries == 4 {
        return Err(run);
      }
    }
  }
  let mut ret = HashMap::new();
  for entry in tmp
    .0
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
        .from_path(tmp.0.join(entry.file_name()))
        .expect("failed to open CSV reader"),
    );
  }
  Ok(Traces(ret))
}

fn assert_compatible(ic: &HookInvocationCounts, conl: &ConstraintList) {
  if ic.len() != conl.length as usize {
    panic!("ic and conl correspond to a different number of hook invocations");
  }
}

pub async fn run_with_parameters(
  executable: &Executable,
  hic: &HookInvocationCounts,
  conl: &ConstraintList,
  clr: Arc<RwLock<TestRuns>>,
  rctx: &mut RunContext<'_>,
) -> (TempDir, Result<Traces, ExecResult>) {
  assert_compatible(hic, conl);
  let tmp = TempDir::new(rctx.scratch).await;
  let unpacked = conl.to_pairs_sorted(&clr.read().unwrap().clr);
  let mut sender2waiters = HashMap::new();
  for (waiter, sender) in unpacked
    .into_iter()
    .filter(|(waiter, sender)| waiter != sender)
  {
    sender2waiters
      .entry(hic.ogrank2hinvoc[sender.idx()].clone())
      .or_insert_with(Vec::new)
      .push(hic.ogrank2hinvoc[waiter.idx()].clone());
  }
  let precedence = Precedence {
    sender2waiters,
    n_connections: hic.n_processes,
    scratch_dir: tmp.0.clone(),
    run_id: RunId(rctx.run_id),
  };
  rctx.ordserv.0.send(Some(precedence)).await.unwrap();
  let mut evars = rctx.ordserv.1.recv().await.unwrap();
  evars.0.push((
    ORDSERV_WAIT_TIMEOUT_MILLISECONDS_ENV_VAR.into(),
    ORDSERV_WAIT_TIMEOUT_MILLISECONDS.into(),
  ));
  evars.0.push((
    C_ORDERING_CLIENT_LIBRARY_PATH_ENV_VAR.into(),
    C_ORDERING_CLIENT_LIBRARY_PATH.into(),
  ));
  let traces = get_traces(executable, &tmp, EnvironmentUpdate::new(rctx.tid, &evars.0)).await;
  (tmp, traces)
}

pub fn clean(scratch: &Path) {
  for entry in scratch.read_dir().expect("failed to read scratch dir") {
    let entry = entry.expect("failed to read scratch dir entry");
    if entry.file_type().expect("failed to get file type").is_dir()
      && entry.file_name().to_str().unwrap().starts_with("rand")
    {
      std::fs::remove_dir_all(entry.path()).expect("failed to remove scratch dir");
    }
  }
}

#[cfg(test)]
mod tests {

  // use crate::TraceRecord;

  // use super::*;
  use std::{
    //ffi::OsString,
    fs::DirEntry,
    path::PathBuf,
  };

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

  // #[test]
  // fn test_get_counts() {
  //   for entry in test_progs() {
  //     let counts = get_counts(
  //       &Executable::new(entry.path()),
  //       &scratch_relpath(),
  //       ThreadId(0),
  //     );
  //     println!("{counts:?}");
  //   }
  // }

  // #[test]
  // fn test_get_traces() {
  //   for entry in test_progs() {
  //     println!("{entry:?}");
  //     let csvs: HashMap<String, Vec<String>> = tokio::runtime::Runtime::new()
  //       .unwrap()
  //       .block_on(get_traces(
  //         &Executable::new(entry.path()),
  //         &TempDir::new(&scratch_relpath()),
  //         EnvironmentUpdate::new::<OsString>(ThreadId(0), &[]),
  //       ))
  //       .unwrap()
  //       .0
  //       .iter_mut()
  //       .map(|(name, reader)| {
  //         println!("{name:?}");
  //         (
  //           name.clone(),
  //           reader
  //             .deserialize()
  //             .map(|r| {
  //               println!("{r:?}");
  //               let r: TraceRecord = r.expect("could not read record");
  //               r.event
  //             })
  //             .collect(),
  //         )
  //       })
  //       .collect();
  //     println!("{csvs:?}");
  //   }
  // }
}
