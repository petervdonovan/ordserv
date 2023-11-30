use log::info;
use rayon::prelude::*;
use streaming_transpositions::StreamingTranspositions;

use std::{
  collections::{hash_map::DefaultHasher, HashMap},
  ffi::OsString,
  fmt::Display,
  fs::{DirEntry, File},
  hash::{Hash, Hasher},
  io::Write,
  os::unix::prelude::OsStrExt,
  path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
  exec::Executable,
  io::{clean, get_commit_hash, get_counts, get_lf_files_non_recursive, get_traces, TempDir},
  outputvector::{OutputVectorKey, OUTPUT_VECTOR_CHUNK_SIZE},
  testing::AccumulatingTracesState,
  HookInvocationCounts, ThreadId, TraceRecord, Traces, CONCURRENCY_LIMIT,
};

#[derive(Debug, Serialize, Deserialize)]
pub enum State {
  Initial(InitialState),
  Compiled(CompiledState),
  KnownCounts(KnownCountsState),
  AccumulatingTraces(AccumulatingTracesState),
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CommitHash(String);
impl CommitHash {
  pub fn new(hash: String) -> Self {
    Self(hash)
  }
}
impl Display for CommitHash {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct InitialState {
  src_commit: CommitHash,
  pub src_files: HashMap<TestId, PathBuf>,
  scratch_dir: PathBuf,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CompiledState {
  initial: InitialState,
  executables: HashMap<TestId, Executable>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct KnownCountsState {
  cs: CompiledState,
  metadata: HashMap<TestId, TestMetadata>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TestMetadata {
  pub hic: HookInvocationCounts,
  pub out_ovkey: OutputVectorKey,
  pub hook_ovkey: OutputVectorKey,
}

#[derive(Default, Debug, Hash, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct TracePointId(u64);
#[derive(Default, Debug, Hash, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct TestId(u128);

impl TracePointId {
  pub fn new(tr: &TraceRecord) -> Self {
    let mut hash = DefaultHasher::new();
    tr.event.hash(&mut hash);
    tr.reactor.hash(&mut hash);
    tr.source.hash(&mut hash);
    tr.destination.hash(&mut hash);
    tr.elapsed_logical_time.hash(&mut hash);
    tr.microstep.hash(&mut hash);
    tr.trigger.hash(&mut hash);
    tr.extra_delay.hash(&mut hash);
    Self(hash.finish())
  }
}
impl TestId {
  fn new(test: &Path) -> Self {
    let mut hasher = Sha256::new();
    hasher.update(test.as_os_str().as_bytes());
    let hash_array: [u8; 16] = hasher.finalize()[0..16].try_into().expect("impossible");
    let hash128 = u128::from_le_bytes(hash_array);
    Self(hash128)
  }
}

impl Display for TestId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:032x}", self.0)
  }
}

pub fn file_name(scratch: &Path, prefix: &str, src_commit: &CommitHash) -> PathBuf {
  scratch.join(format!("{}-{}.mpk", prefix, src_commit))
}

pub fn file_name_with_total_runs(
  scratch: &Path,
  prefix: &str,
  src_commit: &CommitHash,
  total_runs: usize,
) -> PathBuf {
  scratch.join(format!("{}-{}-{}.mpk", prefix, src_commit, total_runs))
}

impl State {
  const INITIAL_NAME: &'static str = "initial";
  const COMPILED_NAME: &'static str = "compiled";
  pub const KNOWN_COUNTS_NAME: &'static str = "known-counts";
  pub const ACCUMULATING_TRACES_NAME: &'static str = "accumulating-traces";

  fn deserialize_one<T: for<'de> Deserialize<'de>>(de: Vec<&(DirEntry, String)>) -> T {
    if de.len() != 1 {
      panic!("expected exactly one file");
    }
    let ret = rmp_serde::from_read(
      File::open(de.get(0).expect("impossible").0.path()).expect("could not open file"),
    )
    .expect("failed to deserialize");
    ret
  }

