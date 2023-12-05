use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Formatter},
    str::FromStr,
};

pub mod axioms;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    FedId,
    Ack,
    Timestamp,
    Net,
    PortAbs,
    Ptag,
    TaggedMsg,
    Tag,
    StopReq,
    StopReqRep,
    StopGrn,
    Ltc,
    FirstTagOrPtag,
    FirstPortAbsOrTaggedMsg,
    FirstNet,
}

impl FromStr for EventType {
    type Err = ();
    fn from_str(event: &str) -> Result<Self, Self::Err> {
        match event {
            "FED_ID" => Ok(EventType::FedId),
            "ACK" => Ok(EventType::Ack),
            "TIMESTAMP" => Ok(EventType::Timestamp),
            "NET" => Ok(EventType::Net),
            "PTAG" => Ok(EventType::Ptag),
            "TAGGED_MSG" => Ok(EventType::TaggedMsg),
            "TAG" => Ok(EventType::Tag),
            "STOP_REQ" => Ok(EventType::StopReq),
            "STOP_REQ_REP" => Ok(EventType::StopReqRep),
            "STOP_GRN" => Ok(EventType::StopGrn),
            "LTC" => Ok(EventType::Ltc),
            "PORT_ABS" => Ok(EventType::PortAbs),
            _ => Err(()),
        }
    }
}

impl FromStr for Event {
    type Err = ();
    fn from_str(event: &str) -> Result<Self, ()> {
        let mut iter = event.split_whitespace();
        if let (Some(event_direction), Some(event_type)) = (iter.next(), iter.next()) {
            if iter.next().is_some() {
                return Err(());
            }
            let event_type = EventType::from_str(event_type)?;
            match event_direction {
                "Sending" => Ok(Event::Send(event_type)),
                "Receiving" => Ok(Event::Recv(event_type)),
                _ => Err(()),
            }
        } else {
            Err(())
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    Send(EventType),
    Recv(EventType),
}

/// If two events match a rule, then the rule says that there is a precedence relation between them
/// (with the preceding event occurring first in all non-error traces).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rule {
    pub preceding_event: Event,
    pub event: Event,
    pub relations: Vec<Relation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Relation {
    TagEqual,
    TagLessThan,
    TagLessThanOrEqual,
    TagFinite,
    FederateEqual,
}

pub fn preceding_permutables_by_ogrank(
    trace: &[TraceRecord],
    axioms: &[Rule],
    always_occurring: HashSet<OgRank>,
) -> Vec<HashSet<OgRank>> {
    let unpermutables = Unpermutables::from_realizable_trace(trace, axioms, always_occurring);
    unpermutables.preceding_permutables_by_ogrank()
}

pub struct Unpermutables {
    pub ogrank2immediatepredecessors: Vec<HashSet<OgRank>>,
    pub always_occurring: HashSet<OgRank>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OgRank(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceRecord {
    pub event: Event,
    pub tag: (i64, i64),
    pub fedid: i32,
}

impl Display for TraceRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} ({}, {}) @ {:?}",
            self.event, self.tag.0, self.tag.1, self.fedid
        )
    }
}

impl TraceRecord {
    pub fn from_lf_trace_record(lf_trace_record: &lf_trace_reader::TraceRecord) -> Self {
        let event = Event::from_str(&lf_trace_record.event).unwrap();
        let tag = (
            lf_trace_record.elapsed_logical_time,
            lf_trace_record.microstep,
        );
        let source = lf_trace_record.destination;
        Self {
            event,
            tag,
            fedid: source,
        }
    }
}

pub struct Elaborated2Og(pub HashMap<OgRank, OgRank>);

