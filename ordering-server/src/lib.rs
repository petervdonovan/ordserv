use std::{collections::HashMap, ffi::OsString, fmt::Display, path::PathBuf};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

pub mod client;
mod connection;
pub mod server;
mod tcpconnectionprovider;

pub const ORDSERV_PORT_ENV_VAR: &str = "ORDSERV_PORT";
pub const ORDSERV_WAIT_TIMEOUT_MILLISECONDS_ENV_VAR: &str = "ORDSERV_WAIT_TIMEOUT";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrecedenceId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunId(pub u32);

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
pub struct SequenceNumberByFileAndLine(pub u32);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct HookInvocation {
    pub hid: HookId,
    pub seqnum: SequenceNumberByFileAndLine,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Precedence {
    pub sender2waiters: HashMap<HookInvocation, Vec<HookInvocation>>,
    pub n_connections: usize,
    pub scratch_dir: PathBuf,
    pub run_id: u32, // to avoid getting mucked up by stragglers from previous runs
}
pub type HookInvocationShort<'a> = (&'a str, i32, u32);
pub type PrecedenceElement<'a> = (HookInvocationShort<'a>, &'a [HookInvocationShort<'a>]);
impl Precedence {
    pub fn from_list(
        n_connections: usize,
        sender2waiters: &[PrecedenceElement],
        scratch_dir: PathBuf,
        run_id: u32,
    ) -> Self {
        Self {
            sender2waiters: sender2waiters
                .iter()
                .map(|(k, v)| {
                    (
                        HookInvocation::from_short(*k),
                        v.iter().map(|v| HookInvocation::from_short(*v)).collect(),
                    )
                })
                .collect(),
            n_connections,
            scratch_dir,
            run_id,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct HookId(String, pub FederateId);

impl HookId {
    pub fn new(hid: String, fedid: FederateId) -> Self {
        Self(hid, fedid)
    }
}

impl Display for HookId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl HookInvocation {
    pub fn from_short(his: HookInvocationShort) -> Self {
        Self {
            hid: HookId::new(his.0.to_string(), FederateId(his.1)),
            seqnum: SequenceNumberByFileAndLine(his.2),
        }
    }
}

pub struct EnvironmentVariables(pub Vec<(OsString, OsString)>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct FederateId(pub i32);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub precedence_id: u32,
    pub federate_id: i32,
    pub hook_id: [u8; 32], // Assume hook id is no more than 31 ascii characters
    pub sequence_number: u32,
    pub run_id: u32,
}

pub(crate) fn channel_vec<T>(
    n_connection_streams: usize,
) -> (Vec<mpsc::Sender<T>>, Vec<mpsc::Receiver<T>>) {
    let mut senders = Vec::with_capacity(n_connection_streams);
    let mut receivers = Vec::with_capacity(n_connection_streams);
    for _ in 0..n_connection_streams {
        let (sender, receiver) = mpsc::channel(1);
        senders.push(sender);
        receivers.push(receiver);
    }
    (senders, receivers)
}
