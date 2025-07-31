use std::{
  collections::{hash_map::DefaultHasher, HashMap},
  hash::{Hash, Hasher},
  path::PathBuf,
  sync::{Arc, RwLock},
  time::Duration,
};

use colored::Colorize;
use log::{error, info};
use priority_queue::DoublePriorityQueue;
use rand::{seq::IteratorRandom, SeedableRng};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use streaming_transpositions::{
  BigSmallIterator, CumSum, HookOgRank2CurRank, OgRank, OgRank2CurRank, OutOgRank2CurRank,
  StreamingTranspositions, StreamingTranspositionsDelta,
};

// const RANDOM_ORDERING_GEOMETRIC_R: f64 = 0.5;
const MAX_NUM_FEDERATES_PER_TEST: usize = 48;
const HEALTH_CHECK_FREQUENCY: u32 = 200;
const MAX_N_RUNS_BEFORE_STOPPING: usize = 5000;

use crate::{
  exec::{ExecResult, Executable},
  io::{run_with_parameters, RunContext},
  outputvector::{OutputVector, OutputVectorRegistry, OvrDelta, OvrReg, VectorfyStatus},
  state::{InitialState, KnownCountsState, State, TestId},
  ConstraintList, ConstraintListIndex, ConstraintListRegistry, ThreadId, TraceRecord,
  CONCURRENCY_LIMIT, TEST_TIMEOUT_SECS,
};
#[derive(Debug)]
pub struct AccumulatingTracesState {
  pub kcs: KnownCountsState,
  pub parent: PathBuf,
  pub runs: HashMap<TestId, Arc<RwLock<TestRuns>>>, // TODO: consider using a rwlock
  pub ovr: OutputVectorRegistry,
  pub dt: std::time::Duration,
  pub seqnum: usize,
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
    std::fs::create_dir_all(&runs_dir).unwrap();
    let runs = self
      .runs
      .par_iter()
      .map(|(id, runs)| {
        let runs = runs.read().unwrap();
        let path = runs_dir.join(format!("{}.mpk", id));
        let mut file = std::fs::File::create(&path).unwrap();
        rmp_serde::encode::write(&mut file, &*runs).unwrap();
        (*id, path)
      })
      .collect();
    let ovrdelta_path = runs_dir.join("ovrdelta.mpk");
    let mut file = std::fs::File::create(&ovrdelta_path).unwrap();
    rmp_serde::encode::write(&mut file, &*self.ovr.read().unwrap()).unwrap();
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
    let ancestors = ancestors_chronological(deserializer)?;
    let kcs: KnownCountsState =
      rmp_serde::from_read(std::fs::File::open(ancestors[0].parent.clone()).unwrap()).unwrap();
    let ovrd: Vec<OvrDelta> = ancestors
      .par_iter()
      .map(|atsd| rmp_serde::from_read(std::fs::File::open(&atsd.ovrdelta_path).unwrap()).unwrap())
      .collect();
    let parent = ancestors.last().unwrap().parent.clone();
    let dt = ancestors.last().unwrap().dt;
    let seqnum = ancestors.len();
    let trdelta_by_id = trdelta_by_id(ancestors);
    let ovr = Arc::new(RwLock::new(OvrReg::rebuild(ovrd.into_iter())));
    let runs = trdelta_by_id
      .into_par_iter()
      .map(|(tid, trdeltas)| {
        (
          tid,
          Arc::new(RwLock::new(reconstruct_test_runs(
            &kcs, &ovr, &tid, trdeltas,
          ))),
        )
      })
      .collect();
    Ok(Self {
      kcs,
      parent,
      ovr,
      runs,
      dt,
      seqnum,
    })
  }
}

fn ancestors_chronological<'de, D>(deserializer: D) -> Result<Vec<AtsDelta>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  info!("Loading ancestors.");
  let mut ancestors = vec![AtsDelta::deserialize(deserializer)?];
  while ancestors.last().unwrap().total_runs > 0 {
    info!(
      "Loading ancestor delta with {} runs.",
      ancestors.last().unwrap().total_runs
    );
    let parent = ancestors.last().unwrap().parent.clone();
    let parent_delta = rmp_serde::from_read(std::fs::File::open(parent).unwrap()).unwrap();
    ancestors.push(parent_delta);
  }
  Ok(ancestors.into_iter().rev().collect())
}

