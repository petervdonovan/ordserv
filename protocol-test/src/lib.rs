#[allow(dead_code)]
mod io;
pub mod outputvector;
pub mod state;
pub mod testing;

use std::{collections::HashMap, fs::File};

use ordering_server::HookId;

use csv::Reader;
use once_cell::sync::OnceCell;
#[cfg(test)]
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

pub static CONCURRENCY_LIMIT: OnceCell<usize> = OnceCell::new();

const TEST_TIMEOUT_SECS: u64 = 2;
const MAX_ERROR_LINES: usize = 20;

#[derive(Debug, Clone, Copy)]
pub struct ThreadId(usize);

#[derive(Debug, Serialize, Deserialize)]
pub struct HookInvocationCounts(HashMap<HookId, u32>);

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

  use serde::{Deserialize, Serialize};

  use crate::{env::EnvironmentUpdate, io::TempDir, TEST_TIMEOUT_SECS};

  #[derive(Debug, Serialize, Deserialize)]
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

    pub fn run(
      &self,
      env: EnvironmentUpdate,
      cwd: &TempDir,
      output_filter: Box<impl Fn(&str) -> bool + std::marker::Send + 'static>,
    ) -> ExecResult {
      let mut child = Command::new(
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
      let pid = child.id();
      thread::spawn(move || {
        let selected_output: Vec<String> = BufReader::new(stdout.unwrap())
          .lines()
          .map(|l| l.expect("failed to read line of output"))
          .filter(|s| output_filter(s))
          .collect();
        if let Err(e) = tselected_output.send(selected_output) {
          eprintln!("failed to send output of child process {pid}: {:?}", e);
        }
      });
      let (terr, rerr) = mpsc::channel();
      thread::spawn(move || {
        let err: Vec<String> = BufReader::new(stderr.unwrap())
          .lines()
          .map(|l| l.expect("failed to read line of output"))
          .take(crate::MAX_ERROR_LINES)
          .collect();
        if let Err(e) = terr.send(err.join("\n")) {
          eprintln!("failed to send stderr of child process {pid}: {:?}", e);
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
        thread::sleep(std::time::Duration::from_millis(10));
      }
      if result.is_none() {
        println!(
          "killing child process {:?} in {:?} due to timeout",
          pid, cwd.0
        );
        let mut kill = Command::new("kill")
          .args(["-s", "TERM", &child.id().to_string()])
          .spawn()
          .unwrap();
        kill.wait().unwrap();
        child.wait().expect("failed to wait for child process");
      }
      ExecResult {
        status: Status::from_result(result),
        selected_output: rselected_output
          .recv_timeout(Duration::from_secs(3))
          .map_err(|e| {
            println!("failed to read output of child process {pid}: {:?}", e);
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
  use std::io::Write;
  use std::sync::Mutex;
  use std::{collections::HashMap, ffi::OsString};

  use crate::CONCURRENCY_LIMIT;
  use crate::{io::TempDir, DelayVector, HookInvocationCounts};
  use crate::{DelayVectorRegistry, ThreadId};

  const LF_FED_PORT: &str = "LF_FED_PORT";
  const LF_FED_DELAYS: &str = "LF_FED_DELAYS";

  #[derive(Debug)]
  pub struct EnvironmentUpdate<'a> {
    evars: HashMap<OsString, OsString>,
    _scratch: Option<&'a TempDir>, // enforce that the scratch directory is not dropped before the environment update
  }

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
      ret.push(get_valid_port());
    }
    ret
  });

  const REQUIRED_CONTIGUOUS_PORTS: u16 = 24;
  const MAX_REQUIRED_PORTS: u16 = 36;

  fn get_valid_port() -> OsString {
    // 1024 is the first valid port, and one test may use a few ports (by trying them in sequence)
    // if they have physical connections. Assume the tests do not use more than 10 ports each.
    let mut open_ports_idx = match OPEN_PORTS_IDX.lock() {
      Ok(guard) => guard,
      Err(poisoned) => {
        println!("poisoned mutex: {:?}", poisoned);
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
    let port = OPEN_PORTS[current];
    // An IndexOutOfBounds exception around here indicates that we are trying to open too many sockets
    OsString::from(port.to_string())
  }

  pub fn stringify_dvec(dvec: &[(u32, i16)], offset: u32) -> String {
    let mut ret = String::new();
    ret.push_str(&format!("{}\n", dvec.len()));
    for (idx, delay) in dvec {
      let adjusted = ((*idx as i32) - (offset as i32)) as u32;
      ret.push_str(&format!("{} {}\n", adjusted, delay));
    }
    ret
  }

  impl<'a> EnvironmentUpdate<'a> {
    pub fn new(tid: ThreadId, tups: &[(&str, &str)]) -> Self {
      let mut evars = HashMap::new();
      for (k, v) in tups {
        evars.insert(OsString::from(*k), OsString::from(*v));
      }
      evars.insert(OsString::from(LF_FED_PORT), PORTS_BY_TID[tid.0].clone());
      evars.insert(OsString::from(LF_FED_DELAYS), OsString::new());
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

    pub fn delayed(
      ic: &HookInvocationCounts,
      dvec: &DelayVector,
      tmp: &TempDir,
      tid: ThreadId,
      dvr: &DelayVectorRegistry,
    ) -> Self {
      let mut ret = Self::new(tid, &[]);
      let mut current: usize = 0;
      let mut cumsum: u32 = 0;
      let delay = tmp.rand_file("delay");
      let mut delay_f = std::fs::File::create(&delay).expect("could not create delay file");
      let pairs_sorted = dvec.to_pairs_sorted(dvr);
      writeln!(delay_f, "{}", ic.0.len()).expect("could not write delay file");
      for (hid, k) in ic.to_vec() {
        let start = current;
        while current < pairs_sorted.len() && pairs_sorted[current].0 < cumsum + *k {
          current += 1;
        }
        write!(
          delay_f,
          "{}\n{}",
          hid,
          stringify_dvec(&pairs_sorted[start..current], cumsum),
        )
        .expect("could not write delay file");
        cumsum += *k;
      }
      ret
        .evars
        .insert(OsString::from(LF_FED_DELAYS), delay.into_os_string());
      ret
    }
  }
}

#[derive(Debug)]
pub struct Traces(HashMap<String, Reader<File>>);

const DELAY_VECTOR_CHUNK_SIZE: usize = 8;

#[derive(Debug, Serialize, Deserialize)]
pub struct DelayVectorIndex(u32);
pub type DelayVectorRegistry = Vec<DelayVector>;
#[derive(Debug, Serialize, Deserialize)]
pub struct DelayVector {
  idxs: [u32; DELAY_VECTOR_CHUNK_SIZE],
  delta_delays: [i16; DELAY_VECTOR_CHUNK_SIZE],
  parent: Option<DelayVectorIndex>,
  length: u32,
}

impl DelayVector {
  pub fn num_of_pairs(&self, dvr: &DelayVectorRegistry) -> usize {
    let mut current = Some(self);
    let mut ret = 0;
    while let Some(node) = current {
      ret += DELAY_VECTOR_CHUNK_SIZE;
      current = node.parent.as_ref().map(|idx| &dvr[idx.0 as usize]);
    }
    ret
  }
  pub fn to_pairs_sorted(&self, dvr: &DelayVectorRegistry) -> Vec<(u32, i16)> {
    let mut current = Some(self);
    let mut ret = Vec::new();
    while let Some(node) = current {
      for i in 0..DELAY_VECTOR_CHUNK_SIZE {
        ret.push((node.idxs[i], node.delta_delays[i]));
      }
      current = node.parent.as_ref().map(|idx| &dvr[idx.0 as usize]);
    }
    ret.sort_by_key(|(idx, _)| *idx);
    ret
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DelayParams {
  pub max_expected_wallclock_overhead_ms: i16,
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
