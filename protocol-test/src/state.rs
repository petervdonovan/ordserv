use std::{
  collections::{hash_map::DefaultHasher, HashMap},
  fs::File,
  hash::{Hash, Hasher},
  io::Write,
  os::unix::prelude::OsStrExt,
  path::{Path, PathBuf},
};

use ndarray::Array2;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
  io::{get_commit_hash, get_lf_files_non_recursive},
  DelayParams, DelayVector, InvocationCounts, TraceRecord,
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
  ic: InvocationCounts,
  ovkey: OutputVectorKey,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AccumulatingTracesState {
  kcs: KnownCountsState,
  runs: HashMap<TestId, TestRuns>,
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
struct Crash {
  exit_status: i32,
  stdout: String,
  stderr: String,
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
  fn new(src_dir: PathBuf, scratch_dir: PathBuf, delay_params: DelayParams) -> Self {
    let src_commit = get_commit_hash(&src_dir);
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
      Self::Initial(_) => "initial".to_string(),
      Self::Compiled(_) => "compiled".to_string(),
      Self::KnownCounts(_) => "known-counts".to_string(),
      Self::AccumulatingTraces(ref ats) => format!("accumulating-traces-{}", ats.total_runs()),
    };
    format!("{}-{}.mpk", phase, self.get_initial_state().src_commit)
  }
  fn save_to_scratch_dir(&self) {
    File::create(self.get_initial_state().scratch_dir.join(self.file_name()))
      .expect("could not create file")
      .write_all(&rmp_serde::to_vec(self).expect("could not serialize state"))
      .expect("could not write to file");
  }
  fn run(&mut self) {
    todo!()
  }
}

impl AccumulatingTracesState {
  fn total_runs(&self) -> usize {
    self.runs.values().map(|tr| tr.raw_traces.len()).sum()
  }
}