fn trdelta_by_id(ancestors_chronological: Vec<AtsDelta>) -> HashMap<TestId, Vec<TestRunsDelta>> {
  let trdelta: Vec<(TestId, TestRunsDelta)> = ancestors_chronological
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
  trdelta_by_id
}

fn reconstruct_test_runs(
  kcs: &KnownCountsState,
  ovr: &OutputVectorRegistry,
  tid: &TestId,
  trdeltas: Vec<TestRunsDelta>,
) -> TestRuns {
  let mut clr = vec![];
  let mut raw_traces = vec![];
  let mut iomats = HashMap::new();
  let mut strans_out = kcs.empty_streaming_transpositions_out(tid);
  let strans_hook = StreamingTranspositions::from_deltas(trdeltas.iter().map(|it| &it.strans_hook));
  let interesting = trdeltas
    .last()
    .unwrap()
    .interesting
    .iter()
    .map(|(idx, int)| (*idx, *int))
    .collect();
  let pair_iterator = trdeltas.last().unwrap().pair_iterator.clone();
  let done = trdeltas.last().unwrap().done;
  let initial_cumsum_in_current_pass = trdeltas.last().unwrap().initial_cumsum_in_current_pass;
  for trdelta in trdeltas {
    for dvrd in trdelta.clr_delta {
      clr.push(dvrd);
    }
    for rtd in trdelta.raws_delta {
      add_to_iomats(&mut iomats, &rtd);
      add_to_strans(&mut strans_out, &rtd.1, ovr);
      raw_traces.push(rtd);
    }
  }
  let dvr_saved_up_to = ConstraintListIndex(clr.len() as u32);
  let raws_saved_up_to = raw_traces.len();
  TestRuns {
    clr,
    raw_traces,
    clr_saved_up_to: dvr_saved_up_to,
    raws_saved_up_to,
    iomats,
    strans_out,
    strans_hook,
    interesting,
    pair_iterator,
    done,
    initial_cumsum_in_current_pass,
  }
}

