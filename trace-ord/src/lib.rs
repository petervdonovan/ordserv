use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Formatter},
    path::Path,
    str::FromStr,
};

use conninfo::{ConnInfo, FedId, Tag};

pub mod axioms;
pub mod conninfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    RecvFedId,
    SendAck,
    SendTimestamp,
    RecvTimestamp,
    RecvNet,
    SendPortAbs,
    RecvPortAbs,
    SendPtag,
    SendTaggedMsg,
    RecvTaggedMsg,
    SendTag,
    RecvStopReq,
    SendStopReq,
    RecvStopReqRep,
    SendStopGrn,
    RecvLtc,
    SendFirstTagOrPtag,
    RecvFirstPortAbsOrTaggedMsg,
    FirstRecvNetOrSendStopGrnOrRecvLtc, // events that lead to sending a TAG
}

impl FromStr for Event {
    type Err = ();
    fn from_str(event: &str) -> Result<Self, Self::Err> {
        match event {
            "Receiving FED_ID" => Ok(Event::RecvFedId),
            "Sending ACK" => Ok(Event::SendAck),
            "Sending TIMESTAMP" => Ok(Event::SendTimestamp),
            "Receiving TIMESTAMP" => Ok(Event::RecvTimestamp),
            "Receiving NET" => Ok(Event::RecvNet),
            "Sending PTAG" => Ok(Event::SendPtag),
            "Sending TAGGED_MSG" => Ok(Event::SendTaggedMsg),
            "Receiving TAGGED_MSG" => Ok(Event::RecvTaggedMsg),
            "Sending TAG" => Ok(Event::SendTag),
            "Receiving STOP_REQ" => Ok(Event::RecvStopReq),
            "Sending STOP_REQ" => Ok(Event::SendStopReq),
            "Receiving STOP_REQ_REP" => Ok(Event::RecvStopReqRep),
            "Sending STOP_GRN" => Ok(Event::SendStopGrn),
            "Receiving LTC" => Ok(Event::RecvLtc),
            "Sending PORT_ABS" => Ok(Event::SendPortAbs),
            "Receiving PORT_ABS" => Ok(Event::RecvPortAbs),
            _ => Err(()),
        }
    }
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
    TagPlusDelayEqual,
    TagPlusDelayLessThan,
    TagPlusDelayLessThanOrEqual,
    FirstTagNonzero,
    TagFinite,
    FederateEqual,
    FederatesConnected,
}

pub fn preceding_permutables_by_ogrank_from_dir(
    dir: &Path,
) -> Result<Vec<HashSet<OgRank>>, String> {
    let rti_csv = dir.join("rti.csv");
    let conninfo = dir.join("conninfo.txt");
    let axioms = axioms::axioms();
    let always_occurring: HashSet<_> = (0..lf_trace_reader::trace_by_physical_time(&rti_csv).len())
        .map(|ogrank| OgRank(ogrank as u32))
        .collect();
    let (trace, _map2og) =
        elaborated_from_lf_trace_records(lf_trace_reader::trace_by_physical_time(&rti_csv));
    let conninfo = ConnInfo::from_str(&std::fs::read_to_string(conninfo).unwrap()).unwrap();
    preceding_permutables_by_ogrank(&trace, &axioms, always_occurring, &conninfo)
}

pub fn preceding_permutables_by_ogrank(
    trace: &[TraceRecord],
    axioms: &[Rule],
    always_occurring: HashSet<OgRank>,
    conninfo: &ConnInfo,
) -> Result<Vec<HashSet<OgRank>>, String> {
    let unpermutables =
        Unpermutables::from_realizable_trace(trace, axioms, always_occurring, conninfo)?;
    Ok(unpermutables.preceding_permutables_by_ogrank())
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
    pub tag: Tag,
    pub fedid: FedId,
}

impl Display for TraceRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {} @ {:?}", self.event, self.tag, self.fedid)
    }
}

fn tracerecords_to_string(
    trace: &[TraceRecord],
    put_marker_if: impl Fn(&TraceRecord) -> bool,
) -> String {
    trace
    .iter().map(|it| (it.to_string(), put_marker_if(it))).fold(String::new(), |s, (other, matches)| if s.is_empty() {String::new()} else {s + "\n"} + if matches {"▶ "} else {"  "} + &other)
}

