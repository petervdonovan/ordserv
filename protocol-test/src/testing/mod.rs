use std::{
  collections::{hash_map::DefaultHasher, HashMap},
  hash::{Hash, Hasher},
  path::PathBuf,
  sync::{Arc, RwLock},
  time::Duration,
};

use colored::Colorize;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::{
  exec::{ExecResult, Executable},
  io::run_with_parameters,
  outputvector::{OutputVector, VectorfyStatus},
  state::{InitialState, KnownCountsState, TestId},
  DelayVector, DelayVectorIndex, DelayVectorRegistry, ThreadId, TraceRecord, CONCURRENCY_LIMIT,
};
#[derive(Debug, Serialize, Deserialize)]
pub struct AccumulatingTracesState {
  pub kcs: KnownCountsState,
  pub parent: Option<PathBuf>,
  pub runs: HashMap<TestId, Arc<RwLock<TestRuns>>>, // TODO: consider using a rwlock
  pub dt: std::time::Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestRuns {
  dvr: DelayVectorRegistry,
  raw_traces: Vec<(DelayVectorIndex, Result<SuccessfulRun, ExecResult>)>,
  // #[serde(skip)] // derived from raw_traces (successes only)
  // in_mat_global: Array2<i64>,
  // #[serde(skip)]
  // out_vectors_global: Vec<Array1<i64>>,
  pub iomats: HashMap<CoarseTraceHash, HashMap<FineTraceHash, Vec<usize>>>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
pub struct CoarseTraceHash(pub u64);
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
pub struct FineTraceHash(pub u64);
#[derive(Debug, Serialize, Deserialize)]
pub struct TraceHash(CoarseTraceHash, FineTraceHash);

pub type SuccessfulRun = (OutputVector, TraceHash, VectorfyStatus);

pub struct TraceHasher {
  coarse: DefaultHasher,
  fine: DefaultHasher,
}

impl Default for TraceHasher {
  fn default() -> Self {
    Self {
      coarse: DefaultHasher::new(),
      fine: DefaultHasher::new(),
    }
  }
}

impl TraceHasher {
  pub fn update(&mut self, tr: &TraceRecord) {
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
  pub fn finish(self) -> TraceHash {
    TraceHash(
      CoarseTraceHash(self.coarse.finish()),
      FineTraceHash(self.fine.finish()),
    )
  }
}

impl AccumulatingTracesState {
  pub fn new(kcs: KnownCountsState) -> Self {
    let runs = kcs
      .tids()
      .map(|id| {
        (
          *id,
          Arc::new(RwLock::new(TestRuns {
            dvr: vec![],
            raw_traces: vec![],
            // in_mat_global: Array2::zeros((0, kcs.metadata(id).hic.len())),
            // out_vectors_global: vec![],
            iomats: HashMap::new(),
          })),
        )
      })
      .collect();
    Self {
      kcs,
      parent: None,
      runs,
      dt: std::time::Duration::from_secs(0),
    }
  }
  pub fn total_runs(&self) -> usize {
    self
      .runs
      .values()
      .map(|tr| tr.read().unwrap().raw_traces.len())
      .sum()
  }
  pub fn get_initial_state(&self) -> &InitialState {
    self.kcs.get_initial_state()
  }
  pub fn get_dt(&self) -> Duration {
    self.dt
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
    tidx: ThreadId,
    dvr: &DelayVectorRegistry,
  ) -> Result<SuccessfulRun, ExecResult> {
    let (_, traces_map) = run_with_parameters(
      exe,
      self.kcs.scratch_dir(),
      &self.kcs.metadata(id).hic,
      dvec,
      tidx,
      dvr,
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

  pub fn accumulate_traces(&mut self, time_seconds: u32) {
    let t0 = std::time::Instant::now();
    let initial_total_runs = self.total_runs();
    let executables = self.kcs.executables().iter().collect::<Vec<_>>();
    let executables = &executables;
    std::thread::scope(|scope| {
      let self_immut = &self;
      for tidx in 0..(*CONCURRENCY_LIMIT.wait()) {
        scope.spawn(move || {
          while std::time::Instant::now() - t0 < std::time::Duration::from_secs(time_seconds as u64)
          {
            let mut rng = rand::thread_rng();
            let (id, exe) = executables.choose(&mut rng).unwrap();
            let dvec = self_immut.get_delay_vector(id);
            let run = self_immut.get_run(
              id,
              exe,
              &dvec,
              ThreadId(tidx),
              &self_immut.runs[id].read().unwrap().dvr,
            );
            // self_immut.add_run(**id, dvec, run);
            let mut entry = self_immut.runs.get(id).unwrap().write().unwrap();
            entry.dvr.push(dvec);
            let idx = DelayVectorIndex(entry.dvr.len() as u32 - 1);
            entry.raw_traces.push((idx, run));
          }
        });
      }
    });
    let dt = std::time::Instant::now() - t0;
    self.dt += dt;
    let msg = format!(
      "Accumulated {} traces in {} seconds = {:.2} hours.",
      self.total_runs() - initial_total_runs,
      dt.as_secs(),
      dt.as_secs_f64() / 3600.0
    )
    .bold()
    .on_green();
    println!("{}", msg);
  }
}
