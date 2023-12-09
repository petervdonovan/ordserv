use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Formatter},
    path::Path,
    str::FromStr,
};

use conninfo::{get_nonnegative_microstep, ConnInfo, FedId, Tag, NO_DELAY};

pub mod axioms;
pub mod conninfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
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
}

/// If two events match a rule, then the rule says that there is a precedence relation between them
/// (with the preceding event occurring first in all non-error traces).
// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rule {
    pub event: Predicate,
    pub preceding_event: BinaryRelation,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BinaryRelation {
    TagPlusDelay2FedEquals,
    TagPlusDelay2FedLessThan,
    TagPlusDelay2FedLessThanOrEqual,
    TagGreaterThanOrEqual,
    TagStrictPlusDelayFromAllImmUpstreamFedsLessThan,
    TagStrictPlusDelayFromSomeImmUpstreamFedGreaterThanOrEquals,
    TagPlusDelayFromAllImmUpstreamFedsLessThan,
    FederateEquals,
    FederateZeroDelayDirectlyUpstreamOf,
    IsFirst(Box<BinaryRelation>),
    And(Box<[BinaryRelation]>),
    Or(Box<[BinaryRelation]>),
    Unary(Box<Predicate>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Predicate {
    TagNonzero,
    TagFinite,
    EventIs(EventKind),
    IsFirst(Box<Predicate>),
    And(Box<[Predicate]>),
    Or(Box<[Predicate]>),
    BoundBinary(Box<(Event, BinaryRelation)>),
}

impl Display for EventKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EventKind::RecvFedId => write!(f, "Receiving FED_ID"),
            EventKind::SendAck => write!(f, "Sending ACK"),
            EventKind::SendTimestamp => write!(f, "Sending TIMESTAMP"),
            EventKind::RecvTimestamp => write!(f, "Receiving TIMESTAMP"),
            EventKind::RecvNet => write!(f, "Receiving NET"),
            EventKind::SendPortAbs => write!(f, "Sending PORT_ABS"),
            EventKind::RecvPortAbs => write!(f, "Receiving PORT_ABS"),
            EventKind::SendPtag => write!(f, "Sending PTAG"),
            EventKind::SendTaggedMsg => write!(f, "Sending TAGGED_MSG"),
            EventKind::RecvTaggedMsg => write!(f, "Receiving TAGGED_MSG"),
            EventKind::SendTag => write!(f, "Sending TAG"),
            EventKind::RecvStopReq => write!(f, "Receiving STOP_REQ"),
            EventKind::SendStopReq => write!(f, "Sending STOP_REQ"),
            EventKind::RecvStopReqRep => write!(f, "Receiving STOP_REQ_REP"),
            EventKind::SendStopGrn => write!(f, "Sending STOP_GRN"),
            EventKind::RecvLtc => write!(f, "Receiving LTC"),
        }
    }
}

impl FromStr for EventKind {
    type Err = ();
    fn from_str(event: &str) -> Result<Self, Self::Err> {
        match event.trim() {
            "Receiving FED_ID" => Ok(EventKind::RecvFedId),
            "Sending ACK" => Ok(EventKind::SendAck),
            "Sending TIMESTAMP" => Ok(EventKind::SendTimestamp),
            "Receiving TIMESTAMP" => Ok(EventKind::RecvTimestamp),
            "Receiving NET" => Ok(EventKind::RecvNet),
            "Sending PTAG" => Ok(EventKind::SendPtag),
            "Sending TAGGED_MSG" => Ok(EventKind::SendTaggedMsg),
            "Receiving TAGGED_MSG" => Ok(EventKind::RecvTaggedMsg),
            "Sending TAG" => Ok(EventKind::SendTag),
            "Receiving STOP_REQ" => Ok(EventKind::RecvStopReq),
            "Sending STOP_REQ" => Ok(EventKind::SendStopReq),
            "Receiving STOP_REQ_REP" => Ok(EventKind::RecvStopReqRep),
            "Sending STOP_GRN" => Ok(EventKind::SendStopGrn),
            "Receiving LTC" => Ok(EventKind::RecvLtc),
            "Sending PORT_ABS" => Ok(EventKind::SendPortAbs),
            "Receiving PORT_ABS" => Ok(EventKind::RecvPortAbs),
            _ => Err(()),
        }
    }
}

