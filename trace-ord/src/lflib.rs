use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Formatter},
    path::Path,
    str::FromStr,
};

use crate::conninfo::{get_nonnegative_microstep, ConnInfo, Delay, FedId, Tag, NO_DELAY, STARTUP};
use ::serde::{Deserialize, Serialize};
use enum_iterator::Sequence;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Sequence)]
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

pub type Predicate = crate::Predicate<PredicateAtom, BinaryRelationAtom, Event>;
pub type BinaryRelation = crate::BinaryRelation<PredicateAtom, BinaryRelationAtom, Event>;

/// If two events match a rule, then the rule says that there is a precedence relation between them
/// (with the preceding event occurring first in all non-error traces).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rule {
    pub event: Predicate,
    pub preceding_event: BinaryRelation,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Sequence)]
pub enum BinaryRelationAtom {
    LessThan(Term, Term),
    LessThanOrEqual(Term, Term),
    GreaterThanOrEqual(Term, Term),
    GreaterThan(Term, Term),
    Equal(Term, Term),
    FederateEquals,
    FederateZeroDelayDirectlyUpstreamOf,
    FederateDirectlyUpstreamOf,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Sequence)]
pub enum Term {
    Tag,
    TagPlusDelay(DelayTerm),
    TagStrictPlusDelay(DelayTerm),
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Sequence)]
pub enum DelayTerm {
    SmallestDelayBetween,
    SmallestDelayFrom,
    SmallestDelayFromSomeImmUpstreamFed,
    LargestDelayFrom,
    LargestDelayFromSomeImmUpstreamFed,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Sequence)]
pub enum PredicateAtom {
    FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag,
    TagNonzero,
    TagFinite,
    EventIs(EventKind),
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
            Predicate::Atom(PredicateAtom::TagNonzero) => write!(f, "Tag ≠ 0"),
            Predicate::Atom(
                PredicateAtom::FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag,
            ) => {
                write!(f, "Fed has no upstream with delay ≤ Tag")
            }
            Predicate::Atom(PredicateAtom::TagFinite) => write!(f, "Tag finite"),
            Predicate::Atom(PredicateAtom::EventIs(event)) => write!(f, "{}", event),
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
            Predicate::Not(relation) => write!(f, "¬{}", relation),
            Predicate::BoundBinary(bound) => write!(f, "(for e = {}, {})", bound.0, bound.1),
        }
    }
}

impl Display for Term {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Term::Tag => write!(f, "Tag"),
            Term::TagPlusDelay(delay) => write!(f, "Tag + {}", delay),
            Term::TagStrictPlusDelay(delay) => write!(f, "Tag strict+ {}", delay),
        }
    }
}

impl Display for DelayTerm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DelayTerm::SmallestDelayBetween => write!(f, "SmallestDelayBetween"),
            DelayTerm::SmallestDelayFrom => write!(f, "SmallestDelayFromPreceding"),
            DelayTerm::SmallestDelayFromSomeImmUpstreamFed => {
                write!(f, "SmallestDelayFromSomeImmUpstreamFed")
            }
            DelayTerm::LargestDelayFrom => write!(f, "LargestDelayFromPreceding"),
            DelayTerm::LargestDelayFromSomeImmUpstreamFed => {
                write!(f, "LargestDelayFromSomeImmUpstreamFed")
            }
        }
    }
}