  pub fn load(src_dir: PathBuf, scratch_dir: PathBuf) -> Self {
    clean(&scratch_dir);
    let src_commit = get_commit_hash(&src_dir);
    let state_files: Vec<_> = scratch_dir
      .read_dir()
      .expect("failed to read scratch directory")
      .map(|it| it.expect("failed to read entry of scratch directory"))
      .map(|it| {
        let s = it
          .file_name()
          .to_str()
          .expect("os string is not UTF-8")
          .to_string();
        (it, s)
      })
      .filter(|(_, f)| f.contains(&src_commit.to_string()))
      .collect();
    let get_files = |kind: &str| {
      state_files
        .iter()
        .filter(|(_, f)| f.contains(kind))
        .collect()
    };
    let ats_files: Vec<_> = get_files(Self::ACCUMULATING_TRACES_NAME);
    if !ats_files.is_empty() {
      let path = ats_files
        .iter()
        .max_by_key(|(entry, _)| entry.metadata().unwrap().modified().unwrap())
        .unwrap()
        .0
        .path();
      let ret = rmp_serde::from_read(File::open(path).expect("could not open file"))
        .expect("failed to deserialize");
      return Self::AccumulatingTraces(ret);
    }
    let kc_files = get_files(Self::KNOWN_COUNTS_NAME);
    if !kc_files.is_empty() {
      return Self::KnownCounts(State::deserialize_one(kc_files));
    }
    let c_files = get_files(Self::COMPILED_NAME);
    if !c_files.is_empty() {
      return Self::Compiled(State::deserialize_one(c_files));
    }
    let src_files = get_lf_files_non_recursive(&src_dir)
      .into_iter()
      .map(|f| (TestId::new(&f), f))
      .collect();
    Self::Initial(InitialState {
      src_commit,
      src_files,
      scratch_dir,
    })
  }

