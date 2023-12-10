use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Formatter},
    path::Path,
    str::FromStr,
};

use conninfo::{get_nonnegative_microstep, ConnInfo, Delay, FedId, Tag, NO_DELAY};

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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    TagStrictPlusDelay2FedLessThan,
    TagPlusDelay2FedGreaterThanOrEquals,
    TagStrictPlusDelayFromSomeImmUpstreamFedGreaterThanOrEquals,
    TagPlusDelayToAllImmDowntreamFedsLessThan,
    TagLessThan,
    TagLessThanOrEqual,
    TagEquals,
    FederateEquals,
    FederateZeroDelayDirectlyUpstreamOf,
    FederateDirectlyUpstreamOf,
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
            BinaryRelation::TagPlusDelay2FedGreaterThanOrEquals => {
                write!(f, "Tag + Delay ≥ Tag")
            }
            BinaryRelation::TagStrictPlusDelay2FedLessThan => {
                write!(f, "Tag strict+ All Delays < Tag")
            }
            BinaryRelation::TagStrictPlusDelayFromSomeImmUpstreamFedGreaterThanOrEquals => {
                write!(f, "Tag strict+ Some Delay ≥ Tag")
            }
            BinaryRelation::TagPlusDelayToAllImmDowntreamFedsLessThan => {
                write!(f, "Tag + All Delays < Tag")
            }
            BinaryRelation::TagGreaterThanOrEqual => write!(f, "Tag ≥ Tag"),
            BinaryRelation::TagEquals => write!(f, "Tag = Tag"),
            BinaryRelation::TagLessThan => write!(f, "Tag < Tag"),
            BinaryRelation::TagLessThanOrEqual => write!(f, "Tag ≤ Tag"),
            BinaryRelation::FederateEquals => write!(f, "Federate = Federate"),
            BinaryRelation::FederateZeroDelayDirectlyUpstreamOf => {
                write!(f, "Federate has zero delay directly upstream of")
            }
            BinaryRelation::FederateDirectlyUpstreamOf => {
                write!(f, "Federate is directly upstream of")
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
) -> (
    Vec<Event>,
    Result<(HashSet<Rule>, Vec<HashSet<OgRank>>), String>,
) {
    let rti_csv = dir.join("rti.csv");
    let conninfo = dir.join("conninfo.txt");
    let conninfo = ConnInfo::from_str(&std::fs::read_to_string(conninfo).unwrap()).unwrap();
    let axioms = axioms::axioms();
    let trace = elaborated_from_trace_records(
        lf_trace_reader::trace_by_physical_time(&rti_csv),
        &axioms,
        &conninfo,
    );
    let always_occurring: HashSet<_> = (0..trace.len())
        .map(|ogrank| OgRank(ogrank as u32))
        .collect();
    let permutables = preceding_permutables_by_ogrank(&trace, &axioms, always_occurring, &conninfo);
    // println!("{}", tracerecords_to_string(&trace[..], true, |_| false));
    (trace, permutables)
}

pub fn preceding_permutables_by_ogrank(
    trace: &[Event],
    axioms: &[Rule],
    always_occurring: HashSet<OgRank>,
    conninfo: &ConnInfo,
) -> Result<(HashSet<Rule>, Vec<HashSet<OgRank>>), String> {
    let (unused, unpermutables) =
        Unpermutables::from_realizable_trace(trace, axioms, always_occurring, conninfo)?;
    Ok((unused, unpermutables.preceding_permutables_by_ogrank()))
}