impl Display for BinaryRelation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryRelation::Atom(BinaryRelationAtom::FederateEquals) => {
                write!(f, "Federate = Federate")
            }
            BinaryRelation::Atom(BinaryRelationAtom::FederateZeroDelayDirectlyUpstreamOf) => {
                write!(f, "Federate has zero delay directly upstream of")
            }
            BinaryRelation::Atom(BinaryRelationAtom::FederateDirectlyUpstreamOf) => {
                write!(f, "Federate is directly upstream of")
            }
            BinaryRelation::IsFirst(relation) => write!(f, "(FIRST {})", relation),
            BinaryRelation::IsFirstForFederate(relation) => {
                write!(f, "(FedwiseFIRST {})", relation)
            }
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
            BinaryRelation::Atom(BinaryRelationAtom::LessThan(t0, t1)) => {
                write!(f, "{} < {}", t0, t1)
            }
            BinaryRelation::Atom(BinaryRelationAtom::LessThanOrEqual(t0, t1)) => {
                write!(f, "{} ≤ {}", t0, t1)
            }
            BinaryRelation::Atom(BinaryRelationAtom::GreaterThanOrEqual(t0, t1)) => {
                write!(f, "{} ≥ {}", t0, t1)
            }
            BinaryRelation::Atom(BinaryRelationAtom::GreaterThan(t0, t1)) => {
                write!(f, "{} > {}", t0, t1)
            }
            BinaryRelation::Atom(BinaryRelationAtom::Equal(t0, t1)) => write!(f, "{} = {}", t0, t1),
        }
    }
}

impl Term {
    pub fn eval(
        &self,
        fedid: FedId,
        tag: Tag,
        fedids: (FedId, FedId),
        conninfo: &ConnInfo,
    ) -> Option<Tag> {
        match self {
            Term::Tag => Some(tag),
            Term::TagPlusDelay(dt) => Some(tag + dt.eval(fedid, fedids, conninfo)?),
            Term::TagStrictPlusDelay(dt) => {
                Some(tag.strict_plus(dt.eval(fedid, fedids, conninfo)?))
            }
        }
    }
}

impl DelayTerm {
    pub fn eval(&self, fedid: FedId, fedids: (FedId, FedId), conninfo: &ConnInfo) -> Option<Delay> {
        match self {
            DelayTerm::SmallestDelayBetween => conninfo.get(fedids.0, fedids.1).copied(),
            DelayTerm::SmallestDelayFrom => {
                conninfo.delays_out(fedid).map(|(_, delay)| *delay).min()
            }
            DelayTerm::SmallestDelayFromSomeImmUpstreamFed => conninfo
                .min_delays2dest(fedid)
                .map(|(_, delay)| *delay)
                .min(),
            DelayTerm::LargestDelayFrom => {
                conninfo.delays_out(fedid).map(|(_, delay)| *delay).max()
            }
            DelayTerm::LargestDelayFromSomeImmUpstreamFed => conninfo.delays_in(fedid).max(),
        }
    }
}

pub type NUsesAndUnpermutables = (HashMap<Rule, u32>, Vec<HashSet<OgRank>>);

pub fn preceding_permutables_by_ogrank_from_dir(
    dir: &Path,
) -> (Vec<Event>, ConnInfo, Result<NUsesAndUnpermutables, String>) {
    let rti_csv = dir.join("rti.csv");
    let conninfo = dir.join("conninfo.txt");
    let rti_conninfo = &std::fs::read_to_string(conninfo).unwrap();
    // iterate over the files in the directory of the form conninfo_k.txt where k is some number
    // and store all of their contents
    let mut fed_conninfos = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "txt" {
                    let filename = path.file_name().unwrap().to_str().unwrap();
                    if filename.starts_with("conninfo_") {
                        let k = filename["conninfo_".len()..filename.len() - ".txt".len()]
                            .parse::<u32>();
                        if k.is_ok() {
                            fed_conninfos.push(std::fs::read_to_string(path).unwrap());
                        }
                    }
                }
            }
        }
    }
    let conninfo = ConnInfo::from_strs(rti_conninfo, &fed_conninfos);
    let axioms = crate::axioms::axioms();
    let trace = elaborated_from_trace_records(
        lf_trace_reader::trace_by_physical_time(&rti_csv),
        &axioms,
        &conninfo,
    );
    let always_occurring: HashSet<_> = (0..trace.len())
        .map(|ogrank| OgRank(ogrank as u32))
        .collect();
    let permutables = preceding_permutables_by_ogrank(&trace, &axioms, always_occurring, &conninfo);
    (trace, conninfo, permutables)
}

