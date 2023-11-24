use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};

pub mod client;
mod connection;
pub mod server;

pub struct PrecedenceId(pub u32);
impl PrecedenceId {
    fn idx(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
pub struct SequenceNumberByFileAndLine(pub u32);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct HookInvocation {
    pub hid: HookId,
    pub seqnum: SequenceNumberByFileAndLine,
}

pub struct Precedence {
    sender2waiters: HashMap<HookInvocation, Vec<HookInvocation>>,
    n_connections: usize,
}
pub type HookInvocationShort<'a> = (&'a str, u32);
pub type PrecedenceElement<'a> = ((&'a str, u32), &'a [(&'a str, u32)]);
impl Precedence {
    pub fn new(
        n_connections: usize,
        sender2waiters: HashMap<HookInvocation, Vec<HookInvocation>>,
    ) -> Self {
        Self {
            sender2waiters,
            n_connections,
        }
    }
    pub fn from_list(n_connections: usize, sender2waiters: &[PrecedenceElement]) -> Self {
        Self {
            sender2waiters: sender2waiters
                .iter()
                .map(|(k, v)| {
                    (
                        HookInvocation {
                            hid: HookId::new(k.0.to_string()),
                            seqnum: SequenceNumberByFileAndLine(k.1 as u32),
                        },
                        v.iter()
                            .map(|(k, v)| HookInvocation {
                                hid: HookId::new(k.to_string()),
                                seqnum: SequenceNumberByFileAndLine(*v as u32),
                            })
                            .collect(),
                    )
                })
                .collect(),
            n_connections,
        }
    }
    fn waits(&self) -> HashMap<HookId, Vec<SequenceNumberByFileAndLine>> {
        let mut waits: HashMap<HookId, Vec<SequenceNumberByFileAndLine>> = HashMap::new();
        for k in self.sender2waiters.values().flat_map(|v| v.iter()) {
            waits.entry(k.hid.clone()).or_default().push(k.seqnum);
        }
        waits
    }
    fn notifys(&self) -> HashMap<HookId, Vec<SequenceNumberByFileAndLine>> {
        let mut notifys: HashMap<HookId, Vec<SequenceNumberByFileAndLine>> = HashMap::new();
        for v in self.sender2waiters.keys() {
            notifys.entry(v.hid.clone()).or_default().push(v.seqnum);
        }
        notifys
    }
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Clone)]
pub struct HookId(String);

impl HookId {
    pub fn new(hid: String) -> Self {
        Self(hid)
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
            hid: HookId::new(his.0.to_string()),
            seqnum: SequenceNumberByFileAndLine(his.1 as u32),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub precedence_id: u32,
    pub federate_id: u32,
    pub hook_id: [u8; 32], // Assume hook id is no more than 31 ascii characters
    pub sequence_number: u32,
}