impl Display for Rule {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ≺ {}", self.preceding_event, self.event)
    }
}

impl Display for Predicate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Predicate::TagNonzero => write!(f, "Tag ≠ 0"),
            Predicate::TagFinite => write!(f, "Tag finite"),
            Predicate::EventIs(event) => write!(f, "{}", event),
            Predicate::IsFirst(relation) => write!(f, "(FIRST {})", relation),
            Predicate::And(relations) => {
                write!(f, "({})", relations[0])?;
                for relation in &relations[1..] {
                    write!(f, " ∧ {}", relation)?;
                }
                write!(f, ")")?;
                Ok(())
            }
            Predicate::Or(relations) => {
                write!(f, "({}", relations[0])?;
                for relation in &relations[1..] {
                    write!(f, " ∨ {}", relation)?;
                }
                write!(f, ")")?;
                Ok(())
            }
            Predicate::BoundBinary(bound) => write!(f, "(for e = {}, {})", bound.0, bound.1),
        }
    }
}

impl Display for BinaryRelation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryRelation::TagPlusDelay2FedEquals => write!(f, "Tag + Delay = Tag"),
            BinaryRelation::TagPlusDelay2FedLessThan => write!(f, "Tag + Delay < Tag"),
            BinaryRelation::TagPlusDelay2FedLessThanOrEqual => write!(f, "Tag + Delay ≤ Tag"),
            BinaryRelation::TagStrictPlusDelayFromAllImmUpstreamFedsLessThan => {
                write!(f, "Tag strict+ All Delays < Tag")
            }
            BinaryRelation::TagStrictPlusDelayFromSomeImmUpstreamFedGreaterThanOrEquals => {
                write!(f, "Tag strict+ Some Delay ≥ Tag")
            }
            BinaryRelation::TagPlusDelayFromAllImmUpstreamFedsLessThan => {
                write!(f, "Tag + All Delays < Tag")
            }
            BinaryRelation::TagGreaterThanOrEqual => write!(f, "Tag ≥ Tag"),
            BinaryRelation::FederateEquals => write!(f, "Federate = Federate"),
            BinaryRelation::FederateZeroDelayDirectlyUpstreamOf => {
                write!(f, "Federate has zero delay directly upstream of")
            }
            BinaryRelation::IsFirst(relation) => write!(f, "(FIRST {})", relation),
            BinaryRelation::And(relations) => {
                write!(f, "({})", relations[0])?;
                for relation in &relations[1..] {
                    write!(f, " ∧ {}", relation)?;
                }
                write!(f, ")")?;
                Ok(())
            }
            BinaryRelation::Or(relations) => {
                write!(f, "({}", relations[0])?;
                for relation in &relations[1..] {
                    write!(f, " ∨ {}", relation)?;
                }
                write!(f, ")")?;
                Ok(())
            }
            BinaryRelation::Unary(p) => write!(f, "{}", p),
        }
    }
}

pub fn preceding_permutables_by_ogrank_from_dir(
    dir: &Path,
) -> Result<Vec<HashSet<OgRank>>, String> {
    let rti_csv = dir.join("rti.csv");
    let conninfo = dir.join("conninfo.txt");
    let conninfo = ConnInfo::from_str(&std::fs::read_to_string(conninfo).unwrap()).unwrap();
    let axioms = axioms::axioms();
    let always_occurring: HashSet<_> = (0..lf_trace_reader::trace_by_physical_time(&rti_csv).len())
        .map(|ogrank| OgRank(ogrank as u32))
        .collect();
    let trace = elaborated_from_trace_records(
        lf_trace_reader::trace_by_physical_time(&rti_csv),
        &axioms,
        &conninfo,
    );
    println!("{}", tracerecords_to_string(&trace[..], true, |_| false));
    preceding_permutables_by_ogrank(&trace, &axioms, always_occurring, &conninfo)
}