pub fn preceding_permutables_by_ogrank(
    trace: &[Event],
    axioms: &[Rule],
    always_occurring: HashSet<OgRank>,
    conninfo: &ConnInfo,
) -> Result<NUsesAndUnpermutables, String> {
    let (unused, unpermutables) =
        Unpermutables::from_realizable_trace(trace, axioms, always_occurring, conninfo)?;
    Ok((unused, unpermutables.preceding_permutables_by_ogrank()))
}

pub struct Unpermutables {
    pub ogrank2immediatepredecessors: Vec<HashSet<OgRank>>,
    pub always_occurring: HashSet<OgRank>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OgRank(pub u32);
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Event {
    Concrete {
        event: EventKind,
        tag: Tag,
        fedid: FedId,
        ogrank: OgRank,
    },
    First(Predicate),
    FirstForFederate(FedId, Predicate),
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Concrete {
                event,
                tag,
                fedid,
                ogrank,
            } => write!(f, "{} {} @ {:?} (src={})", event, tag, fedid, ogrank.0),
            Event::First(p) => write!(f, "(FIRST {})", p),
            Event::FirstForFederate(fedid, p) => write!(f, "(FedwiseFIRST {} {})", fedid.0, p),
        }
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
        let unique_id = s
            .next()
            .unwrap()
            .parse()
            .map_err(|_| EventParseError::InvalidUniqueId)?;
        Ok(Self::Concrete {
            event,
            tag,
            fedid,
            ogrank: OgRank(unique_id),
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

pub fn tracerecords_to_string(
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
        Self::Concrete {
            event,
            tag,
            fedid: FedId(source),
            ogrank: OgRank(ogrank.0),
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
        .enumerate()
        .map(|(ogr, record)| Event::from_lf_trace_record(record, OgRank(ogr as u32)))
        .collect::<Vec<_>>();
    let mut firsts = Vec::<Vec<Event>>::new();
    for _ in 0..concretes.len() {
        firsts.push(vec![]);
    }
    let (predicates, federate_wise_predicates) = get_first_predicates(axioms, &concretes, conninfo);
    for p in predicates {
        for (ogr, record) in concretes.iter().enumerate() {
            if p.holds(record, conninfo) {
                firsts[ogr].push(Event::First(p.clone()));
                break;
            }
        }
    }
    for p in federate_wise_predicates {
        let mut federates_hit = HashSet::new();
        for (ogr, record) in concretes.iter().enumerate() {
            if let Event::Concrete { fedid, .. } = record {
                if p.holds(record, conninfo) && !federates_hit.contains(fedid) {
                    firsts[ogr].push(Event::FirstForFederate(*fedid, p.clone()));
                    federates_hit.insert(*fedid);
                    if federates_hit.len() == conninfo.n_federates() {
                        break;
                    }
                }
            }
        }
    }
    let mut ret = Vec::new();
    for (ogidx, e) in concretes.into_iter().enumerate() {
        let mut firsts: Vec<_> = firsts.get_mut(ogidx).unwrap().drain(..).collect();
        firsts.sort(); // this is a hack around a subtle quirk by which firsts associated with the same og event can have dependencies between each other that affect the final precedence relation even after it is quotiented out by the firsts. It is troubling that the ordering can matter
        for first in firsts {
            ret.push(first);
        }
        ret.push(e);
    }
    ret
}

fn get_first_predicates(
    axioms: &[Rule],
    concretes: &[Event],
    conninfo: &ConnInfo,
) -> (HashSet<Predicate>, HashSet<Predicate>) {
    let mut ret = (HashSet::new(), HashSet::new());
    for a in axioms {
        add_first_predicates_recursive(
            &a.event,
            &a.preceding_event,
            concretes,
            &mut ret.0,
            &mut ret.1,
            conninfo,
        );
    }
    ret
}

fn add_first_predicates_recursive(
    event: &Predicate,
    brel: &BinaryRelation,
    concretes: &[Event],
    predicates: &mut HashSet<Predicate>,
    federate_wise_predicates: &mut HashSet<Predicate>,
    conninfo: &ConnInfo,
) {
    match brel {
        // BinaryRelation::TagPlusDelay2FedEquals
        // | BinaryRelation::TagPlusDelay2FedLessThan
        // | BinaryRelation::TagPlusDelay2FedLessThanOrEqual
        // | BinaryRelation::TagPlusDelay2FedGreaterThanOrEquals
        // | BinaryRelation::TagPlusLargestDelayLessThan
        // | BinaryRelation::TagPlusLargestDelayLessThanOrEqual
        // | BinaryRelation::TagPlusLargestDelayGreaterThanOrEqual
        // | BinaryRelation::TagGreaterThanOrEqual
        // | BinaryRelation::TagEquals
        // | BinaryRelation::TagLessThan
        // | BinaryRelation::TagLessThanOrEqual
        // | BinaryRelation::TagStrictPlusDelay2FedLessThan
        // | BinaryRelation::TagStrictPlusDelayFromSomeImmUpstreamFedGreaterThanOrEquals
        // | BinaryRelation::TagPlusDelayToAllImmDownstreamFedsLessThan
        BinaryRelation::Atom(BinaryRelationAtom::FederateEquals)
        | BinaryRelation::Atom(BinaryRelationAtom::FederateZeroDelayDirectlyUpstreamOf)
        | BinaryRelation::Atom(BinaryRelationAtom::FederateDirectlyUpstreamOf) => {}
        BinaryRelation::IsFirst(rel) => {
            for e in concretes.iter().filter(|e| event.holds(e, conninfo)) {
                predicates.insert(Predicate::BoundBinary(Box::new((e.clone(), *rel.clone()))));
            }
            add_first_predicates_recursive(
                event,
                rel,
                concretes,
                predicates,
                federate_wise_predicates,
                conninfo,
            );
        }
        BinaryRelation::IsFirstForFederate(rel) => {
            for e in concretes.iter().filter(|e| event.holds(e, conninfo)) {
                federate_wise_predicates
                    .insert(Predicate::BoundBinary(Box::new((e.clone(), *rel.clone()))));
            }
            add_first_predicates_recursive(
                event,
                rel,
                concretes,
                predicates,
                federate_wise_predicates,
                conninfo,
            );
        }
        BinaryRelation::And(rels) | BinaryRelation::Or(rels) => {
            for rel in &**rels {
                add_first_predicates_recursive(
                    event,
                    rel,
                    concretes,
                    predicates,
                    federate_wise_predicates,
                    conninfo,
                );
            }
        }
        BinaryRelation::Unary(prel) => {
            add_first_predicates_recursive_from_predicate(prel, predicates)
        }
        BinaryRelation::Atom(BinaryRelationAtom::LessThan(_, _))
        | BinaryRelation::Atom(BinaryRelationAtom::LessThanOrEqual(_, _))
        | BinaryRelation::Atom(BinaryRelationAtom::GreaterThanOrEqual(_, _))
        | BinaryRelation::Atom(BinaryRelationAtom::GreaterThan(_, _))
        | BinaryRelation::Atom(BinaryRelationAtom::Equal(_, _)) => {
            // do nothing; we have recursed below the level of predicates
        }
    }
}

fn add_first_predicates_recursive_from_predicate(
    prel: &Predicate,
    predicates: &mut HashSet<Predicate>,
) {
    match prel {
        Predicate::Atom(PredicateAtom::TagNonzero)
        | Predicate::Atom(PredicateAtom::TagFinite)
        | Predicate::Atom(PredicateAtom::EventIs(_))
        | Predicate::Atom(PredicateAtom::FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag) => {}
        Predicate::IsFirst(prel) => {
            predicates.insert(*prel.clone());
            add_first_predicates_recursive_from_predicate(prel, predicates);
        }
        Predicate::And(prels) | Predicate::Or(prels) => {
            for prel in &**prels {
                add_first_predicates_recursive_from_predicate(prel, predicates);
            }
        }
        Predicate::Not(prel) => {
            add_first_predicates_recursive_from_predicate(prel, predicates);
        }
        Predicate::BoundBinary(_) => {
            panic!("it never makes sense to use a bound binary inside a predicate inside a bound binary in user-facing rules");
        }
    }
}

impl Predicate {
    pub fn holds(&self, e: &Event, conninfo: &ConnInfo) -> bool {
        match self {
            Predicate::Atom(PredicateAtom::TagNonzero) => {
                if let Event::Concrete { tag, .. } = e {
                    tag != &Tag(0, 0)
                } else {
                    false
                }
            }
            Predicate::Atom(
                PredicateAtom::FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag,
            ) => {
                if let Event::Concrete { fedid, tag, .. } = e {
                    conninfo
                        .min_delays2dest(*fedid)
                        .all(|(_, delay)| STARTUP + *delay > *tag)
                } else {
                    false
                }
            }
            Predicate::Atom(PredicateAtom::TagFinite) => {
                if let Event::Concrete { tag, .. } = e {
                    tag.0.abs() < 1_000_000_000_000
                } else {
                    false
                }
            }
            Predicate::Atom(PredicateAtom::EventIs(event)) => {
                if let Event::Concrete { event: other, .. } = e {
                    other == event
                } else {
                    false
                }
            }
            Predicate::IsFirst(relation) => {
                if let Event::First(other) = &e {
                    other == &**relation // TODO: consider logical equivalence?
                } else {
                    false
                }
            }
            Predicate::And(relations) => relations.iter().all(|rel| rel.holds(e, conninfo)),
            Predicate::Or(relations) => relations.iter().any(|rel| rel.holds(e, conninfo)),
            Predicate::Not(relation) => !relation.holds(e, conninfo),
            Predicate::BoundBinary(bound) => bound.1.holds(&bound.0, e, conninfo),
        }
    }
}

impl BinaryRelation {
    pub fn holds(&self, e: &Event, preceding: &Event, conninfo: &ConnInfo) -> bool {
        let neither_first = |f: fn(&Tag, &Tag, &FedId, &FedId, Option<&Delay>) -> bool| {
            if let (
                Event::Concrete {
                    tag: ptag,
                    fedid: pfedid,
                    ..
                },
                Event::Concrete { tag, fedid, .. },
            ) = (preceding, e)
            {
                f(ptag, tag, pfedid, fedid, conninfo.get(*pfedid, *fedid))
            } else {
                false
            }
        };
        fn evaluate(
            f: fn(&Tag, &Tag) -> bool,
            t0: &Term,
            t1: &Term,
            e: &Event,
            preceding: &Event,
            conninfo: &ConnInfo,
        ) -> bool {
            if let (
                Event::Concrete {
                    tag: ptag,
                    fedid: pfedid,
                    ..
                },
                Event::Concrete { tag, fedid, .. },
            ) = (preceding, e)
            {
                let t0 = t0.eval(*pfedid, *ptag, (*pfedid, *fedid), conninfo);
                let t1 = t1.eval(*fedid, *tag, (*pfedid, *fedid), conninfo);
                if let (Some(t0), Some(t1)) = (t0, t1) {
                    f(&t0, &t1)
                } else {
                    false
                }
            } else {
                false
            }
        }
        match self {
            BinaryRelation::Atom(BinaryRelationAtom::FederateEquals) => {
                neither_first(|_, _, a, b, _| a == b)
            }
            BinaryRelation::Atom(BinaryRelationAtom::FederateZeroDelayDirectlyUpstreamOf) => {
                neither_first(|_, _, _pfed, _fed, delay| delay == Some(&NO_DELAY))
            }
            BinaryRelation::Atom(BinaryRelationAtom::FederateDirectlyUpstreamOf) => {
                neither_first(|_, _, _pfed, _fed, delay| delay.is_some())
            }
            BinaryRelation::IsFirst(r) => {
                if let Event::First(other) = &preceding {
                    other == &Predicate::BoundBinary(Box::new((e.clone(), *r.clone())))
                // maybe not the most efficient
                // TODO: consider logical equivalence?
                } else {
                    false
                }
            }
            BinaryRelation::IsFirstForFederate(r) => {
                if let Event::FirstForFederate(_, other_r) = &preceding {
                    other_r == &Predicate::BoundBinary(Box::new((e.clone(), *r.clone())))
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
            BinaryRelation::Atom(BinaryRelationAtom::LessThan(t0, t1)) => {
                evaluate(Tag::lt, t0, t1, e, preceding, conninfo)
            }
            BinaryRelation::Atom(BinaryRelationAtom::LessThanOrEqual(t0, t1)) => {
                evaluate(Tag::le, t0, t1, e, preceding, conninfo)
            }
            BinaryRelation::Atom(BinaryRelationAtom::GreaterThanOrEqual(t0, t1)) => {
                evaluate(Tag::ge, t0, t1, e, preceding, conninfo)
            }
            BinaryRelation::Atom(BinaryRelationAtom::GreaterThan(t0, t1)) => {
                evaluate(Tag::gt, t0, t1, e, preceding, conninfo)
            }
            BinaryRelation::Atom(BinaryRelationAtom::Equal(t0, t1)) => {
                evaluate(Tag::eq, t0, t1, e, preceding, conninfo)
            }
        }
    }
}

impl OgRank {
    pub fn idx(&self) -> usize {
        self.0 as usize
    }
}

impl Unpermutables {
    pub fn from_realizable_trace(
        trace: &[Event],
        axioms: &[Rule],
        always_occurring: HashSet<OgRank>,
        conninfo: &ConnInfo,
    ) -> Result<(HashMap<Rule, u32>, Self), String> {
        let mut n_uses_by_axiom: HashMap<Rule, u32> =
            axioms.iter().cloned().map(|it| (it, 0)).collect();
        let mut ogrank2immediatepredecessors = Vec::new();
        for (ogrank, tr) in trace.iter().enumerate() {
            let mut immediate_predecessors = HashSet::new();
            for rule in axioms {
                let preds =
                    Self::apply_rule(rule, tr, &trace[..ogrank], &trace[ogrank + 1..], conninfo)?;
                if !preds.is_empty() {
                    *n_uses_by_axiom.get_mut(rule).unwrap() += preds.len() as u32;
                }
                immediate_predecessors.extend(preds);
            }
            ogrank2immediatepredecessors.push(immediate_predecessors);
        }
        Self::add_precedences_for_firsts(&mut ogrank2immediatepredecessors, trace, conninfo);
        Ok((
            n_uses_by_axiom,
            Self {
                ogrank2immediatepredecessors,
                always_occurring,
            },
        ))
    }
    /// The first appears before all that which it matches, and all that appears before all that
    /// which it matches, excluding itself, appears before it.
    fn add_precedences_for_firsts(
        ogrank2immediatepredecessors: &mut [HashSet<OgRank>],
        trace: &[Event],
        conninfo: &ConnInfo,
    ) {
        for (ogrank, tr) in trace.iter().enumerate() {
            if let Event::First(ref rel) | Event::FirstForFederate(_, ref rel) = &tr {
                let fedid = if let Event::FirstForFederate(fedid, _) = &tr {
                    Some(*fedid)
                } else {
                    None
                };
                // println!("DEBUG: {}", rel);
                let mut running_before_intersection: Option<HashSet<OgRank>> = None;
                let mut n_matches = 0;
                let mut first_match = None;
                for ((preds, ogr), _tr) in ogrank2immediatepredecessors[ogrank + 1..]
                    .iter_mut()
                    .zip(ogrank + 1..)
                    .zip(&trace[ogrank + 1..])
                    .filter(|(_, tr)| {
                        // println!("DEBUG: {}\n    {},    {}", tr, rel, rel.holds(tr, conninfo));
                        rel.holds(tr, conninfo)
                            && !matches!(tr, Event::First(_))
                            && !matches!(tr, Event::FirstForFederate(_, _))
                            && if let (Event::Concrete { fedid: other, .. }, Some(fedid)) =
                                (tr, fedid)
                            {
                                *other == fedid
                            } else {
                                true
                            }
                    })
                {
                    n_matches += 1;
                    if n_matches == 1 {
                        first_match = Some(ogr);
                    }
                    if let Some(running_intersection) = &mut running_before_intersection {
                        running_intersection.retain(|ogr| preds.contains(ogr));
                    } else {
                        running_before_intersection = Some(preds.clone());
                    }
                    preds.insert(OgRank(ogrank as u32));
                }
                if let Some(running_intersection) = running_before_intersection {
                    ogrank2immediatepredecessors[ogrank].extend(running_intersection.into_iter());
                }
                if n_matches == 1 {
                    for after in ogrank2immediatepredecessors[first_match.unwrap() + 1..]
                        .iter_mut()
                        .filter(|ogrs| ogrs.contains(&OgRank(ogrank as u32)))
                    {
                        after.insert(OgRank(first_match.unwrap() as u32));
                    }
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
                for ipred in immediate_predecessors
                    .iter()
                    .filter(|ogrank| self.always_occurring.contains(ogrank) && **ogrank != ipred0)
                {
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
                "Observed\n{}\n★ {}\n{}\nTracepoint:\n    {}\nfollowed by:\n    {}\nis a counterexample to the axiom:\n{}",
                tracerecords_to_string(before, false, |_| false), e, tracerecords_to_string(after, false, |it| it == other), e, other, rule
            ));
        }
        Ok(before
            .iter()
            .enumerate()
            .filter(|(_, tr_before)| p.holds(tr_before, conninfo))
            .map(|(ogr, _)| OgRank(ogr as u32))
            .collect())
    }
}

fn check_rule(
    before: &[Event],
    e: Event,
    after: &[Event],
    rule: &Rule,
    conninfo: &ConnInfo,
) -> Result<(), String> {
    if !rule.event.holds(&e, conninfo) {
        return Ok(());
    }
    let p = Predicate::BoundBinary(Box::new((e.clone(), rule.preceding_event.clone())));
    if let Some(other) = after.iter().find(|tr_after| p.holds(tr_after, conninfo)) {
        return Result::Err(format!(
                "Observed\n{}\n★ {}\n{}\nTracepoint:\n    {}\nfollowed by:\n    {}\nis a counterexample to the axiom:\n{}",
                tracerecords_to_string(before, false, |_| false), e, tracerecords_to_string(after, false, |it| it == other), e, other, rule
            ));
    }
    Ok(())
}

impl Rule {
    pub fn check(&self, trace: &[Event], conninfo: &ConnInfo) -> Result<(), String> {
        for (ogrank, e) in trace.iter().enumerate() {
            check_rule(
                &trace[..ogrank],
                e.clone(),
                &trace[ogrank + 1..],
                self,
                conninfo,
            )?;
        }
        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     use crate::{conninfo::Tag, EventKind, Rule};

//     use crate::BinaryRelation::{
//         And, FederateEquals, FederateZeroDelayDirectlyUpstreamOf, TagPlusDelay2FedEquals,
//         TagPlusDelay2FedLessThan, TagPlusDelay2FedLessThanOrEqual,
//         TagPlusDelayToAllImmDownstreamFedsLessThan, Unary,
//     };
//     use crate::EventKind::*;
//     use crate::Predicate::*;
//     use crate::{BinaryRelation, Predicate};

//     //     #[test]
//     //     fn rule_test0() {
//     //         let e = Event::from_str("Receiving TAGGED_MSG (0, 1) @ FedId(0) (src=6)").unwrap();
//     //         println!("{}", e);
//     //         let predecessor = Event::from_str("Receiving LTC (0, 0) @ FedId(0) (src=9)").unwrap();
//     //         println!("{}", predecessor);
//     //         let rule = Rule {
//     //             preceding_event: And(Box::new([
//     //                 Unary(Box::new(EventIs(RecvLtc))),
//     //                 FederateEquals,
//     //                 TagPlusDelayToAllImmDowntreamFedsLessThan,
//     //             ])),
//     //             event: EventIs(RecvTaggedMsg),
//     //         };
//     //         println!("{}", rule);
//     //         let conninfo = ConnInfo::from_str(
//     //             "1
//     // 0 1 0 0
//     // ",
//     //         )
//     //         .unwrap();
//     //         assert!(rule.preceding_event.holds(&e, &predecessor, &conninfo));
//     //     }
// }