pub fn elaborated_from_lf_trace_records(
    lf_trace_records: Vec<lf_trace_reader::TraceRecord>,
) -> (Vec<TraceRecord>, Elaborated2Og) {
    let mut ret = Vec::new();
    let mut elaborated2og = HashMap::new();
    let mut firsts = HashSet::new();
    for (ogidx, record) in lf_trace_records.iter().enumerate() {
        let tr = TraceRecord::from_lf_trace_record(record);
        match tr.event {
            Event::Send(EventType::Ptag) | Event::Send(EventType::Tag) => {
                let elaborated = TraceRecord {
                    event: Event::Send(EventType::FirstTagOrPtag),
                    tag: tr.tag,
                    fedid: tr.fedid,
                };
                if !firsts.contains(&elaborated) {
                    firsts.insert(elaborated);
                    ret.push(elaborated);
                }
            }
            Event::Recv(EventType::TaggedMsg) | Event::Recv(EventType::PortAbs) => {
                let elaborated = TraceRecord {
                    event: Event::Recv(EventType::FirstPortAbsOrTaggedMsg),
                    tag: tr.tag,
                    fedid: tr.fedid,
                };
                if !firsts.contains(&elaborated) {
                    firsts.insert(elaborated);
                    ret.push(elaborated);
                }
            }
            Event::Recv(EventType::Net) => {
                let elaborated = TraceRecord {
                    event: Event::Recv(EventType::FirstNet),
                    tag: tr.tag,
                    fedid: tr.fedid,
                };
                if !firsts.contains(&elaborated) {
                    firsts.insert(elaborated);
                    ret.push(elaborated);
                }
            }
            _ => {}
        }
        elaborated2og.insert(OgRank(ret.len() as u32), OgRank(ogidx as u32));
        ret.push(tr);
    }
    (ret, Elaborated2Og(elaborated2og))
}

impl Rule {
    fn matches(&self, tr_before: &TraceRecord, tr_after: &TraceRecord) -> bool {
        self.event == tr_after.event
            && self.preceding_event == tr_before.event
            && self
                .relations
                .iter()
                .all(|rel| rel.holds(tr_before, tr_after))
    }
}

impl Relation {
    fn holds(&self, tr_before: &TraceRecord, tr_after: &TraceRecord) -> bool {
        fn finite(a: i64) -> bool {
            a.abs() < 1_000_000_000_000
        }
        match self {
            Relation::TagEqual => tr_before.tag == tr_after.tag,
            Relation::TagLessThan => tr_before.tag < tr_after.tag,
            Relation::TagLessThanOrEqual => tr_before.tag <= tr_after.tag,
            Relation::FederateEqual => tr_before.fedid == tr_after.fedid,
            Relation::TagFinite => finite(tr_before.tag.0) && finite(tr_after.tag.0),
        }
    }
}

impl OgRank {
    fn idx(&self) -> usize {
        self.0 as usize
    }
}

