#[allow(dead_code)]
mod io;
pub mod outputvector;
pub mod state;
pub mod testing;

use std::{collections::HashMap, fs::File};

use ordering_server::{HookId, HookInvocation};

use csv::Reader;
use once_cell::sync::OnceCell;
#[cfg(test)]
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use streaming_transpositions::OgRank;

pub static CONCURRENCY_LIMIT: OnceCell<usize> = OnceCell::new();

const TEST_TIMEOUT_SECS: u64 = 5;
const MAX_ERROR_LINES: usize = 20;

#[derive(Debug, Clone, Copy)]
pub struct ThreadId(usize);

#[derive(Debug, Serialize, Deserialize)]
pub struct HookInvocationCounts {
  hid2ic: HashMap<HookId, u32>,
  ogrank2hinvoc: Vec<HookInvocation>,
  n_processes: usize,
}

pub mod exec {
  use std::{
    fmt::{Display, Formatter},
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
  };

  use log::{debug, error, warn};
  use serde::{Deserialize, Serialize};
  use tokio::io::AsyncBufReadExt;

  use crate::{env::EnvironmentUpdate, io::TempDir, MAX_ERROR_LINES, TEST_TIMEOUT_SECS};

  #[derive(Debug, Serialize, Deserialize, Clone)]
  pub struct Executable(PathBuf);

  #[derive(Debug, Serialize, Deserialize, Clone, Copy)]
  pub enum Status {
    Timeout,
    TerminatedBySignal,
    Termination(i32),
  }

  impl Status {
    pub fn code(&self) -> Option<i32> {
      match self {
        Status::Timeout => None,
        Status::TerminatedBySignal => None,
        Status::Termination(status) => Some(*status),
      }
    }
    pub fn is_success(&self) -> bool {
      match self {
        Status::Timeout => false,
        Status::TerminatedBySignal => false,
        Status::Termination(status) => *status == 0,
      }
    }
    pub fn is_timeout(&self) -> bool {
      match self {
        Status::Timeout => true,
        Status::TerminatedBySignal => false,
        Status::Termination(_) => false,
      }
    }
    fn from_result(result: Option<std::process::ExitStatus>) -> Self {
      if let Some(status) = result {
        if let Some(code) = status.code() {
          Status::Termination(code)
        } else {
          Status::TerminatedBySignal
        }
      } else {
        Status::Timeout
      }
    }
  }

  #[derive(Debug, Serialize, Deserialize, Clone)]
  pub struct ExecResult {
    pub status: Status,
    pub selected_output: Vec<String>,
    pub stderr: String,
  }

  impl Display for ExecResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
      write!(f, "status: {:?}", self.status)?;
      write!(f, "\nselected output:\n{:?}", self.selected_output)?;
      write!(f, "\nstderr:\n{}\n\n", self.stderr)?;
      Ok(())
    }
  }

  impl ExecResult {
    pub fn retain_output(&mut self, f: impl Fn(&str) -> bool) {
      self.selected_output.retain(|s| f(s));
    }
  }

  impl Display for Executable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0.display())
    }
  }

  impl Executable {
    pub fn new(path: PathBuf) -> Self {
      Self(path)
    }

    pub fn name(&self) -> String {
      self
        .0
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
        .split('-')
        .next()
        .unwrap()
        .to_string()
    }

    pub async fn run(
      // TODO: make this async
      &self,
      env: EnvironmentUpdate<'_>,
      cwd: &TempDir,
      output_filter: Box<impl Fn(&str) -> bool + std::marker::Send + 'static>,
    ) -> ExecResult {
      let mut child = tokio::process::Command::new(
        self
          .0
          .canonicalize()
          .expect("failed to resolve executable path")
          .as_os_str(),
      )
      .envs(env.get_evars())
      .current_dir(&cwd.0)
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .expect("failed to execute program");
      let (tselected_output, rselected_output) = mpsc::channel();
      let stdout = child.stdout.take();
      let stderr = child.stderr.take();
      let pid = child
        .id()
        .expect("the child has not been polled, so it cannot have been reaped");
      let mut out_lines = tokio::io::BufReader::new(stdout.unwrap()).lines();
      tokio::task::spawn(async move {
        let mut out: Vec<String> = vec![];
        while let Some(line) = out_lines.next_line().await.transpose() {
          if let Err(e) = line {
            error!("failed to read line of stdout: {:?}", e);
            out.push("???".to_string());
          } else {
            let s = line.unwrap();
            // debug!("DEBUG: {}: {}", pid, s);
            out.push(s);
            if out.len() > MAX_ERROR_LINES {
              out.push("...".to_string());
              break;
            }
          }
        }
        if let Err(e) = tselected_output.send(out) {
          debug!("failed to send stderr of child process {pid}: {:?}", e);
        }
      });
      let (terr, rerr) = mpsc::channel();
      let mut err_lines = tokio::io::BufReader::new(stderr.unwrap()).lines();
      tokio::task::spawn(async move {
        let mut err = vec![];
        while let Some(line) = err_lines.next_line().await.transpose() {
          if let Err(e) = line {
            error!("failed to read line of stderr: {:?}", e);
            err.push("???".to_string());
          } else {
            let s = line.unwrap();
            // debug!("DEBUG: {}: {}", pid, s);
            err.push(s.to_string());
            if err.len() > MAX_ERROR_LINES {
              err.push("...".to_string());
              break;
            }
          }
        }
        if let Err(e) = terr.send(err.join("\n")) {
          debug!("failed to send stderr of child process {pid}: {:?}", e);
        }
      });
      let mut result = None;
      for _ in 0..(TEST_TIMEOUT_SECS * 100) {
        if let Some(status) = child
          .try_wait()
          .expect("unexpected error occurred while checking if child process has terminated")
        {
          result = Some(status);
          break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
      }
      if result.is_none() {
        warn!(
          "killing child process {:?} in {:?} due to timeout",
          pid, cwd.0
        );
        let mut kill = tokio::process::Command::new("kill")
          .args([
            "-s",
            "TERM",
            &child
              .id()
              .expect("not reaped because the latest try_wait() failed")
              .to_string(),
          ])
          .spawn()
          .unwrap();
        kill.wait().await.unwrap();
        child
          .wait()
          .await
          .expect("failed to wait for child process");
      }
      ExecResult {
        status: Status::from_result(result),
        selected_output: rselected_output
          .recv_timeout(Duration::from_secs(3))
          .map_err(|e| {
            error!("failed to read output of child process {pid}: {:?}", e);
            e
          })
          .unwrap_or_default(),
        stderr: rerr
          .recv_timeout(Duration::from_secs(3))
          .unwrap_or_default(),
      }
    }
  }
}

