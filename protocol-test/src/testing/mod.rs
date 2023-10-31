use std::{
  collections::{hash_map::DefaultHasher, HashMap},
  hash::{Hash, Hasher},
  sync::{Arc, Mutex},
};

use ndarray::{Array1, Array2};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::{
  exec::{ExecResult, Executable},
  io::run_with_parameters,
  state::{
    InitialState, KnownCountsState, OutputVector, OutputVectorKey, TestId, TracePointId,
    CONCURRENCY_LIMIT,
  },
  DelayVector, TraceRecord,
};
#[derive(Debug, Serialize, Deserialize)]
pub struct AccumulatingTracesState {
  kcs: KnownCountsState,
  runs: HashMap<TestId, Arc<Mutex<TestRuns>>>, // TODO: consider using a rwlock
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestRuns {
  raw_traces: Vec<(DelayVector, Result<SuccessfulRun, ExecResult>)>,
  #[serde(skip)] // derived from raw_traces (successes only)
  in_mat_global: Array2<i64>,
  #[serde(skip)]
  out_vectors_global: Vec<Array1<i64>>,
  iomats: HashMap<CoarseTraceHash, HashMap<FineTraceHash, Vec<usize>>>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
struct CoarseTraceHash(u64);
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
struct FineTraceHash(u64);
#[derive(Debug, Serialize, Deserialize)]
struct TraceHash(CoarseTraceHash, FineTraceHash);

type SuccessfulRun = (OutputVector, TraceHash, VectorfyStatus);

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

impl OutputVectorKey {
  fn vectorfy(&self, records: impl Iterator<Item = TraceRecord>) -> SuccessfulRun {
    let mut ov = vec![0; self.n_tracepoints];
    let mut th = TraceHasher::new();
    let mut status = VectorfyStatus::Ok;
    let mut subidxs = HashMap::new();
    for tr in records {
      let tpi = TracePointId::new(&tr);
      if let Some(idxs) = self.map.get(&tpi) {
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
#[derive(Debug, Serialize, Deserialize)]
enum VectorfyStatus {
  Ok,
  MissingTracePointId,
  ExtraTracePointId,
}

impl AccumulatingTracesState {
  pub fn new(kcs: KnownCountsState) -> Self {
    let runs = kcs
      .tids()
      .map(|id| {
        (
          *id,
          Arc::new(Mutex::new(TestRuns {
            raw_traces: vec![],
            in_mat_global: Array2::zeros((0, kcs.metadata(id).hic.len())),
            out_vectors_global: vec![],
            iomats: HashMap::new(),
          })),
        )
      })
      .collect();
    Self { kcs, runs }
  }
  pub fn total_runs(&self) -> usize {
    self
      .runs
      .values()
      .map(|tr| tr.lock().unwrap().raw_traces.len())
      .sum()
  }
  pub fn get_initial_state(&self) -> &InitialState {
    self.kcs.get_initial_state()
  }
  fn get_delay_vector(&self, id: &TestId) -> DelayVector {
    let params = &self.kcs.delay_params();
    let mut rng = rand::thread_rng();
    DelayVector::random(&self.kcs.metadata(id).hic, &mut rng, params)
  }
  fn get_run(
    &self,
    id: &TestId,
    exe: &Executable,
    dvec: &DelayVector,
  ) -> Result<SuccessfulRun, ExecResult> {
    let (_, traces_map) = run_with_parameters(
      exe,
      self.kcs.scratch_dir(),
      &self.kcs.metadata(id).hic,
      dvec,
    );
    let mut traces_map = traces_map?;
    let raw_traces = traces_map
      .0
      .get_mut("rti.csv")
      .expect("no trace file named rti.csv")
      .deserialize()
      .map(|r| r.expect("could not read record"));
    let (ov, th, status) = self.kcs.metadata(id).ovkey.vectorfy(raw_traces);
    Ok((ov, th, status))
  }
  fn add_run(&self, id: TestId, dvec: DelayVector, run: Result<SuccessfulRun, ExecResult>) {
    let entry = self.runs.get(&id).unwrap().clone();
    let mut entry = entry.lock().unwrap();
    if let Ok(run) = &run {
      let dvec_int = dvec.0.iter().map(|it| *it as i64).collect::<Array1<_>>();
      let ov = Array1::from_vec(run.0 .0.clone());
      entry
        .in_mat_global
        .push_row(dvec_int.view())
        .expect("shape error: should be impossible");
      entry.out_vectors_global.push(ov);
      let idx = entry.in_mat_global.nrows() - 1;
      let vec = entry
        .iomats
        .entry(run.1 .0)
        .or_insert_with(HashMap::new)
        .entry(run.1 .1)
        .or_insert_with(Vec::new);
      vec.push(idx);
    }
    entry.raw_traces.push((dvec, run));
  }
  pub fn accumulate_traces(&mut self, time_seconds: u32) {
    let t0 = std::time::Instant::now();
    let executables = self.kcs.executables().iter().collect::<Vec<_>>();
    std::thread::scope(|scope| {
      for _ in 0..CONCURRENCY_LIMIT {
        scope.spawn(|| {
          while std::time::Instant::now() - t0 < std::time::Duration::from_secs(time_seconds as u64)
          {
            let mut rng = rand::thread_rng();
            let (id, exe) = executables.choose(&mut rng).unwrap();
            let dvec = self.get_delay_vector(id);
            let run = self.get_run(id, exe, &dvec);
            self.add_run(**id, dvec, run);
          }
        });
      }
    });
  }
}