impl Unpermutables {
    pub fn from_realizable_trace(
        trace: &[TraceRecord],
        axioms: &[Rule],
        always_occurring: HashSet<OgRank>,
    ) -> Self {
        let mut ogrank2immediatepredecessors = Vec::new();
        for (ogrank, tr) in trace.iter().enumerate() {
            let mut immediate_predecessors = HashSet::new();
            for rule in axioms {
                immediate_predecessors.extend(Self::apply_rule(
                    rule,
                    tr,
                    &trace[..ogrank],
                    &trace[ogrank + 1..],
                ));
            }
            ogrank2immediatepredecessors.push(immediate_predecessors);
        }
        Self {
            ogrank2immediatepredecessors,
            always_occurring,
        }
    }
    /// An ogrank A is _preceding_ an ogrank B iff A is numerically less than B, having occurred
    /// first in the OG trace.
    ///
    /// An ogrank is a _predecessor_ of an ogrank B if it precedes B in _all_ non-error traces.
    ///
    /// Preceding is transitive; predecessor is only transitive when restricted to events that
    /// always occur.
    ///
    /// Preceding is easy to represent compactly because it is a total order. Predecessor is harder.
    ///
    /// The OG trace is a non-error trace, so the relation _preceding_ is a superset of the relation
    /// _predecessor_.
    ///
    /// A preceding ogrank is a predecessor of the current ogrank iff it is in the union of the sets
    /// of predecessors of the immediate and always occurring predecessors of the current ogrank.
    /// That is, it is a preceding non-predecessor of the current ogrank iff it is in the
    /// intersection of the sets of (preceding or non-preceding) non-predecessors of the immediate
    /// and always occurring predecessors of the current ogrank. The utility of this demorganish
    /// restatement is that the sets of preceding non-predecessors is presumed to be smaller than
    ///
    /// This function implements a dynamic programming algorithm to compute the sets of preceding
    /// non-predecessors.
    pub fn preceding_permutables_by_ogrank(&self) -> Vec<HashSet<OgRank>> {
        let mut ret: Vec<HashSet<OgRank>> = Vec::new();
        for ogrank in 0..self.ogrank2immediatepredecessors.len() {
            let immediate_predecessors = &self.ogrank2immediatepredecessors[ogrank];
            let candidate_non_predecessors_size = |other: OgRank| {
                ret[other.idx()].len() as u32 + (ogrank as u32) - other.0
                // the non-predecessors of `other` that precede `ogrank` are partitioned into the
                // preceding non-predecessors of `other` and the ogranks that are preceding of
                // `ogrank` but not preceding of `other`
            };
            // start with the smallest for efficiency since we're going to be intersecting
            let (ipred0, smallest_non_predecessor_set) = immediate_predecessors
                .iter()
                .filter(|ogrank| self.always_occurring.contains(ogrank))
                .min_by_key(|ogrank| candidate_non_predecessors_size(**ogrank))
                .map(|ogrank| (Some(*ogrank), ret[ogrank.idx()].clone()))
                .unwrap_or_default();
            if let Some(ipred0) = ipred0 {
                let mut running_intersection = smallest_non_predecessor_set;
                // the sets implicitly contain everything greater than to `ipred0`
                running_intersection.extend((ipred0.0 + 1..ogrank as u32).map(OgRank));
                if ogrank == 17 {
                    for ipred in immediate_predecessors.iter().filter(|ogrank| {
                        self.always_occurring.contains(ogrank) && **ogrank != ipred0
                    }) {
                        let mut remove_list = Vec::new();
                        for ogrank in running_intersection.iter() {
                            // the sets implicitly contain everything greater than `ipred`
                            if !(ret[ipred.idx()].contains(ogrank) || ogrank > ipred) {
                                remove_list.push(*ogrank);
                            }
                        }
                        for ogrank in remove_list {
                            running_intersection.remove(&ogrank);
                        }
                    }
                    println!(
                        "immediate_predecessors: {:?} at # 17",
                        immediate_predecessors
                    );
                    println!(
                        "smallest_non_predecessor_set: {:?} and ipred0: {} and ret length: {}",
                        running_intersection,
                        ipred0.0,
                        ret.len()
                    );
                }
                ret.push(running_intersection);
            } else {
                ret.push((0..ogrank as u32).map(OgRank).collect());
            }
        }
        ret
    }
    fn apply_rule(
        rule: &Rule,
        tr: &TraceRecord,
        before: &[TraceRecord],
        after: &[TraceRecord],
    ) -> HashSet<OgRank> {
        let mut immediate_predecessors = HashSet::new();
        if after.iter().any(|tr_after| rule.matches(tr_after, tr)) {
            panic!(
                "Tracepoint:\n    {:?}\nfollowed by:\n    {:?} is a counterexample to the axiom:\n{:?}",
                tr, after, rule
            );
        }
        if rule.event == tr.event {
            immediate_predecessors.extend(
                before
                    .iter()
                    .enumerate()
                    .filter(|(_, tr_before)| rule.matches(tr_before, tr))
                    .map(|(ogrank, _)| OgRank(ogrank as u32)),
            );
        }
        immediate_predecessors
    }
}