pub mod env {
  use std::sync::Mutex;
  use std::{collections::HashMap, ffi::OsString};

  use crate::io::TempDir;
  use crate::ThreadId;
  use crate::CONCURRENCY_LIMIT;

  const LF_FED_PORT: &str = "LF_FED_PORT";

  #[derive(Debug)]
  pub struct EnvironmentUpdate<'a> {
    evars: HashMap<OsString, OsString>,
    _scratch: Option<&'a TempDir>, // enforce that the scratch directory is not dropped before the environment update
  }

  use log::error;
  use once_cell::sync::Lazy;

  static OPEN_PORTS: Lazy<Vec<u16>> = Lazy::new(|| {
    (1024..32768)
      .filter(|p| std::net::TcpListener::bind(("127.0.0.1", *p)).is_ok())
      .collect()
  });
  static OPEN_PORTS_IDX: Mutex<usize> = Mutex::new(0);
  static PORTS_BY_TID: Lazy<Vec<OsString>> = Lazy::new(|| {
    let mut ret = Vec::new();
    for _ in 0..(*CONCURRENCY_LIMIT.wait()) {
      ret.push(OsString::from(get_valid_port().to_string()));
    }
    ret
  });

  const REQUIRED_CONTIGUOUS_PORTS: u16 = 24;
  const MAX_REQUIRED_PORTS: u16 = 36;

  pub fn get_valid_port() -> u16 {
    // 1024 is the first valid port, and one test may use a few ports (by trying them in sequence)
    // if they have physical connections. Assume the tests do not use more than MAX_REQUIRED_PORTS
    // ports each.
    let mut open_ports_idx = match OPEN_PORTS_IDX.lock() {
      Ok(guard) => guard,
      Err(poisoned) => {
        error!("poisoned mutex: {:?}", poisoned);
        panic!("poisoned mutex")
      }
    };
    let mut current: usize = *open_ports_idx;
    if current + 10 >= OPEN_PORTS.len() {
      panic!("not enough open ports");
    }
    while OPEN_PORTS[current + (REQUIRED_CONTIGUOUS_PORTS as usize)]
      > OPEN_PORTS[current] + REQUIRED_CONTIGUOUS_PORTS
    {
      current += 1;
    }
    *open_ports_idx = current + (MAX_REQUIRED_PORTS as usize);
    OPEN_PORTS[current]
  }

  pub fn stringify_dvec(conl: &[(u32, i16)], offset: u32) -> String {
    let mut ret = String::new();
    ret.push_str(&format!("{}\n", conl.len()));
    for (idx, delay) in conl {
      let adjusted = ((*idx as i32) - (offset as i32)) as u32;
      ret.push_str(&format!("{} {}\n", adjusted, delay));
    }
    ret
  }

  impl<'a> EnvironmentUpdate<'a> {
    pub fn new<T>(tid: ThreadId, tups: &[(T, T)]) -> Self
    where
      T: Into<OsString> + Clone + std::fmt::Debug,
    {
      let mut evars: HashMap<OsString, OsString> = HashMap::new();
      for (k, v) in tups {
        evars.insert(k.clone().into(), v.clone().into());
      }
      evars.insert(OsString::from(LF_FED_PORT), PORTS_BY_TID[tid.0].clone());
      Self {
        evars,
        _scratch: None,
      }
    }

    pub fn insert(&mut self, key: OsString, value: OsString) {
      self.evars.insert(key, value);
    }

    pub fn get_evars(&self) -> &HashMap<OsString, OsString> {
      &self.evars
    }
  }
}

