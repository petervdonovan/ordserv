use std::{
  collections::{hash_map::DefaultHasher, HashMap},
  hash::{Hash, Hasher},
  path::PathBuf,
  sync::{Arc, RwLock},
  time::Duration,
};

use colored::Colorize;
use ndarray::{Array1, Array2};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::{
  exec::{ExecResult, Executable},
  io::run_with_parameters,
  state::{InitialState, KnownCountsState, OutputVector, OutputVectorKey, TestId, TracePointId},
  DelayVector, ThreadId, TraceRecord, CONCURRENCY_LIMIT,
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
  raw_traces: Vec<(DelayVector, Result<SuccessfulRun, ExecResult>)>,
  #[serde(skip)] // derived from raw_traces (successes only)
  in_mat_global: Array2<i64>,
  #[serde(skip)]
  out_vectors_global: Vec<Array1<i64>>,
  pub iomats: HashMap<CoarseTraceHash, HashMap<FineTraceHash, Vec<usize>>>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
pub struct CoarseTraceHash(pub u64);
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
pub struct FineTraceHash(pub u64);
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
          Arc::new(RwLock::new(TestRuns {
            raw_traces: vec![],
            in_mat_global: Array2::zeros((0, kcs.metadata(id).hic.len())),
            out_vectors_global: vec![],
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
  fn empty_in_mat_global(&self, tid: &TestId) -> Array2<i64> {
    Array2::zeros((0, self.kcs.metadata(tid).hic.len()))
  }
  /// After deserialization, the raw_traces field is the only field that is up to date.
  pub fn make_consistent(&mut self, path: PathBuf) {
    self.parent = Some(path);
    for tid in self.kcs.tids() {
      let mut entry = self.runs.get(tid).unwrap().write().unwrap();
      let entry = &mut *entry;
      entry.in_mat_global = self.empty_in_mat_global(tid);
      for (dvec, run) in entry.raw_traces.iter() {
        self.add_run_from_raw(
          dvec,
          run,
          &mut entry.in_mat_global,
          &mut entry.out_vectors_global,
        );
      }
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
  ) -> Result<SuccessfulRun, ExecResult> {
    let (_, traces_map) = run_with_parameters(
      exe,
      self.kcs.scratch_dir(),
      &self.kcs.metadata(id).hic,
      dvec,
      tidx,
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
  fn add_run_from_raw(
    &self,
    dvec: &DelayVector,
    run: &Result<SuccessfulRun, ExecResult>,
    in_mat_global: &mut Array2<i64>,
    out_vectors_global: &mut Vec<Array1<i64>>,
  ) {
    if let Ok(run) = &run {
      let dvec_int = dvec.0.iter().map(|it| *it as i64).collect::<Array1<_>>();
      let ov = Array1::from_vec(run.0 .0.clone());
      match in_mat_global.push_row(dvec_int.view()) {
        Ok(ov) => ov,
        Err(e) => {
          println!(
            "shape error: {} with shape {:?} and length {}",
            e,
            in_mat_global.shape(),
            dvec_int.len()
          );
          panic!("shape error")
        }
      };
      out_vectors_global.push(ov);
    }
  }
  fn add_run(&self, tid: TestId, dvec: DelayVector, run: Result<SuccessfulRun, ExecResult>) {
    let entry = self.runs.get(&tid).unwrap().clone();
    let mut entry = entry.write().unwrap();
    let entry = &mut *entry;
    self.add_run_from_raw(
      &dvec,
      &run,
      &mut entry.in_mat_global,
      &mut entry.out_vectors_global,
    );
    if let Ok(run) = &run {
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
    let initial_total_runs = self.total_runs();
    let executables = self.kcs.executables().iter().collect::<Vec<_>>();
    let executables = &executables;
    std::thread::scope(|scope| {
      let self_immut = &self;
      for tidx in 0..CONCURRENCY_LIMIT {
        scope.spawn(move || {
          while std::time::Instant::now() - t0 < std::time::Duration::from_secs(time_seconds as u64)
          {
            let mut rng = rand::thread_rng();
            let (id, exe) = executables.choose(&mut rng).unwrap();
            let dvec = self_immut.get_delay_vector(id);
            let run = self_immut.get_run(id, exe, &dvec, ThreadId(tidx));
            self_immut.add_run(**id, dvec, run);
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