pub fn preceding_permutables_by_ogrank(
    trace: &[Event],
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventPerTraceUniqueId {
    Og(OgRank),
    First(Predicate),
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Event {
    pub event: EventKind,
    pub tag: Tag,
    pub fedid: FedId,
    pub unique_id: EventPerTraceUniqueId,
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} @ {:?} (src={})",
            self.event, self.tag, self.fedid, self.unique_id
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventParseError {
    InvalidEventKind,
    InvalidTag,
    InvalidFedId,
    InvalidUniqueId,
}
impl FromStr for Event {
    type Err = EventParseError;
    /// Inverse of Display::fmt
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.split(" @ ");
        let event_and_tag = s.next().unwrap();
        let (event, tag) = event_and_tag.split_at(event_and_tag.find('(').unwrap());
        let event = EventKind::from_str(event).map_err(|_| EventParseError::InvalidEventKind)?;
        let tag = Tag::from_str(tag).map_err(|_| EventParseError::InvalidTag)?;
        let mut s = s.next().unwrap().split(' ');
        let fedid =
            FedId::from_str(s.next().unwrap()).map_err(|_| EventParseError::InvalidFedId)?;
        let mut s = s.next().unwrap().split("(src=");
        s.next().unwrap(); // should be error, not panic, ditto everywhere else
        let mut s = s.next().unwrap().split(')');
        let unique_id = EventPerTraceUniqueId::from_str(s.next().unwrap())
            .map_err(|_| EventParseError::InvalidUniqueId)?;
        Ok(Self {
            event,
            tag,
            fedid,
            unique_id,
        })
    }
}

impl FromStr for FedId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        Ok(Self(
            trimmed["FedId(".len()..trimmed.len() - 1]
                .parse()
                .map_err(|_| format!("Invalid FED_ID: {}", s))?,
        ))
    }
}

impl Display for EventPerTraceUniqueId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EventPerTraceUniqueId::Og(ogr) => write!(f, "{}", ogr.0),
            EventPerTraceUniqueId::First(rel) => write!(f, "{}", rel),
        }
    }
}

impl FromStr for EventPerTraceUniqueId {
    type Err = ();
    /// Inverse of Display::fmt
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(ogr) = s.parse::<u32>() {
            return Ok(Self::Og(OgRank(ogr)));
        }
        // Ok(Self::First(Predicate::from_str(s).map_err(|_| ())?))
        Result::Err(())
    }
}

fn tracerecords_to_string(
    trace: &[Event],
    numbering: bool,
    put_marker_if: impl Fn(&Event) -> bool,
) -> String {
    trace
        .iter()
        .enumerate()
        .map(|(ogr, it)| (ogr, it.to_string(), put_marker_if(it)))
        .fold(String::new(), |mut s, (ogr, other, matches)| {
            if !s.is_empty() {
                s += "\n";
            }
            if numbering {
                s += &ogr.to_string();
                s += " ";
            }
            if matches {
                s += "▶ ";
            } else {
                s += "  ";
            }
            s += &other;
            s
        })
}

impl Event {
    pub fn from_lf_trace_record(
        lf_trace_record: &lf_trace_reader::TraceRecord,
        ogrank: OgRank,
    ) -> Self {
        let event = EventKind::from_str(&lf_trace_record.event)
            .unwrap_or_else(|_| panic!("Unrecognized event: {}", lf_trace_record.event));
        let tag = Tag(
            lf_trace_record.elapsed_logical_time,
            get_nonnegative_microstep(lf_trace_record.microstep),
        );
        let source = lf_trace_record.destination;
        Self {
            event,
            tag,
            fedid: FedId(source),
            unique_id: EventPerTraceUniqueId::Og(OgRank(ogrank.0)),
        }
    }
}

