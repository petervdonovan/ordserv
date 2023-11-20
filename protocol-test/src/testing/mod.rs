use std::{
  collections::{hash_map::DefaultHasher, HashMap},
  hash::{Hash, Hasher},
  path::PathBuf,
  sync::{Arc, Mutex, RwLock},
  time::Duration,
};

use colored::Colorize;
use rand::seq::SliceRandom;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{ser::SerializeStruct, Deserialize, Serialize};

use crate::{
  exec::{ExecResult, Executable},
  io::run_with_parameters,
  outputvector::{OutputVector, OutputVectorRegistry, OvrDelta, OvrReg, VectorfyStatus},
  state::{InitialState, KnownCountsState, State, TestId},
  DelayVector, DelayVectorIndex, DelayVectorRegistry, ThreadId, TraceRecord, CONCURRENCY_LIMIT,
};
#[derive(Debug)]
pub struct AccumulatingTracesState {
  pub kcs: KnownCountsState,
  pub parent: PathBuf,
  pub runs: HashMap<TestId, Arc<RwLock<TestRuns>>>, // TODO: consider using a rwlock
  pub ovr: OutputVectorRegistry,
  pub dt: std::time::Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AtsDelta {
  pub parent: PathBuf,
  pub ovrdelta_path: PathBuf,
  pub runs: HashMap<TestId, PathBuf>,
  pub dt: Duration,
  pub total_runs: usize,
}

impl Serialize for AccumulatingTracesState {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let runs_dir = self.kcs.scratch_dir().join(format!(
      "runs-{}-{}",
      self.total_runs(),
      self.kcs.src_commit()
    ));
    let runs = self
      .runs
      .iter()
      .map(|(id, runs)| {
        let runs = runs.read().unwrap();
        let path = runs_dir.join(format!("{}.mpk", id));
        std::fs::create_dir_all(&runs_dir).unwrap();
        let mut file = std::fs::File::create(&path).unwrap();
        rmp_serde::encode::write(&mut file, &*runs).unwrap();
        (*id, path)
      })
      .collect();
    let ovrdelta_path = runs_dir.join("ovrdelta.mpk");
    let mut file = std::fs::File::create(&ovrdelta_path).unwrap();
    rmp_serde::encode::write(&mut file, &*self.ovr.lock().unwrap()).unwrap();
    let delta = AtsDelta {
      parent: self.parent.clone(),
      runs,
      ovrdelta_path,
      dt: self.dt,
      total_runs: self.total_runs(),
    };
    delta.serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for AccumulatingTracesState {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let mut ancestors = vec![AtsDelta::deserialize(deserializer)?];
    while ancestors.last().unwrap().total_runs > 0 {
      let parent = ancestors.last().unwrap().parent.clone();
      let parent_delta = rmp_serde::from_read(std::fs::File::open(parent).unwrap()).unwrap();
      ancestors.push(parent_delta);
    }
    let kcs: KnownCountsState =
      rmp_serde::from_read(std::fs::File::open(&ancestors.last().unwrap().parent.clone()).unwrap())
        .unwrap();
    let ovrd: Vec<OvrDelta> = ancestors
      .par_iter()
      .map(|atsd| rmp_serde::from_read(std::fs::File::open(&atsd.ovrdelta_path).unwrap()).unwrap())
      .collect();
    let trdelta: Vec<(TestId, TestRunsDelta)> = ancestors
      .par_iter()
      .flat_map(|atsd| {
        atsd.runs.par_iter().map(|(id, path)| {
          let runs_deserialized: TestRunsDelta =
            rmp_serde::from_read(std::fs::File::open(path).unwrap()).unwrap();
          (*id, runs_deserialized)
        })
      })
      .collect();
    let mut trdelta_by_id = HashMap::new();
    for (id, trd) in trdelta {
      trdelta_by_id.entry(id).or_insert(vec![]).push(trd);
    }
    let runs = trdelta_by_id
      .into_par_iter()
      .map(|(tid, trdeltas)| {
        let mut dvr = vec![];
        let mut raw_traces = vec![];
        let mut iomats = HashMap::new();
        for trdelta in trdeltas {
          for dvrd in trdelta.dvr_delta {
            dvr.push(dvrd);
          }
          for rtd in trdelta.raws_delta {
            add_to_iomats(&mut iomats, &rtd);
            raw_traces.push(rtd);
          }
        }
        let dvr_saved_up_to = DelayVectorIndex(dvr.len() as u32);
        let raws_saved_up_to = raw_traces.len();
        (
          tid,
          Arc::new(RwLock::new(TestRuns {
            dvr,
            raw_traces,
            dvr_saved_up_to,
            raws_saved_up_to,
            iomats,
          })),
        )
      })
      .collect();
    Ok(Self {
      kcs,
      parent: ancestors[0].parent.clone(),
      ovr: Arc::new(Mutex::new(OvrReg::rebuild(ovrd.into_iter().rev()))),
      runs,
      dt: ancestors[0].dt,
    })
  }
}
type RawElement = (DelayVectorIndex, Result<SuccessfulRun, ExecResult>);
type IoMats = HashMap<CoarseTraceHash, HashMap<FineTraceHash, Vec<OutputVector>>>;
#[derive(Debug)]
pub struct TestRuns {
  dvr: DelayVectorRegistry,
  pub raw_traces: Vec<RawElement>,
  dvr_saved_up_to: DelayVectorIndex,
  raws_saved_up_to: usize,
  pub iomats: IoMats,
}
impl Serialize for TestRuns {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let mut ret = serializer.serialize_struct("TestRunsDelta", 2)?;
    ret
      .serialize_field("dvr_delta", &self.dvr[self.dvr_saved_up_to.0 as usize..])
      .unwrap();
    ret
      .serialize_field("raws_delta", &self.raw_traces[self.raws_saved_up_to..])
      .unwrap();
    ret.end()
  }
}
#[derive(Deserialize)]
/// This exists solely for deserialization and must be in sync with TestRuns::serialize.
struct TestRunsDelta {
  dvr_delta: DelayVectorRegistry,
  raws_delta: Vec<RawElement>,
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

fn add_to_iomats(iomats: &mut IoMats, raw: &RawElement) {
  if let Ok((ov, trhash, _status)) = &raw.1 {
    iomats
      .entry(trhash.0)
      .or_insert(HashMap::new())
      .entry(trhash.1)
      .or_insert(vec![])
      .push(*ov);
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
            raws_saved_up_to: 0,
            dvr_saved_up_to: DelayVectorIndex(0),
            iomats: HashMap::new(),
          })),
        )
      })
      .collect();
    let parent = crate::state::file_name(
      kcs.scratch_dir(),
      State::KNOWN_COUNTS_NAME,
      kcs.src_commit(),
    );
    Self {
      kcs,
      parent,
      runs,
      ovr: Arc::new(Mutex::new(Default::default())),
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
    let (ov, th, status) = self
      .kcs
      .metadata(id)
      .ovkey
      .vectorfy(raw_traces, Arc::clone(&self.ovr));
    Ok((ov, th, status))
  }

  pub fn accumulate_traces(&mut self, time_seconds: u32) {
    self.parent = crate::state::file_name_with_total_runs(
      self.kcs.scratch_dir(),
      State::ACCUMULATING_TRACES_NAME,
      self.kcs.src_commit(),
      self.total_runs(),
    );
    let t0 = std::time::Instant::now();
    let initial_total_runs = self.total_runs();
    let executables = self.kcs.executables().iter().collect::<Vec<_>>();
    let executables = &executables;
    println!(
      "Spawning {} threads to gather execution traces.",
      *CONCURRENCY_LIMIT.wait()
    );
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
      "Accumulated {} traces in {} seconds = {:.2} hours ({}/second).",
      self.total_runs() - initial_total_runs,
      dt.as_secs(),
      dt.as_secs_f64() / 3600.0,
      (self.total_runs() - initial_total_runs) as u64 / dt.as_secs()
    )
    .bold()
    .on_green();
    println!("{}", msg);
  }
}