pub struct Unpermutables {
    pub ogrank2immediatepredecessors: Vec<HashSet<OgRank>>,
    pub always_occurring: HashSet<OgRank>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OgRank(pub u32);
// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub enum EventPerTraceUniqueId {
//     Og(OgRank),
//     First(Predicate),
// }
// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub struct Event {
//     pub event: EventKind,
//     pub tag: Tag,
//     pub fedid: FedId,
//     pub unique_id: EventPerTraceUniqueId,
// }
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Event {
    Concrete {
        event: EventKind,
        tag: Tag,
        fedid: FedId,
        ogrank: OgRank,
    },
    First(Predicate),
}

// impl Display for Event {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "{} {} @ {:?} (src={})",
//             self.event, self.tag, self.fedid, self.unique_id
//         )
//     }
// }
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

// impl Display for EventPerTraceUniqueId {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         match self {
//             EventPerTraceUniqueId::Og(ogr) => write!(f, "{}", ogr.0),
//             EventPerTraceUniqueId::First(rel) => write!(f, "{}", rel),
//         }
//     }
// }

// impl FromStr for EventPerTraceUniqueId {
//     type Err = ();
//     /// Inverse of Display::fmt
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         if let Ok(ogr) = s.parse::<u32>() {
//             return Ok(Self::Og(OgRank(ogr)));
//         }
//         // Ok(Self::First(Predicate::from_str(s).map_err(|_| ())?))
//         Result::Err(())
//     }
// }

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
    for p in get_first_predicates(axioms, &concretes, conninfo) {
        // println!("DEBUG: {}", p);
        for (ogr, record) in concretes.iter().enumerate() {
            if p.holds(record, conninfo) {
                // firsts[ogr].push(Event {
                //     unique_id: EventPerTraceUniqueId::First(p.clone()),
                //     ..record.clone()
                // });
                firsts[ogr].push(Event::First(p.clone()));
                break;
            }
        }
    }
    let mut ret = Vec::new();
    for (ogidx, e) in concretes.into_iter().enumerate() {
        for first in firsts.get_mut(ogidx).unwrap().drain(..) {
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
        | BinaryRelation::TagPlusDelay2FedGreaterThanOrEquals
        | BinaryRelation::TagGreaterThanOrEqual
        | BinaryRelation::TagEquals
        | BinaryRelation::TagLessThan
        | BinaryRelation::TagLessThanOrEqual
        | BinaryRelation::FederateEquals
        | BinaryRelation::TagStrictPlusDelay2FedLessThan
        | BinaryRelation::TagStrictPlusDelayFromSomeImmUpstreamFedGreaterThanOrEquals
        | BinaryRelation::TagPlusDelayToAllImmDowntreamFedsLessThan
        | BinaryRelation::FederateZeroDelayDirectlyUpstreamOf
        | BinaryRelation::FederateDirectlyUpstreamOf => {}
        BinaryRelation::IsFirst(rel) => {
            for e in concretes.iter().filter(|e| event.holds(e, conninfo)) {
                // if let Event::Concrete {
                //     event,
                //     tag,
                //     fedid,
                //     ogrank,
                // } = e
                // {
                //     if ogrank.0 == 0 {
                //         println!("DEBUG:!! {} is first", e);
                //     }
                // }
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
            Predicate::TagNonzero => {
                if let Event::Concrete { tag, .. } = e {
                    tag != &Tag(0, 0)
                } else {
                    false
                }
            }
            Predicate::TagFinite => {
                if let Event::Concrete { tag, .. } = e {
                    tag.0.abs() < 1_000_000_000_000
                } else {
                    false
                }
            }
            Predicate::EventIs(event) => {
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
                f(ptag, tag, pfedid, fedid, conninfo.0.get(&(*pfedid, *fedid)))
            } else {
                false
            }
        };
        let compare_tags_given_delay = |f: fn(&Tag, &Tag, &Delay) -> bool| {
            if let (
                Event::Concrete {
                    tag: ptag,
                    fedid: pfedid,
                    ..
                },
                Event::Concrete { tag, fedid, .. },
            ) = (preceding, e)
            {
                if let Some(delay) = conninfo.0.get(&(*pfedid, *fedid)) {
                    f(&(*ptag), &tag, delay)
                } else {
                    false
                }
            } else {
                false
            }
        };
        let compare_tags_given_all_downstream_delays =
            |f: fn(&Tag, &Tag, &[(FedId, Delay)]) -> bool| {
                if let (
                    Event::Concrete {
                        tag: ptag,
                        fedid: pfedid,
                        ..
                    },
                    Event::Concrete { tag, fedid, .. },
                ) = (preceding, e)
                {
                    let mut delays = Vec::new();
                    for ((src, _), delay) in
                        conninfo.0.iter().filter(|((_, dst), _)| *dst == *fedid)
                    {
                        delays.push((*src, *delay));
                    }
                    f(&(*ptag), &tag, &delays)
                } else {
                    false
                }
            };
        match self {
            BinaryRelation::TagPlusDelay2FedEquals => {
                compare_tags_given_delay(|a, b, d| *a + *d == *b)
            }
            BinaryRelation::TagPlusDelay2FedLessThan => {
                compare_tags_given_delay(|a, b, d| *a + *d < *b)
            }
            BinaryRelation::TagPlusDelay2FedLessThanOrEqual => {
                compare_tags_given_delay(|a, b, d| *a + *d <= *b)
            }
            BinaryRelation::TagPlusDelay2FedGreaterThanOrEquals => {
                compare_tags_given_delay(|a, b, d| *a + *d >= *b)
            }
            BinaryRelation::TagGreaterThanOrEqual => compare_tags_given_delay(|a, b, d| *a >= *b),
            BinaryRelation::TagEquals => neither_first(|a, b, _, _, _| a == b),
            BinaryRelation::TagLessThan => neither_first(|a, b, _, _, _| a < b),
            BinaryRelation::TagLessThanOrEqual => neither_first(|a, b, _, _, _| a <= b),
            BinaryRelation::TagStrictPlusDelay2FedLessThan => {
                compare_tags_given_delay(|a, b, d| a.strict_add(*d) < *b)
            }
            // conninfo
            //     .0
            //     .iter()
            //     .filter(|((src, _), _)| *src == preceding.fedid)
            //     .all(|(_, delay)| preceding.tag.strict_add(*delay) < e.tag),
            BinaryRelation::TagStrictPlusDelayFromSomeImmUpstreamFedGreaterThanOrEquals => {
                compare_tags_given_all_downstream_delays(|a, b, delays| {
                    delays.iter().any(|(_, delay)| a.strict_add(*delay) >= *b)
                })
            }
            // conninfo
            //     .0
            //     .iter()
            //     .filter(|((src, _), _)| *src == preceding.fedid)
            //     .any(|(_, delay)| preceding.tag.strict_add(*delay) >= e.tag),
            BinaryRelation::TagPlusDelayToAllImmDowntreamFedsLessThan => {
                compare_tags_given_all_downstream_delays(|a, b, delays| {
                    delays.iter().all(|(_, delay)| *a + *delay < *b)
                })
            }
            // conninfo
            //     .0
            //     .iter()
            //     .filter(|((src, _), _)| *src == preceding.fedid)
            //     .all(|(_, delay)| preceding.tag + *delay < e.tag),
            BinaryRelation::FederateEquals => neither_first(|_, _, a, b, _| a == b),
            // e.fedid == preceding.fedid,
            BinaryRelation::FederateZeroDelayDirectlyUpstreamOf => {
                neither_first(|_, _, _pfed, _fed, delay| delay == Some(&NO_DELAY))
            }
            BinaryRelation::FederateDirectlyUpstreamOf => {
                neither_first(|_, _, _pfed, _fed, delay| delay.is_some())
            }
            BinaryRelation::IsFirst(r) => {
                if let Event::First(other) = &preceding {
                    // if matches!(other, Predicate::BoundBinary(_)) {
                    //     println!(
                    //         "DEBUG: {}\n    {}\n    {}",
                    //         other,
                    //         Predicate::BoundBinary(Box::new((e.clone(), *r.clone()))),
                    //         other == &Predicate::BoundBinary(Box::new((e.clone(), *r.clone())))
                    //     );
                    // }
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
    ) -> Result<(HashSet<Rule>, Self), String> {
        let mut unused_axioms: HashSet<Rule> = axioms.iter().cloned().collect();
        let mut ogrank2immediatepredecessors = Vec::new();
        for (ogrank, tr) in trace.iter().enumerate() {
            let mut immediate_predecessors = HashSet::new();
            for rule in axioms {
                let before_size = immediate_predecessors.len();
                let iter =
                    Self::apply_rule(rule, tr, &trace[..ogrank], &trace[ogrank + 1..], conninfo)?;
                if !iter.is_empty() {
                    unused_axioms.remove(rule);
                }
                immediate_predecessors.extend(iter);
                // if immediate_predecessors.len() > before_size {
                //     unused_axioms.remove(rule);
                // }
            }
            ogrank2immediatepredecessors.push(immediate_predecessors);
        }
        Self::add_precedences_for_firsts(&mut ogrank2immediatepredecessors, trace, conninfo);
        Ok((
            unused_axioms,
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
            if let Event::First(ref rel) = &tr {
                // println!("DEBUG: {}", rel);
                let mut running_before_intersection: Option<HashSet<OgRank>> = None;
                let mut n_matches = 0;
                let mut first_match = None;
                // let mut second_match = None;
                for ((ogr, preds), tr) in ogrank2immediatepredecessors[ogrank + 1..]
                    .iter_mut()
                    .enumerate()
                    .zip(&trace[ogrank + 1..])
                    .filter(|(_, tr)| {
                        // println!("DEBUG: {}\n    {},    {}", tr, rel, rel.holds(tr, conninfo));
                        rel.holds(tr, conninfo) && !matches!(tr, Event::First(_))
                    })
                {
                    let ogr = ogrank + 1 + ogr;
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
                // if n_matches > 1 {
                //     println!("n_matches: {}", n_matches);
                // }
                // let len = ogrank2immediatepredecessors.len();
                // for after in ogrank2immediatepredecessors
                //     [first_match.unwrap() + 1..second_match.unwrap_or(len)]
                //     .iter_mut()
                //     .filter(|ogrs| ogrs.contains(&OgRank(ogrank as u32)))
                // {
                //     after.insert(OgRank(first_match.unwrap() as u32));
                // }
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

/// testing module
#[cfg(test)]
mod tests {
    use super::*;

    use crate::{conninfo::Tag, EventKind, Rule};

    use crate::BinaryRelation::{
        And, FederateEquals, FederateZeroDelayDirectlyUpstreamOf, TagPlusDelay2FedEquals,
        TagPlusDelay2FedLessThan, TagPlusDelay2FedLessThanOrEqual,
        TagPlusDelayToAllImmDowntreamFedsLessThan, Unary,
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
                TagPlusDelayToAllImmDowntreamFedsLessThan,
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