pub fn elaborated_from_trace_records(
    trace_records: Vec<lf_trace_reader::TraceRecord>,
    axioms: &[Rule],
    conninfo: &ConnInfo,
) -> Vec<Event> {
    let concretes = trace_records
        .iter()
        .map(|record| Event::from_lf_trace_record(record, OgRank(0)))
        .collect::<Vec<_>>();
    let mut firsts = Vec::<Vec<Event>>::new();
    for _ in 0..concretes.len() {
        firsts.push(vec![]);
    }
    for p in get_first_predicates(axioms, &concretes, conninfo) {
        for (ogr, record) in concretes.iter().enumerate() {
            if p.holds(record, &ConnInfo(HashMap::new())) {
                firsts[ogr].push(Event {
                    unique_id: EventPerTraceUniqueId::First(p.clone()),
                    ..record.clone()
                });
                break;
            }
        }
    }
    let mut ret = Vec::new();
    for (ogidx, record) in trace_records.iter().enumerate() {
        for first in firsts.get_mut(ogidx).unwrap().drain(..) {
            ret.push(first);
        }
        ret.push(Event::from_lf_trace_record(record, OgRank(ogidx as u32)));
    }
    ret
}

fn get_first_predicates(
    axioms: &[Rule],
    concretes: &[Event],
    conninfo: &ConnInfo,
) -> HashSet<Predicate> {
    let mut ret = HashSet::new();
    for a in axioms {
        add_first_predicates_recursive(&a.event, &a.preceding_event, concretes, &mut ret, conninfo);
    }
    ret
}

fn add_first_predicates_recursive(
    event: &Predicate,
    brel: &BinaryRelation,
    concretes: &[Event],
    predicates: &mut HashSet<Predicate>,
    conninfo: &ConnInfo,
) {
    match brel {
        BinaryRelation::TagPlusDelay2FedEquals
        | BinaryRelation::TagPlusDelay2FedLessThan
        | BinaryRelation::TagPlusDelay2FedLessThanOrEqual
        | BinaryRelation::TagGreaterThanOrEqual
        | BinaryRelation::FederateEquals
        | BinaryRelation::TagStrictPlusDelayFromAllImmUpstreamFedsLessThan
        | BinaryRelation::TagStrictPlusDelayFromSomeImmUpstreamFedGreaterThanOrEquals
        | BinaryRelation::TagPlusDelayFromAllImmUpstreamFedsLessThan
        | BinaryRelation::FederateZeroDelayDirectlyUpstreamOf => {}
        BinaryRelation::IsFirst(rel) => {
            for e in concretes.iter().filter(|e| event.holds(e, conninfo)) {
                predicates.insert(Predicate::BoundBinary(Box::new((e.clone(), *rel.clone()))));
            }
            add_first_predicates_recursive(event, rel, concretes, predicates, conninfo);
        }
        BinaryRelation::And(rels) | BinaryRelation::Or(rels) => {
            for rel in &**rels {
                add_first_predicates_recursive(event, rel, concretes, predicates, conninfo);
            }
        }
        BinaryRelation::Unary(prel) => {
            add_first_predicates_recursive_from_predicate(prel, predicates)
        }
    }
}

fn add_first_predicates_recursive_from_predicate(
    prel: &Predicate,
    predicates: &mut HashSet<Predicate>,
) {
    match prel {
        Predicate::TagNonzero | Predicate::TagFinite | Predicate::EventIs(_) => {}
        Predicate::IsFirst(prel) => {
            predicates.insert(*prel.clone());
            add_first_predicates_recursive_from_predicate(prel, predicates);
        }
        Predicate::And(prels) | Predicate::Or(prels) => {
            for prel in &**prels {
                add_first_predicates_recursive_from_predicate(prel, predicates);
            }
        }
        Predicate::BoundBinary(_) => {
            panic!("it never makes sense to use a bound binary inside a predicate inside a bound binary in user-facing rules");
        }
    }
}

impl Predicate {
    pub fn holds(&self, e: &Event, conninfo: &ConnInfo) -> bool {
        match self {
            Predicate::TagNonzero => e.tag != Tag(0, 0),
            Predicate::TagFinite => e.tag.0.abs() < 1_000_000_000_000,
            Predicate::EventIs(event) => e.event == *event,
            Predicate::IsFirst(relation) => {
                if let EventPerTraceUniqueId::First(other) = &e.unique_id {
                    other == &**relation // TODO: consider logical equivalence?
                } else {
                    false
                }
            }
            Predicate::And(relations) => relations.iter().all(|rel| rel.holds(e, conninfo)),
            Predicate::Or(relations) => relations.iter().any(|rel| rel.holds(e, conninfo)),
            Predicate::BoundBinary(bound) => bound.1.holds(&bound.0, e, conninfo),
        }
    }
}

