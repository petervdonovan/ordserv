use rayon::prelude::*;

use std::{
  collections::{hash_map::DefaultHasher, HashMap},
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
  outputvector::OutputVectorKey,
  testing::AccumulatingTracesState,
  DelayParams, HookInvocationCounts, ThreadId, TraceRecord, Traces, CONCURRENCY_LIMIT,
};

#[derive(Debug, Serialize, Deserialize)]
pub enum State {
  Initial(InitialState),
  Compiled(CompiledState),
  KnownCounts(KnownCountsState),
  AccumulatingTraces(AccumulatingTracesState),
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CommitHash(u128);
impl CommitHash {
  pub fn new(hash: u128) -> Self {
    Self(hash)
  }
}
impl Display for CommitHash {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:x?}", self.0)
  }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct InitialState {
  src_commit: CommitHash,
  src_files: HashMap<TestId, PathBuf>,
  scratch_dir: PathBuf,
  delay_params: DelayParams,
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
  pub ovkey: OutputVectorKey,
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

impl State {
  const INITIAL_NAME: &'static str = "initial";
  const COMPILED_NAME: &'static str = "compiled";
  const KNOWN_COUNTS_NAME: &'static str = "known-counts";
  const ACCUMULATING_TRACES_NAME: &'static str = "accumulating-traces";

  pub fn load(src_dir: PathBuf, scratch_dir: PathBuf, delay_params: DelayParams) -> Self {
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
    let deserialize_one = |de: Vec<&(DirEntry, _)>| {
      if de.len() != 1 {
        panic!("expected exactly one file");
      }
      let ret: Self = rmp_serde::from_read(
        File::open(de.get(0).expect("impossible").0.path()).expect("could not open file"),
      )
      .expect("failed to deserialize");
      ret
    };
    let ats_files: Vec<_> = get_files(Self::ACCUMULATING_TRACES_NAME);
    if !ats_files.is_empty() {
      let path = ats_files[0].0.path();
      let ret: Self = rmp_serde::from_read(File::open(path).expect("could not open file"))
        .expect("failed to deserialize");
      return ret;
    }
    let kc_files = get_files(Self::KNOWN_COUNTS_NAME);
    if !kc_files.is_empty() {
      return deserialize_one(kc_files);
    }
    let c_files = get_files(Self::COMPILED_NAME);
    if !c_files.is_empty() {
      return deserialize_one(c_files);
    }
    let src_files = get_lf_files_non_recursive(&src_dir)
      .into_iter()
      .map(|f| (TestId::new(&f), f))
      .collect();
    Self::Initial(InitialState {
      src_commit,
      src_files,
      scratch_dir,
      delay_params,
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
  fn file_name(&self) -> String {
    let phase = match self {
      Self::Initial(_) => Self::INITIAL_NAME.to_string(),
      Self::Compiled(_) => Self::COMPILED_NAME.to_string(),
      Self::KnownCounts(_) => Self::KNOWN_COUNTS_NAME.to_string(),
      Self::AccumulatingTraces(ref ats) => {
        format!("{}-{}", Self::ACCUMULATING_TRACES_NAME, ats.total_runs())
      }
    };
    format!("{}-{}.mpk", phase, self.get_initial_state().src_commit)
  }
  pub fn save_to_scratch_dir(&self) {
    File::create(self.get_initial_state().scratch_dir.join(self.file_name()))
      .expect("could not create file")
      .write_all(&rmp_serde::to_vec(self).expect("could not serialize state"))
      .expect("could not write to file");
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
        println!("compiling {src:?} with {lfc_name}...",);
        let output = std::process::Command::new(lfc_name)
          .arg(src)
          .arg("--trace")
          .arg("--logging")
          .arg("debug")
          .arg("--build-type")
          .arg("release")
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
      let ret = get_traces(
        executable,
        &tmp,
        crate::env::EnvironmentUpdate::new(tid, &[]),
      );
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
        let ic = get_counts(
          exe,
          &self.initial.scratch_dir,
          ThreadId(rayon::current_thread_index().unwrap()),
        );
        let (_, mut traces_map) = CompiledState::get_traces_attempts(
          exe,
          &self.initial.scratch_dir,
          ThreadId(rayon::current_thread_index().unwrap()),
        );
        let traces = traces_map
          .0
          .get_mut("rti.csv")
          .expect("no trace file named rti.csv")
          .deserialize()
          .map(|r| r.expect("could not read record"))
          .map(|tr| TracePointId::new(&tr));
        let ovkey = OutputVectorKey::new(traces);
        (*id, TestMetadata { hic: ic, ovkey })
      })
      .collect::<HashMap<_, _>>()
  }
  fn known_counts(self) -> KnownCountsState {
    let pool = rayon::ThreadPoolBuilder::new()
      .num_threads(std::cmp::min(CONCURRENCY_LIMIT, self.executables.len()))
      .build()
      .expect("failed to build thread pool");
    let metadata = pool.install(|| self.get_metadata());
    println!("metadata collected.");
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
  pub fn metadata(&self, id: &TestId) -> &TestMetadata {
    self.metadata.get(id).expect("unknown test id")
  }
  pub fn delay_params(&self) -> &DelayParams {
    &self.cs.initial.delay_params
  }
}