#[derive(Debug)]
pub struct Traces(HashMap<String, Reader<File>>);

const DELAY_VECTOR_CHUNK_SIZE: usize = 8;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConstraintListIndex(u32);
pub type ConstraintListRegistry = Vec<ConstraintList>;
#[derive(Debug, Serialize, Deserialize)]
pub struct ConstraintList {
  waiter_idxs: [u32; DELAY_VECTOR_CHUNK_SIZE],
  notifier_delta_idxs: [i16; DELAY_VECTOR_CHUNK_SIZE],
  parent: Option<ConstraintListIndex>,
  length: u32,
}

impl ConstraintList {
  pub fn singleton(waiter_idx: OgRank, notifier_idx: OgRank, length: u32) -> Self {
    let mut waiter_idxs = [0; DELAY_VECTOR_CHUNK_SIZE];
    let mut notifier_delta_idxs = [0; DELAY_VECTOR_CHUNK_SIZE];
    waiter_idxs[0] = waiter_idx.0;
    notifier_delta_idxs[0] = (notifier_idx.0 as i32 - waiter_idx.0 as i32) as i16;
    Self {
      waiter_idxs,
      notifier_delta_idxs,
      parent: None,
      length,
    }
  }
  pub fn num_of_pairs(&self, clr: &ConstraintListRegistry) -> usize {
    let mut current = Some(self);
    let mut ret = 0;
    while let Some(node) = current {
      ret += DELAY_VECTOR_CHUNK_SIZE;
      current = node.parent.as_ref().map(|idx| &clr[idx.0 as usize]);
    }
    ret
  }
  pub fn to_pairs_sorted(&self, clr: &ConstraintListRegistry) -> Vec<(OgRank, OgRank)> {
    let mut current = Some(self);
    let mut ret = Vec::new();
    while let Some(node) = current {
      for i in 0..DELAY_VECTOR_CHUNK_SIZE {
        ret.push((
          OgRank(node.waiter_idxs[i]),
          OgRank((node.waiter_idxs[i] as i32 + node.notifier_delta_idxs[i] as i32) as u32),
        ));
      }
      current = node.parent.as_ref().map(|idx| &clr[idx.0 as usize]);
    }
    ret.sort_by_key(|(idx, _)| *idx);
    ret
  }
}

impl Traces {
  pub fn hooks_and_outs(&mut self) -> (Vec<TraceRecord>, Vec<TraceRecord>) {
    let mut raw_traces: Vec<TraceRecord> = self
      .0
      .values_mut()
      .flat_map(|reader| reader.deserialize())
      .map(|r| r.expect("could not read record"))
      .collect();
    raw_traces.sort_by_key(|tr| tr.elapsed_physical_time);
    let raw_traces_rti_only: Vec<_> = raw_traces
      .iter()
      .filter(|tr| tr.source == -1)
      .cloned()
      .collect();
    (raw_traces, raw_traces_rti_only)
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TraceRecord {
  #[serde(rename = "Event")]
  event: String,
  #[serde(rename = "Reactor")]
  reactor: String,
  #[serde(rename = "Source")]
  source: i32,
  #[serde(rename = "Destination")]
  destination: i32,
  #[serde(rename = "Elapsed Logical Time")]
  elapsed_logical_time: i64,
  #[serde(rename = "Microstep")]
  microstep: i64,
  #[serde(rename = "Elapsed Physical Time")]
  elapsed_physical_time: i64,
  #[serde(rename = "Trigger")]
  trigger: String,
  #[serde(rename = "Extra Delay")]
  extra_delay: u64,
  #[serde(rename = "File Index")]
  file_index: u32,
  #[serde(rename = "Line Number")]
  line_number: u32,
  #[serde(rename = "Sequence Number for File and Line")]
  sequence_number_for_file_and_line: u32,
}
#[cfg(test)]
impl TraceRecord {
  pub fn mock() -> Self {
    let rng = &mut rand::thread_rng();
    Self {
      event: vec!["A", "B", "C", "D"].choose(rng).unwrap().to_string(),
      reactor: vec!["R", "S", "T", "U"].choose(rng).unwrap().to_string(),
      source: rand::random(),
      destination: rand::random(),
      elapsed_logical_time: rand::random(),
      microstep: rand::random(),
      elapsed_physical_time: rand::random(),
      trigger: vec!["W", "X", "Y", "Z"].choose(rng).unwrap().to_string(),
      extra_delay: rand::random(),
      file_index: rand::random::<u32>() % 10,
      line_number: rand::random::<u32>() % 10,
      sequence_number_for_file_and_line: rand::random::<u32>() % 100,
    }
  }
}