impl BinaryRelation {
    pub fn holds(&self, e: &Event, preceding: &Event, conninfo: &ConnInfo) -> bool {
        let compare_before_plus_delay_with_after = |f: fn(&Tag, &Tag) -> bool| {
            f(
                &(preceding.tag
                    + *conninfo
                        .0
                        .get(&(preceding.fedid, e.fedid))
                        .unwrap_or(&NO_DELAY)),
                &e.tag,
            )
        };
        match self {
            BinaryRelation::TagPlusDelay2FedEquals => {
                compare_before_plus_delay_with_after(|a, b| a == b)
            }
            BinaryRelation::TagPlusDelay2FedLessThan => {
                compare_before_plus_delay_with_after(|a, b| a < b)
            }
            BinaryRelation::TagPlusDelay2FedLessThanOrEqual => {
                compare_before_plus_delay_with_after(|a, b| a <= b)
            }
            BinaryRelation::TagGreaterThanOrEqual => {
                compare_before_plus_delay_with_after(|a, b| a >= b)
            }
            BinaryRelation::TagStrictPlusDelayFromAllImmUpstreamFedsLessThan => conninfo
                .0
                .iter()
                .filter(|((src, _), _)| *src == preceding.fedid)
                .all(|(_, delay)| preceding.tag.strict_add(*delay) < e.tag),
            BinaryRelation::TagStrictPlusDelayFromSomeImmUpstreamFedGreaterThanOrEquals => conninfo
                .0
                .iter()
                .filter(|((src, _), _)| *src == preceding.fedid)
                .any(|(_, delay)| preceding.tag.strict_add(*delay) >= e.tag),
            BinaryRelation::TagPlusDelayFromAllImmUpstreamFedsLessThan => conninfo
                .0
                .iter()
                .filter(|((src, _), _)| *src == preceding.fedid)
                .all(|(_, delay)| preceding.tag + *delay < e.tag),
            BinaryRelation::FederateEquals => e.fedid == preceding.fedid,
            BinaryRelation::FederateZeroDelayDirectlyUpstreamOf => conninfo
                .0
                .get(&(e.fedid, preceding.fedid))
                .map(|delay| *delay == NO_DELAY)
                .unwrap_or(false),
            BinaryRelation::IsFirst(r) => {
                if let EventPerTraceUniqueId::First(other) = &e.unique_id {
                    other == &Predicate::BoundBinary(Box::new((e.clone(), *r.clone())))
                // maybe not the most efficient
                // TODO: consider logical equivalence?
                } else {
                    false
                }
            }
            BinaryRelation::And(relations) => relations
                .iter()
                .all(|rel| rel.holds(e, preceding, conninfo)),
            BinaryRelation::Or(relations) => relations
                .iter()
                .any(|rel| rel.holds(e, preceding, conninfo)),
            BinaryRelation::Unary(p) => p.holds(preceding, conninfo),
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
        trace: &[Event],
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
        Self::add_precedences_for_firsts(&mut ogrank2immediatepredecessors, trace, conninfo);
        Ok(Self {
            ogrank2immediatepredecessors,
            always_occurring,
        })
    }
    /// The first appears before all that which it matches, and all that appears before all that
    /// which it matches, excluding itself, appears before it.
    fn add_precedences_for_firsts(
        ogrank2immediatepredecessors: &mut [HashSet<OgRank>],
        trace: &[Event],
        conninfo: &ConnInfo,
    ) {
        for (ogrank, tr) in trace.iter().enumerate() {
            if let EventPerTraceUniqueId::First(ref rel) = &tr.unique_id {
                let mut running_intersection: Option<HashSet<OgRank>> = None;
                for (preds, tr) in ogrank2immediatepredecessors[ogrank + 1..]
                    .iter_mut()
                    .zip(&trace[ogrank + 1..])
                {
                    if let Some(running_intersection) = &mut running_intersection {
                        running_intersection.retain(|ogr| preds.contains(ogr));
                    } else {
                        running_intersection = Some(preds.clone());
                    }
                    if rel.holds(tr, conninfo) {
                        preds.insert(OgRank(ogrank as u32));
                    }
                }
                if let Some(running_intersection) = running_intersection {
                    ogrank2immediatepredecessors[ogrank].extend(running_intersection.into_iter());
                }
            }
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
        e: &Event,
        before: &[Event],
        after: &[Event],
        conninfo: &ConnInfo,
    ) -> Result<HashSet<OgRank>, String> {
        if !rule.event.holds(e, conninfo) {
            return Ok(HashSet::new());
        }
        let p = Predicate::BoundBinary(Box::new((e.clone(), rule.preceding_event.clone())));
        if let Some(other) = after.iter().find(|tr_after| p.holds(tr_after, conninfo)) {
            return Result::Err(format!(
                "Observed\n{}\n★ {}\n{}\nTracepoint:\n    {}\nfollowed by:\n    {}\nwith delay:\n    {}\nis a counterexample to the axiom:\n{}",
                tracerecords_to_string(before, false, |_| false), e, tracerecords_to_string(after, false, |it| it == other), e, other, conninfo.0.get(&(e.fedid, other.fedid)).map(|it| it.to_string()).unwrap_or("∞".to_string()), rule
            ));
        }
        Ok(before
            .iter()
            .enumerate()
            .filter(|(_, tr_before)| p.holds(tr_before, conninfo))
            .map(|(ogr, _)| OgRank(ogr as u32))
            .collect())
    }
    // fn apply_rule(
    //     rule: &Rule,
    //     tr: &Event,
    //     before: &[Event],
    //     after: &[Event],
    //     conninfo: &ConnInfo,
    // ) -> Result<HashSet<OgRank>, String> {
    //     let mut immediate_predecessors = HashSet::new();
    //     if let Some(other) = after
    //         .iter()
    //         .find(|tr_after| rule.matches(tr_after, tr, conninfo))
    //     {
    //         return Result::Err(format!(
    //             "Observed\n{}\n★ {}\n{}\nTracepoint:\n    {}\nfollowed by:\n    {}\nwith delay:\n    {:?}\nis a counterexample to the axiom:\n{:?}",
    //             tracerecords_to_string(before, |_| false), tr, tracerecords_to_string(after, |it| it == other), tr, other, conninfo.0.get(&(tr.fedid, other.fedid)), rule
    //         ));
    //     }
    //     if rule.event == tr.event {
    //         immediate_predecessors.extend(
    //             before
    //                 .iter()
    //                 .enumerate()
    //                 .filter(|(_, tr_before)| rule.matches(tr_before, tr, conninfo))
    //                 .map(|(ogrank, _)| OgRank(ogrank as u32)),
    //         );
    //     }
    //     Ok(immediate_predecessors)
    // }
}

/// testing module
#[cfg(test)]
mod tests {
    use super::*;

    use crate::{conninfo::Tag, EventKind, Rule};

    use crate::BinaryRelation::{
        And, FederateEquals, FederateZeroDelayDirectlyUpstreamOf, TagPlusDelay2FedEquals,
        TagPlusDelay2FedLessThan, TagPlusDelay2FedLessThanOrEqual,
        TagPlusDelayFromAllImmUpstreamFedsLessThan, Unary,
    };
    use crate::EventKind::*;
    use crate::Predicate::*;
    use crate::{BinaryRelation, Predicate};

    #[test]
    fn rule_test0() {
        let e = Event::from_str("Receiving TAGGED_MSG (0, 1) @ FedId(0) (src=6)").unwrap();
        println!("{}", e);
        let predecessor = Event::from_str("Receiving LTC (0, 0) @ FedId(0) (src=9)").unwrap();
        println!("{}", predecessor);
        let rule = Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(RecvLtc))),
                FederateEquals,
                TagPlusDelayFromAllImmUpstreamFedsLessThan,
            ])),
            event: EventIs(RecvTaggedMsg),
        };
        println!("{}", rule);
        let conninfo = ConnInfo::from_str(
            "1
0 1 0 0
",
        )
        .unwrap();
        assert!(rule.preceding_event.holds(&e, &predecessor, &conninfo));
    }
}
