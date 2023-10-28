#[allow(dead_code)]
mod io;
pub mod state;

use std::{collections::HashMap, fs::File};

use csv::Reader;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HookId(String);

#[derive(Debug, Serialize, Deserialize)]
pub struct InvocationCounts(HashMap<HookId, u32>);

pub mod exec {
  use std::{
    fmt::{Display, Formatter},
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::Command,
  };

  use serde::{Deserialize, Serialize};
  use wait_timeout::ChildExt;

  use crate::env::EnvironmentUpdate;

  #[derive(Debug, Serialize, Deserialize)]
  pub struct Executable(PathBuf);

  #[derive(Debug, Serialize, Deserialize)]
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

  #[derive(Debug, Serialize, Deserialize)]
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
      cwd: &Path,
      output_filter: impl Fn(&str) -> bool,
    ) -> ExecResult {
      println!("running with evars: {:?}", env.get_evars());
      let mut child = Command::new(
        self
          .0
          .canonicalize()
          .expect("failed to resolve executable path")
          .as_os_str(),
      )
      .envs(env.get_evars())
      .current_dir(cwd)
      // .stdout(Stdio::piped())
      .spawn()
      .expect("failed to execute program");
      let result = child
        .wait_timeout(core::time::Duration::from_secs(10))
        .expect("failed to wait for program");
      let selected_output: Vec<String> = BufReader::new(child.stdout.take().unwrap())
        .lines()
        .map(|l| l.expect("failed to read line of output"))
        .filter(|s| output_filter(s))
        .collect();
      let mut stderr = String::new();
      child
        .stderr
        .take()
        .expect("failed to take stderr of child process")
        .read_to_string(&mut stderr)
        .expect("output of run executable is not utf-8");
      ExecResult {
        status: Status::from_result(result),
        selected_output,
        stderr,
      }
    }
  }
}

pub mod env {
  use std::{collections::HashMap, ffi::OsString};

  use crate::{DelayVector, InvocationCounts};

  const LF_FED_PORT: &str = "LF_FED_PORT";

  #[derive(Debug)]
  pub struct EnvironmentUpdate {
    evars: HashMap<OsString, OsString>,
  }

  fn stringify_dvec(dvec: &[u64]) -> OsString {
    OsString::from(dvec.iter().fold(String::from(""), |mut acc, x| {
      acc.push_str(&x.to_string());
      acc
    }))
  }

  fn get_valid_port() -> OsString {
    let tid = rayon::current_thread_index().unwrap_or(0);
    // 1024 is the first valid port, and one test may use a few ports (by trying them in sequence)
    // if they have physical connections. Assume the tests do not use more than 10 ports each.
    let tid = tid * 10 + 1024;
    if tid > 2_usize.pow(15) - 1 {
      panic!("too many threads");
    }
    OsString::from((tid as u16).to_string())
  }

  impl Default for EnvironmentUpdate {
    fn default() -> Self {
      Self {
        evars: HashMap::from([(LF_FED_PORT.into(), get_valid_port())]),
      }
    }
  }

  impl EnvironmentUpdate {
    pub fn new(tups: &[(&str, &str)]) -> Self {
      let mut ret = Self::default();
      for (k, v) in tups {
        ret.evars.insert(OsString::from(*k), OsString::from(*v));
      }
      ret
    }

    pub fn insert(&mut self, key: OsString, value: OsString) {
      self.evars.insert(key, value);
    }

    pub fn get_evars(&self) -> &HashMap<OsString, OsString> {
      &self.evars
    }

    pub fn delayed(ic: &InvocationCounts, dvec: &DelayVector) -> Self {
      let mut ret = Self::default();
      let mut cumsum: usize = 0;
      for (hid, k) in ic.to_vec() {
        ret.evars.insert(
          OsString::from(hid.0.clone()),
          stringify_dvec(&dvec.0[cumsum..cumsum + (*k as usize)]),
        );
        cumsum += *k as usize;
      }
      ret
    }
  }
}

#[derive(Debug)]
pub struct Traces(HashMap<String, Reader<File>>);

#[derive(Debug, Serialize, Deserialize)]
pub struct DelayVector(Vec<u64>);
#[derive(Debug, Serialize, Deserialize)]
pub struct DelayParams {
  pub max_expected_wallclock_overhead: u64,
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