type RawElement = (ConstraintListIndex, Result<SuccessfulRun, ExecResult>);
type IoMats = HashMap<CoarseTraceHash, HashMap<FineTraceHash, Vec<OutputVector>>>;
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct Interestingness(pub u32);
#[derive(Debug)]
pub struct TestRuns {
  pub(crate) clr: ConstraintListRegistry,
  pub raw_traces: Vec<RawElement>,
  clr_saved_up_to: ConstraintListIndex,
  raws_saved_up_to: usize,
  pub iomats: IoMats, // derived from raws
  pub strans_out: StreamingTranspositions,
  pub strans_hook: StreamingTranspositions,
  pub interesting: DoublePriorityQueue<ConstraintListIndex, Interestingness>,
  pub pair_iterator: BigSmallIterator,
  pub done: bool,
  pub initial_cumsum_in_current_pass: CumSum,
}
impl Serialize for TestRuns {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let mut ret = serializer.serialize_struct("TestRunsDelta", 7)?;
    ret
      .serialize_field("clr_delta", &self.clr[self.clr_saved_up_to.0 as usize..])
      .unwrap();
    ret
      .serialize_field("raws_delta", &self.raw_traces[self.raws_saved_up_to..])
      .unwrap();
    ret
      .serialize_field("interesting", &self.interesting.iter().collect::<Vec<_>>())
      .unwrap();
    ret
      .serialize_field("strans_hook", self.strans_hook.as_delta())
      .unwrap();
    ret
      .serialize_field("pair_iterator", &self.pair_iterator)
      .unwrap();
    ret.serialize_field("done", &self.done).unwrap();
    ret
      .serialize_field(
        "initial_cumsum_in_current_pass",
        &self.initial_cumsum_in_current_pass,
      )
      .unwrap();
    ret.end()
  }
}
impl TestRuns {
  pub fn update_saved_up_to_for_saving_deltas(&mut self) {
    self.clr_saved_up_to = ConstraintListIndex(self.clr.len() as u32);
    self.raws_saved_up_to = self.raw_traces.len();
    self.strans_hook.update_ancestors();
    self.strans_out.update_ancestors();
  }
}
#[derive(Deserialize)]
/// This exists solely for deserialization and must be in sync with TestRuns::serialize.
struct TestRunsDelta {
  clr_delta: ConstraintListRegistry,
  raws_delta: Vec<RawElement>,
  interesting: Vec<(ConstraintListIndex, Interestingness)>,
  strans_hook: StreamingTranspositionsDelta, // FIXME: This is not incremental! It is aggregated, not a delta.
  pair_iterator: BigSmallIterator,
  done: bool,
  initial_cumsum_in_current_pass: CumSum,
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
    if tr.elapsed_logical_time >= 0 {
      tr.elapsed_logical_time.hash(&mut self.coarse);
    }
    tr.microstep.hash(&mut self.coarse);
    tr.event.hash(&mut self.fine);
    tr.reactor.hash(&mut self.fine);
    tr.source.hash(&mut self.fine);
    tr.destination.hash(&mut self.fine);
    if tr.elapsed_logical_time >= 0 {
      tr.elapsed_logical_time.hash(&mut self.fine);
    }
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

fn add_to_strans(
  strans: &mut StreamingTranspositions,
  raw: &Result<SuccessfulRun, ExecResult>,
  ovr: &OutputVectorRegistry,
) {
  if let Ok((ov, _trhash, _status)) = &raw {
    strans.record(OgRank2CurRank(ov.unpack(ovr)), ov.sentinel());
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
            clr: vec![],
            raw_traces: vec![],
            raws_saved_up_to: 0,
            clr_saved_up_to: ConstraintListIndex(0),
            iomats: HashMap::new(),
            strans_out: kcs.empty_streaming_transpositions_out(id),
            strans_hook: kcs.empty_streaming_transpositions_hook(id),
            interesting: DoublePriorityQueue::new(),
            pair_iterator: BigSmallIterator::new(OgRank(kcs.metadata(id).hic.len() as u32)),
            done: false,
            initial_cumsum_in_current_pass: CumSum(0),
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
      ovr: Arc::new(RwLock::new(Default::default())),
      dt: std::time::Duration::from_secs(0),
      seqnum: 0,
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
  fn get_constraint_vector(&self, id: &TestId) -> ConstraintList {
    let mut guard = self.runs[id].write().unwrap();
    let filter = |before: OgRank, after: OgRank| {
      let before_hinvoc = &self.kcs.metadata(id).hic.ogrank2hinvoc[before.idx()];
      let after_hinvoc = &self.kcs.metadata(id).hic.ogrank2hinvoc[after.idx()];
      before_hinvoc.hid.1 != after_hinvoc.hid.1
    };
    let (before, after);
    loop {
      let power = guard.pair_iterator.power();
      if let Some((i_after, i_before)) = guard.pair_iterator.next() {
        if guard.strans_hook.contains(i_before, i_after) || !filter(i_before, i_after) {
          continue;
        }
        if guard.pair_iterator.power() != power {
          info!(
            "Max difference: {}. Current difference: 2^{}.",
            guard.pair_iterator.max_ogrank_strict().0,
            guard.pair_iterator.power()
          );
        }
        (before, after) = (i_before, i_after);
        if guard.raw_traces.len() > MAX_N_RUNS_BEFORE_STOPPING {
          guard.done = true;
        }
        break;
      } else {
        guard.pair_iterator =
          BigSmallIterator::new(OgRank(guard.pair_iterator.max_ogrank_strict().0));
        guard.initial_cumsum_in_current_pass = guard.strans_out.cumsum();
        continue;
      }
    }
    assert!(before > after);
    ConstraintList::singleton(after, before, self.kcs.metadata(id).hic.len() as u32)
  }
  async fn get_run(
    &self,
    id: &TestId,
    exe: &Executable,
    conl: &ConstraintList,
    clr: Arc<RwLock<TestRuns>>,
    rctx: &mut RunContext<'_>,
  ) -> Result<
    (
      HookOgRank2CurRank,
      OutOgRank2CurRank,
      TraceHash,
      VectorfyStatus,
    ),
    ExecResult,
  > {
    let (raw_traces, raw_traces_rti_only);
    loop {
      let (_, traces_map) = run_with_parameters(
        exe,
        &self.kcs.metadata(id).hic,
        conl,
        Arc::clone(&clr),
        rctx,
      )
      .await;
      let mut traces_map = traces_map?;
      if let Ok(result) = traces_map.hooks_and_outs() {
        (raw_traces, raw_traces_rti_only) = result;
        break;
      }
    }
    let (hook_orcr, _th, _status) = self
      .kcs
      .metadata(id)
      .hook_ovkey
      .vectorfy(raw_traces.into_iter());
    let (out_orcr, th, status) = self
      .kcs
      .metadata(id)
      .out_ovkey
      .vectorfy(raw_traces_rti_only.into_iter());
    Ok((
      HookOgRank2CurRank(hook_orcr, self.kcs.metadata(id).hook_ovkey.sentinel()),
      OutOgRank2CurRank(out_orcr, self.kcs.metadata(id).out_ovkey.sentinel()),
      th,
      status,
    ))
  }

  pub fn update_saved_up_to_for_saving_deltas(&mut self) {
    for runs in self.runs.values() {
      runs.write().unwrap().update_saved_up_to_for_saving_deltas();
    }
    self
      .ovr
      .write()
      .unwrap()
      .update_saved_up_to_for_saving_deltas();
    self.seqnum += 1;
  }

  pub fn accumulate_traces(&mut self, time_seconds: u32) -> u32 {
    self.parent = crate::state::file_name_with_total_runs(
      self.kcs.scratch_dir(),
      State::ACCUMULATING_TRACES_NAME,
      self.kcs.src_commit(),
      self.total_runs(),
      self.seqnum - 1,
    );
    let t0 = std::time::Instant::now();
    let initial_total_runs = self.total_runs();
    let executables = self
      .kcs
      .executables()
      .iter()
      .map(|(a, b)| (*a, b.clone()))
      .collect::<Vec<_>>();
    let executables_immut = SendableExecutables(&executables as *const Vec<(TestId, Executable)>);
    info!(
      "Spawning {} threads to gather execution traces.",
      *CONCURRENCY_LIMIT.wait()
    );
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    let scratch = self.kcs.scratch_dir().to_owned();
    rt.block_on(async {
      // async_scoped::TokioScope::scope_and_block(|scope|
      {
        let self_immut = SendableAts(self as *const AccumulatingTracesState);
        let mut jhs = Vec::with_capacity(*CONCURRENCY_LIMIT.wait());
        for tidx in 0..*CONCURRENCY_LIMIT.wait() {
          info!("Spawning thread {}.", tidx);
          let my_ovr = Arc::clone(&self.ovr);
          let scratch = scratch.clone();
          jhs.push(tokio::task::spawn(async move {
            loop {
              let my_ovr = Arc::clone(&my_ovr);
              // The following is unsafe because we are dereferencing raw pointers. It is OK because
              // the thread that uses the pointers is being joined, but tokio doesn't know that and
              // wants them to have static lifetimes.
              unsafe {
                let self_immut = self_immut.get();
                let executables_immut = executables_immut.get();
                let spawned = tokio::task::spawn(Self::main(
                  scratch.clone(),
                  tidx,
                  time_seconds,
                  t0,
                  self_immut,
                  executables_immut,
                  my_ovr,
                ));
                if spawned.await.is_err() {
                  error!("Thread {} panicked.", tidx);
                  crate::kill_everything().await;  // shotgun-style error handling. Ok cuz children are already assumed unreliable
                  continue;  // Catch panics in spawned thread and keep looping.
                }
                break;
              }
            }
          }));
        }
        tokio::time::sleep(std::time::Duration::from_secs(time_seconds as u64)).await;
        println!("Waiting for threads to join...");
        for (tid, jh) in jhs.iter_mut().enumerate() {
          println!("Thread {} aborting...", tid);
          jh.abort();  // FIXME: This hack is an alternative to shutting them down "the right way"
          println!("Thread {} aborted.", tid);
        }
        crate::kill_everything().await;
      } //)
      ;
    });
    rt.shutdown_timeout(std::time::Duration::from_secs(
      time_seconds as u64 + TEST_TIMEOUT_SECS * 2,
    ));
    let dt = std::time::Instant::now() - t0;
    self.dt += dt;
    let msg = format!(
      "Accumulated {} traces in {} seconds = {:.2} hours ({}/second).",
      self.total_runs() - initial_total_runs,
      dt.as_secs(),
      dt.as_secs_f64() / 3600.0,
      (self.total_runs() - initial_total_runs) as f64 / dt.as_secs_f64()
    )
    .bold()
    .on_green();
    println!("{}", msg);
    crate::io::clean(self.kcs.scratch_dir());
    self.print_tests_not_done()
  }
  fn get_executable(
    tidx: usize,
    testruns: &HashMap<TestId, Arc<RwLock<TestRuns>>>,
    executables: &Vec<(TestId, Executable)>,
  ) -> Option<(TestId, Executable)> {
    let mut rng = rand::thread_rng();
    let start_at = (0..executables.len()).choose(&mut rng).unwrap();
    let mut current = start_at;
    loop {
      let (id, exe) = &executables[current];
      let testruns_read = testruns.get(id).unwrap().read().unwrap();
      if !testruns_read.done {
        return Some((*id, exe.clone()));
      }
      current = (current + 1) % executables.len();
      if current == start_at {
        return None;
      }
    }
  }
  fn print_tests_not_done(&self) -> u32 {
    let mut not_done = vec![];
    for (id, runs) in &self.runs {
      let runs = runs.read().unwrap();
      if !runs.done {
        not_done.push(id);
      }
    }
    if !not_done.is_empty() {
      println!("Tests not done:");
      for id in not_done.iter() {
        println!("  {}", self.kcs.executables().get(id).unwrap());
      }
    }
    not_done.len() as u32
  }
  #[allow(clippy::too_many_arguments)] // FIXME: refactor. sry clippy don't have time for u
  async fn main(
    scratch: PathBuf,
    tidx: usize,
    time_seconds: u32,
    t0: std::time::Instant,
    self_immut: &AccumulatingTracesState,
    executables: &Vec<(TestId, Executable)>,
    my_ovr: OutputVectorRegistry,
  ) {
    let mut ordserv_handle =
      ordering_server::server::run_reusing_connections(1, MAX_NUM_FEDERATES_PER_TEST).await;
    let ordserv = &mut ordserv_handle.updates_acks[0];
    let mut rctx = RunContext {
      scratch: &scratch,
      tid: ThreadId(tidx),
      ordserv,
      run_id: 0,
    };
    let mut successes = 0;
    while std::time::Instant::now() - t0 < std::time::Duration::from_secs(time_seconds as u64) {
      if let Some((id, exe)) = Self::get_executable(tidx, &self_immut.runs, executables) {
        let conl = self_immut.get_constraint_vector(&id);
        let clr = Arc::clone(&self_immut.runs[&id]);
        let run = self_immut.get_run(&id, &exe, &conl, clr, &mut rctx).await;
        rctx.run_id += 1;
        if run.is_ok() {
          successes += 1;
        }
        if rctx.run_id % HEALTH_CHECK_FREQUENCY == 0 || run.is_err() {
          info!(
            "Thread {} health check. Success rate: {} / {} ({}). Speed: {} runs/second.",
            tidx,
            successes,
            rctx.run_id,
            (successes as f64) / (rctx.run_id as f64),
            rctx.run_id as f64 / (std::time::Instant::now() - t0).as_secs_f64()
          );
        }
        let mut entry = self_immut.runs.get(&id).unwrap().write().unwrap();
        entry.clr.push(conl);
        let idx = ConstraintListIndex(entry.clr.len() as u32 - 1);
        match run {
          Ok((hook_orcr, out_orcr, trhash, status)) => {
            entry.strans_hook.record(hook_orcr.0.clone(), hook_orcr.1);
            entry.strans_out.record(out_orcr.0.clone(), out_orcr.1);
            let ov = OutputVector::new(out_orcr.0, Arc::clone(&my_ovr));
            entry
              .iomats
              .entry(trhash.0)
              .or_insert(HashMap::new())
              .entry(trhash.1)
              .or_insert(vec![])
              .push(ov);
            entry.raw_traces.push((idx, Ok((ov, trhash, status))));
          }
          Err(err) => {
            entry.raw_traces.push((idx, Err(err)));
          }
        }
      } else {
        info!("Done.");
        tokio::time::sleep(std::time::Duration::from_secs(time_seconds as u64)).await;
      }
    }
    ordserv_handle.updates_acks[0].0.send(None).await.unwrap();
    ordserv_handle.join_handle.await.unwrap();
  }
}
#[derive(Debug, Clone, Copy)]
struct SendableAts(*const AccumulatingTracesState);
impl SendableAts {
  unsafe fn get<'a>(&self) -> &'a AccumulatingTracesState {
    unsafe { &*self.0 }
  }
}
unsafe impl Send for SendableAts {}
#[derive(Debug, Clone, Copy)]
struct SendableExecutables(*const Vec<(TestId, Executable)>);
unsafe impl Send for SendableExecutables {}
impl SendableExecutables {
  unsafe fn get<'a>(&self) -> &'a Vec<(TestId, Executable)> {
    unsafe { &*self.0 }
  }
}