  fn get_initial_state(&self) -> &InitialState {
    match self {
      Self::Initial(s) => s,
      Self::Compiled(s) => &s.initial,
      Self::KnownCounts(s) => &s.cs.initial,
      Self::AccumulatingTraces(s) => s.get_initial_state(),
    }
  }
  fn file_name(&self) -> PathBuf {
    let scratch_dir = &self.get_initial_state().scratch_dir;
    let src_commit = &self.get_initial_state().src_commit;
    let phase = match self {
      Self::Initial(_) => Self::INITIAL_NAME.to_string(),
      Self::Compiled(_) => Self::COMPILED_NAME.to_string(),
      Self::KnownCounts(_) => Self::KNOWN_COUNTS_NAME.to_string(),
      Self::AccumulatingTraces(ref ats) => {
        return file_name_with_total_runs(
          scratch_dir,
          Self::ACCUMULATING_TRACES_NAME,
          src_commit,
          ats.total_runs(),
        )
      }
    };
    file_name(scratch_dir, &phase, src_commit)
  }
  fn update_saved_up_to_for_saving_deltas(&mut self) {
    if let Self::AccumulatingTraces(ref mut ats) = self {
      ats.update_saved_up_to_for_saving_deltas();
    }
  }
  pub fn save_to_scratch_dir(&mut self) {
    File::create(self.file_name())
      .expect("could not create file")
      .write_all(
        &(match self {
          Self::Initial(x) => rmp_serde::to_vec(x),
          Self::Compiled(x) => rmp_serde::to_vec(x),
          Self::KnownCounts(x) => rmp_serde::to_vec(x),
          Self::AccumulatingTraces(x) => rmp_serde::to_vec(x),
        })
        .expect("could not serialize state"),
      )
      .expect("could not write to file");
    self.update_saved_up_to_for_saving_deltas();
  }
  pub fn run(self, time_seconds: u32) -> Self {
    match self {
      Self::Initial(is) => Self::Compiled(is.compile()),
      Self::Compiled(cs) => Self::KnownCounts(cs.known_counts()),
      Self::KnownCounts(kcs) => Self::AccumulatingTraces(kcs.advance()),
      Self::AccumulatingTraces(mut ats) => {
        ats.accumulate_traces(time_seconds);
        Self::AccumulatingTraces(ats)
      }
    }
  }
}

impl InitialState {
  fn compile(self) -> CompiledState {
    let executables = self
      .src_files
      .par_iter()
      .map(|(id, src)| {
        let mut exe = src.clone();
        loop {
          let do_break = exe.ends_with("src");
          exe = exe
            .parent()
            .expect("could not get parent of exe")
            .to_path_buf();
          if do_break {
            break;
          }
        }
        let src_stem = src.file_stem().expect("could not get file stem");
        exe = exe.join("bin").join(src_stem);
        let lfc_name = format!("lfcpartest-{}", self.src_commit);
        info!("compiling {src:?} with {lfc_name}...",);
        let output = std::process::Command::new(lfc_name)
          .arg(src)
          .arg("--trace")
          .arg("--tracing")
          .arg("--logging")
          .arg("warn")
          .arg("--build-type")
          .arg("release")
          .arg("--fast")
          .output()
          .expect("failed to run lfcpartest");
        if !output.status.success() {
          panic!(
            "failed to compile {}:\n{output:?}\n",
            src.to_str().expect("os string is not UTF-8")
          );
        }
        let exe_renamed = exe
          .parent()
          .expect("executable should have a parent directory")
          .join(format!(
            "{}-{}",
            src_stem
              .to_str()
              .expect("executable file name is not UTF-8"),
            self.src_commit
          ));
        std::fs::rename(exe, &exe_renamed).expect("failed to rename executable");
        (*id, Executable::new(exe_renamed))
      })
      .collect();
    CompiledState {
      initial: self,
      executables,
    }
  }
}

impl CompiledState {
  const ATTEMPTS: u32 = 10;
  fn get_traces_attempts(
    executable: &Executable,
    scratch_dir: &Path,
    tid: ThreadId,
  ) -> (TempDir, Traces) {
    for _ in 0..Self::ATTEMPTS {
      let tmp = TempDir::new(scratch_dir);
      let ret = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(get_traces(
          executable,
          &tmp,
          crate::env::EnvironmentUpdate::new::<OsString>(tid, &[]),
        ));
      if let Ok(ret) = ret {
        return (tmp, ret);
      }
    }
    panic!("could not get metadata for executable");
  }
  fn get_metadata(&self) -> HashMap<TestId, TestMetadata> {
    self
      .executables
      .par_iter()
      .map(|(id, exe)| {
        let (_, mut traces_map) = CompiledState::get_traces_attempts(
          exe,
          &self.initial.scratch_dir,
          ThreadId(rayon::current_thread_index().unwrap()),
        );
        let (hook_trace, out_trace) = traces_map.hooks_and_outs();
        let ic = get_counts(&hook_trace);
        let hook_ovkey =
          OutputVectorKey::new(hook_trace.into_iter().map(|tr| TracePointId::new(&tr)), 1);
        if hook_ovkey.len() != ic.len() {
          panic!(
            "hook_ovkey.len() != ic.len(): {} != {}",
            hook_ovkey.len(),
            ic.len(),
          );
        }
        (
          *id,
          TestMetadata {
            hic: ic,
            out_ovkey: OutputVectorKey::new(
              out_trace.into_iter().map(|tr| TracePointId::new(&tr)),
              OUTPUT_VECTOR_CHUNK_SIZE,
            ),
            hook_ovkey,
          },
        )
      })
      .collect::<HashMap<_, _>>()
  }
  fn known_counts(self) -> KnownCountsState {
    let pool = rayon::ThreadPoolBuilder::new()
      .num_threads(std::cmp::min(
        *CONCURRENCY_LIMIT.wait(),
        self.executables.len(),
      ))
      .build()
      .expect("failed to build thread pool");
    let metadata = pool.install(|| self.get_metadata());
    info!("metadata collected.");
    KnownCountsState { cs: self, metadata }
  }
}

impl KnownCountsState {
  fn advance(self) -> AccumulatingTracesState {
    AccumulatingTracesState::new(self)
  }
  pub fn get_initial_state(&self) -> &InitialState {
    &self.cs.initial
  }
  pub fn tids(&self) -> impl Iterator<Item = &TestId> {
    self.metadata.keys()
  }
  pub fn executables(&self) -> &HashMap<TestId, Executable> {
    &self.cs.executables
  }
  pub fn scratch_dir(&self) -> &Path {
    &self.cs.initial.scratch_dir
  }
  pub fn src_commit(&self) -> &CommitHash {
    &self.cs.initial.src_commit
  }
  pub fn metadata(&self, id: &TestId) -> &TestMetadata {
    self.metadata.get(id).expect("unknown test id")
  }
  pub fn empty_streaming_transpositions_out(&self, tid: &TestId) -> StreamingTranspositions {
    StreamingTranspositions::new(
      self.metadata.get(tid).unwrap().og_ov_length_rounded_up(),
      64,
      0.0025,
    )
  }
  pub fn empty_streaming_transpositions_hook(&self, tid: &TestId) -> StreamingTranspositions {
    StreamingTranspositions::new(self.metadata.get(tid).unwrap().hic.len(), 128, 0.01)
  }
}