impl TraceRecord {
    pub fn from_lf_trace_record(lf_trace_record: &lf_trace_reader::TraceRecord) -> Self {
        let event = Event::from_str(&lf_trace_record.event)
            .unwrap_or_else(|_| panic!("Unrecognized event: {}", lf_trace_record.event));
        let tag = Tag(
            lf_trace_record.elapsed_logical_time,
            lf_trace_record.microstep,
        );
        let source = lf_trace_record.destination;
        Self {
            event,
            tag,
            fedid: FedId(source),
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
        if let Some(event) = match tr.event {
            Event::SendPtag | Event::SendTag => Some(TraceRecord {
                event: Event::SendFirstTagOrPtag,
                tag: tr.tag,
                fedid: tr.fedid,
            }),
            Event::RecvTaggedMsg | Event::RecvPortAbs => Some(TraceRecord {
                event: Event::RecvFirstPortAbsOrTaggedMsg,
                tag: tr.tag,
                fedid: tr.fedid,
            }),
            Event::RecvNet => Some(TraceRecord {
                event: Event::FirstRecvNetOrSendStopGrnOrRecvLtc,
                tag: tr.tag,
                fedid: tr.fedid,
            }),
            Event::SendStopGrn => Some(TraceRecord {
                event: Event::FirstRecvNetOrSendStopGrnOrRecvLtc,
                tag: tr.tag,
                fedid: tr.fedid,
            }),
            _ => None,
        } {
            if !firsts.contains(&event) {
                firsts.insert(event);
                ret.push(event);
            }
        }
        // if tr.event == Event::RecvLtc {
        //     for downstream in conninfo::downstream_federates(tr.fedid) {
        //         let event = TraceRecord {
        //             event: Event::RecvLtc,
        //             tag: tr.tag,
        //             fedid: downstream,
        //         };
        //         if !firsts.contains(&event) {
        //             firsts.insert(event);
        //             ret.push(event);
        //         }
        //     }
        // }
        elaborated2og.insert(OgRank(ret.len() as u32), OgRank(ogidx as u32));
        ret.push(tr);
    }
    (ret, Elaborated2Og(elaborated2og))
}

impl Rule {
    fn matches(
        &self,
        tr_before: &TraceRecord,
        tr_after: &TraceRecord,
        conninfo: &ConnInfo,
    ) -> bool {
        self.event == tr_after.event
            && self.preceding_event == tr_before.event
            && self
                .relations
                .iter()
                .all(|rel| rel.holds(tr_before, tr_after, conninfo))
    }
}

impl Relation {
    fn holds(&self, tr_before: &TraceRecord, tr_after: &TraceRecord, conninfo: &ConnInfo) -> bool {
        fn finite(a: i64) -> bool {
            a.abs() < 1_000_000_000_000
        }
        let compare_before_plus_delay_with_after = |f: fn(&Tag, &Tag) -> bool| {
            if let Some(delay) = conninfo.0.get(&(tr_before.fedid, tr_after.fedid)) {
                f(&(tr_before.tag + *delay), &tr_after.tag)
            } else {
                false
            }
        };
        match self {
            Relation::TagPlusDelayEqual => compare_before_plus_delay_with_after(Tag::eq),
            Relation::TagPlusDelayLessThan => compare_before_plus_delay_with_after(Tag::lt),
            Relation::TagPlusDelayLessThanOrEqual => compare_before_plus_delay_with_after(Tag::le),
            Relation::FederateEqual => tr_before.fedid == tr_after.fedid,
            Relation::FirstTagNonzero => tr_before.tag != Tag(0, 0),
            Relation::FederatesConnected => {
                conninfo.0.contains_key(&(tr_before.fedid, tr_after.fedid))
            }
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
        conninfo: &ConnInfo,
    ) -> Result<Self, String> {
        let mut ogrank2immediatepredecessors = Vec::new();
        for (ogrank, tr) in trace.iter().enumerate() {
            let mut immediate_predecessors = HashSet::new();
            for rule in axioms {
                immediate_predecessors.extend(Self::apply_rule(
                    rule,
                    tr,
                    &trace[..ogrank],
                    &trace[ogrank + 1..],
                    conninfo,
                )?);
            }
            ogrank2immediatepredecessors.push(immediate_predecessors);
        }
        Ok(Self {
            ogrank2immediatepredecessors,
            always_occurring,
        })
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
        conninfo: &ConnInfo,
    ) -> Result<HashSet<OgRank>, String> {
        let mut immediate_predecessors = HashSet::new();
        if let Some(other) = after
            .iter()
            .find(|tr_after| rule.matches(tr_after, tr, conninfo))
        {
            return Result::Err(format!(
                "Observed\n{}\n★ {}\n{}\nTracepoint:\n    {:?}\nfollowed by:\n    {}\nwith delay:\n    {:?}\nis a counterexample to the axiom:\n{:?}",
                tracerecords_to_string(before, |_| false), tr, tracerecords_to_string(after, |it| it == other), tr, other, conninfo.0.get(&(tr.fedid, other.fedid)), rule
            ));
        }
        if rule.event == tr.event {
            immediate_predecessors.extend(
                before
                    .iter()
                    .enumerate()
                    .filter(|(_, tr_before)| rule.matches(tr_before, tr, conninfo))
                    .map(|(ogrank, _)| OgRank(ogrank as u32)),
            );
        }
        Ok(immediate_predecessors)
    }
}
