use std::{
  collections::{hash_map::DefaultHasher, HashMap},
  fs::{DirEntry, File},
  hash::{Hash, Hasher},
  io::Write,
  os::unix::prelude::OsStrExt,
  path::{Path, PathBuf},
};

use ndarray::Array2;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
  io::{get_commit_hash, get_counts, get_lf_files_non_recursive, get_traces, run_with_parameters},
  DelayParams, DelayVector, InvocationCounts, TraceRecord, Traces,
};

#[derive(Debug, Serialize, Deserialize)]
pub enum State {
  Initial(InitialState),
  Compiled(CompiledState),
  KnownCounts(KnownCountsState),
  AccumulatingTraces(AccumulatingTracesState),
}
#[derive(Debug, Serialize, Deserialize)]
pub struct InitialState {
  src_commit: u128,
  src_files: HashMap<TestId, PathBuf>,
  scratch_dir: PathBuf,
  delay_params: DelayParams,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CompiledState {
  initial: InitialState,
  executables: HashMap<TestId, PathBuf>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct KnownCountsState {
  cs: CompiledState,
  metadata: HashMap<TestId, TestMetadata>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AccumulatingTracesState {
  kcs: KnownCountsState,
  runs: HashMap<TestId, TestRuns>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TestMetadata {
  ic: InvocationCounts,
  ovkey: OutputVectorKey,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TestRuns {
  raw_traces: Vec<(DelayVector, Result<SuccessfulRun, Crash>)>,
  #[serde(skip)] // derived from raw_traces
  iomat_global: Array2<i64>,
  iomats: HashMap<CoarseTraceHash, HashMap<FineTraceHash, u64>>,
}

#[derive(Default, Debug, Hash, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
struct TracePointId(u64);
#[derive(Default, Debug, Hash, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
struct TestId(u128);

impl TracePointId {
  fn new(tr: &TraceRecord) -> Self {
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
#[derive(Debug, Serialize, Deserialize)]
struct OutputVector(Vec<i64>);
#[derive(Debug, Serialize, Deserialize)]
struct OutputVectorKey(HashMap<TracePointId, Vec<usize>>);

type SuccessfulRun = (OutputVector, TraceHash, VectorfyStatus);
#[derive(Debug, Serialize, Deserialize)]
pub struct Crash {
  pub exit_code: i32,
  pub stdout: String,
  pub stderr: String,
}

impl OutputVectorKey {
  fn new(tpis: impl Iterator<Item = TracePointId>) -> Self {
    let mut ret = HashMap::new();
    for (idx, tpi) in tpis.enumerate() {
      ret.entry(tpi).or_insert(vec![]).push(idx);
    }
    Self(ret)
  }
  fn vectorfy(&self, records: impl Iterator<Item = TraceRecord>) -> SuccessfulRun {
    let mut ov = vec![];
    let mut th = TraceHasher::new();
    let mut status = VectorfyStatus::Ok;
    let mut subidxs = HashMap::new();
    for tr in records {
      let tpi = TracePointId::new(&tr);
      if let Some(idxs) = self.0.get(&tpi) {
        subidxs.entry(tpi).or_insert(0);
        if let Some(idx) = idxs.get(subidxs[&tpi]) {
          ov[*idx] = tr.elapsed_physical_time;
        } else {
          status = VectorfyStatus::ExtraTracePointId;
        }
      } else {
        status = VectorfyStatus::MissingTracePointId;
      }
      th.update(&tr);
    }
    (OutputVector(ov), th.finish(), status)
  }
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
struct CoarseTraceHash(u64);
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
struct FineTraceHash(u64);
#[derive(Debug, Serialize, Deserialize)]
struct TraceHash(CoarseTraceHash, FineTraceHash);

struct TraceHasher {
  coarse: DefaultHasher,
  fine: DefaultHasher,
}

impl TraceHasher {
  fn new() -> Self {
    Self {
      coarse: DefaultHasher::new(),
      fine: DefaultHasher::new(),
    }
  }
  fn update(&mut self, tr: &TraceRecord) {
    tr.event.hash(&mut self.coarse);
    tr.destination.hash(&mut self.coarse);
    tr.elapsed_logical_time.hash(&mut self.coarse);
    tr.microstep.hash(&mut self.coarse);
    tr.event.hash(&mut self.fine);
    tr.reactor.hash(&mut self.fine);
    tr.source.hash(&mut self.fine);
    tr.destination.hash(&mut self.fine);
    tr.elapsed_logical_time.hash(&mut self.fine);
    tr.microstep.hash(&mut self.fine);
    tr.trigger.hash(&mut self.fine);
    tr.extra_delay.hash(&mut self.fine);
  }
  fn finish(self) -> TraceHash {
    TraceHash(
      CoarseTraceHash(self.coarse.finish()),
      FineTraceHash(self.fine.finish()),
    )
  }
}
#[derive(Debug, Serialize, Deserialize)]
enum VectorfyStatus {
  Ok,
  MissingTracePointId,
  ExtraTracePointId,
}

impl State {
  const INITIAL_NAME: &'static str = "initial";
  const COMPILED_NAME: &'static str = "compiled";
  const KNOWN_COUNTS_NAME: &'static str = "known-counts";
  const ACCUMULATING_TRACES_NAME: &'static str = "accumulating-traces";

  pub fn load(src_dir: PathBuf, scratch_dir: PathBuf, delay_params: DelayParams) -> Self {
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
      return rmp_serde::from_read(File::open(ats_files[0].0.path()).expect("could not open file"))
        .expect("failed to deserialize");
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
      Self::AccumulatingTraces(s) => &s.kcs.cs.initial,
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
  pub fn run(self) -> Self {
    match self {
      Self::Initial(is) => Self::Compiled(is.compile()),
      Self::Compiled(cs) => Self::KnownCounts(cs.known_counts()),
      Self::KnownCounts(kcs) => Self::AccumulatingTraces(kcs.advance()),
      Self::AccumulatingTraces(mut ats) => {
        ats.accumulate_traces();
        Self::AccumulatingTraces(ats)
      }
    }
  }
}

impl InitialState {
  fn compile(self) -> CompiledState {
    let executables = self
      .src_files
      .iter()
      .map(|(id, src)| {
        let mut exe = src.clone();
        loop {
          exe = exe
            .parent()
            .expect("could not get parent of exe")
            .to_path_buf();
          if exe.ends_with("src") {
            break;
          }
        }
        exe = exe
          .join("bin")
          .join(src.file_stem().expect("could not get file stem"));
        println!("compiling {src:?}...",);
        let output = std::process::Command::new("lfcpartest")
          .arg(src)
          .output()
          .expect("failed to run lfcpartest");
        if !output.status.success() {
          panic!(
            "failed to compile {}:\n{output:?}\n",
            src.to_str().expect("os string is not UTF-8")
          );
        }
        (*id, exe)
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
  fn get_traces_attempts(executable: &Path, scratch_dir: &Path) -> Traces {
    for _ in 0..Self::ATTEMPTS {
      let ret = get_traces(executable, scratch_dir, crate::EnvironmentUpdate::default());
      if let Ok(ret) = ret {
        return ret;
      }
    }
    panic!("could not get metadata for executable");
  }
  fn known_counts(self) -> KnownCountsState {
    let metadata = self
      .executables
      .iter()
      .map(|(id, exe)| {
        let ic = get_counts(exe);
        let mut traces_map = CompiledState::get_traces_attempts(exe, &self.initial.scratch_dir).0;
        let traces = traces_map
          .get_mut("rti.csv")
          .expect("no trace file named rti.csv")
          .deserialize()
          .map(|r| r.expect("could not read record"))
          .map(|tr| TracePointId::new(&tr));
        let ovkey = OutputVectorKey::new(traces);
        (*id, TestMetadata { ic, ovkey })
      })
      .collect::<HashMap<_, _>>();
    KnownCountsState { cs: self, metadata }
  }
}

impl KnownCountsState {
  fn advance(self) -> AccumulatingTracesState {
    AccumulatingTracesState {
      kcs: self,
      runs: HashMap::new(),
    }
  }
}

impl AccumulatingTracesState {
  fn total_runs(&self) -> usize {
    self.runs.values().map(|tr| tr.raw_traces.len()).sum()
  }
  fn get_delay_vector(&self, id: &TestId) -> DelayVector {
    let params = &self.kcs.cs.initial.delay_params;
    let mut rng = rand::thread_rng();
    DelayVector::random(&self.kcs.metadata[id].ic, &mut rng, params)
  }
  fn get_run(&self, id: &TestId, exe: &Path, dvec: &DelayVector) -> Result<SuccessfulRun, Crash> {
    let mut traces_map = run_with_parameters(
      exe,
      &self.kcs.cs.initial.scratch_dir,
      &self.kcs.metadata[id].ic,
      dvec,
    )?;
    let raw_traces = traces_map
      .0
      .get_mut("rti.csv")
      .expect("no trace file named rti.csv")
      .deserialize()
      .map(|r| r.expect("could not read record"));
    let (ov, th, status) = self.kcs.metadata[id].ovkey.vectorfy(raw_traces);
    Ok((ov, th, status))
  }
  fn accumulate_traces(&mut self) {
    // TODO: validate and fix this
    for (id, exe) in &self.kcs.cs.executables {
      let dvec = self.get_delay_vector(id);
      let run = self.get_run(id, exe, &dvec);
      self
        .runs
        .entry(*id)
        .or_insert(TestRuns {
          raw_traces: vec![],
          iomat_global: Array2::zeros((0, 0)),
          iomats: HashMap::new(),
        })
        .raw_traces
        .push((dvec, run));
    }
  }
}
