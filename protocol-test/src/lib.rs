#[allow(dead_code)]
mod io;
mod outputvector;
pub mod state;
pub mod testing;

use std::{collections::HashMap, fs::File};

use csv::Reader;
use serde::{Deserialize, Serialize};

pub const CONCURRENCY_LIMIT: usize = 350;

const TEST_TIMEOUT_SECS: u64 = 45;

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HookId(String);

#[derive(Debug, Clone, Copy)]
pub struct ThreadId(usize);

#[derive(Debug, Serialize, Deserialize)]
pub struct HookInvocationCounts(HashMap<HookId, u32>);

pub mod exec {
  use std::{
    fmt::{Display, Formatter},
    io::{BufRead, BufReader, Read},
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
  };

  use serde::{Deserialize, Serialize};
  use wait_timeout::ChildExt;

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
      write!(f, "\nstderr:\n{:?}", self.stderr)?;
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
      thread::spawn(move || {
        let selected_output: Vec<String> = BufReader::new(stdout.unwrap())
          .lines()
          .map(|l| l.expect("failed to read line of output"))
          .filter(|s| output_filter(s))
          .collect();
        tselected_output.send(selected_output).unwrap();
      });
      let mut result = None;
      for _ in 0..TEST_TIMEOUT_SECS {
        if let Some(status) = child
          .try_wait()
          .expect("unexpected error occurred while waiting")
        {
          result = Some(status);
          break;
        }
        thread::sleep(std::time::Duration::from_secs(1));
      }
      let mut stderr = String::new();
      child.wait().expect("failed to wait for child process");
      child
        .stderr
        .take()
        .expect("failed to take stderr of child process")
        .read_to_string(&mut stderr)
        .expect("output of run executable is not utf-8");
      ExecResult {
        status: Status::from_result(result),
        selected_output: rselected_output.recv().unwrap(),
        stderr,
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

  #[derive(Debug)]
  pub struct EnvironmentUpdate<'a> {
    evars: HashMap<OsString, OsString>,
    _scratch: Option<&'a TempDir>, // enforce that the scratch directory is not dropped before the environment update
  }

  fn stringify_dvec(dvec: &[u64]) -> String {
    dvec.iter().fold(String::from(""), |mut acc, x| {
      acc.push_str(&x.to_string());
      acc.push('\n');
      acc
    })
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
    for _ in 0..CONCURRENCY_LIMIT {
      ret.push(get_valid_port());
    }
    ret
  });

  const REQUIRED_CONTIGUOUS_PORTS: u16 = 5;
  const MAX_REQUIRED_PORTS: u16 = 10;

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

  impl<'a> EnvironmentUpdate<'a> {
    pub fn new(tid: ThreadId, tups: &[(&str, &str)]) -> Self {
      let mut evars = HashMap::new();
      for (k, v) in tups {
        evars.insert(OsString::from(*k), OsString::from(*v));
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

    pub fn delayed(
      ic: &HookInvocationCounts,
      dvec: &DelayVector,
      tmp: &TempDir,
      tid: ThreadId,
      dvr: &DelayVectorRegistry,
    ) -> Self {
      let mut ret = Self::new(tid, &[]);
      let mut cumsum: usize = 0;
      for (hid, k) in ic.to_vec() {
        let delay = tmp.rand_file("delay");
        let mut delay_f = std::fs::File::create(&delay).expect("could not create delay file");
        write!(
          delay_f,
          "{}",
          stringify_dvec(&dvec.unpack(dvr)[cumsum..cumsum + (*k as usize)]),
        )
        .expect("could not write delay file");
        ret
          .evars
          .insert(OsString::from(hid.0.clone()), delay.into_os_string());
        cumsum += *k as usize;
      }
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
  pub fn unpack(&self, dvr: &DelayVectorRegistry) -> Vec<u64> {
    let mut ret: Vec<i64> = vec![0; self.length as usize];
    let mut current = Some(self);
    while let Some(node) = current {
      for i in 0..DELAY_VECTOR_CHUNK_SIZE {
        ret[node.idxs[i] as usize] += node.delta_delays[i] as i64 * 1_000_000;
      }
      current = node.parent.as_ref().map(|idx| &dvr[idx.0 as usize]);
    }
    ret.iter().map(|x| *x as u64).collect()
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DelayParams {
  pub max_expected_wallclock_overhead: i16,
}

#[derive(Debug, Serialize, Deserialize)]
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
}
