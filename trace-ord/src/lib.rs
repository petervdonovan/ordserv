use std::{collections::HashSet, str::FromStr};

// pub enum EventType {
//     FED_ID,
//     ACK,
//     TIMESTAMP,
//     NET,
//     PTAG,
//     TAGGED_MSG,
//     TAG,
//     STOP_REQ,
//     STOP_REQ_REP,
//     STOP_GRN,
//     LTC,
// }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    FedId,
    Ack,
    Timestamp,
    Net,
    Ptag,
    TaggedMsg,
    Tag,
    StopReq,
    StopReqRep,
    StopGrn,
    Ltc,
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
    pub event: Event,
    pub preceding_event: Event,
    pub relations: Vec<Relation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Relation {
    TagEqual,
    TagLessThan,
    TagLessThanOrEqual,
    FederateEqual,
}

pub struct Unpermutables {
    pub ogrank2predecessors: Vec<PredecessorSet>,
    pub always_occurring: HashSet<OgRank>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OgRank(u32);

pub struct PredecessorSet {
    largest_predecessor: Option<OgRank>,
    delta: HashSet<OgRank>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceRecord {
    pub event: Event,
    pub tag: (i64, i64),
    pub source: i32,
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
        match self {
            Relation::TagEqual => tr_before.tag == tr_after.tag,
            Relation::TagLessThan => tr_before.tag < tr_after.tag,
            Relation::TagLessThanOrEqual => tr_before.tag <= tr_after.tag,
            Relation::FederateEqual => tr_before.source == tr_after.source,
        }
    }
}

impl OgRank {
    fn all_predecessors_and_self<'a>(
        &self,
        ogrank2predecessors: &'a [PredecessorSet],
        empty: &'a HashSet<OgRank>,
    ) -> impl Iterator<Item = OgRank> + 'a {
        std::iter::once(*self)
            .chain(
                if let Some(pred) = ogrank2predecessors[self.0 as usize].largest_predecessor {
                    ogrank2predecessors[pred.0 as usize].delta.iter().copied()
                } else {
                    empty.iter().copied()
                },
            )
            .chain(ogrank2predecessors[self.0 as usize].delta.iter().copied())
    }
    fn idx(&self) -> usize {
        self.0 as usize
    }
}

impl Unpermutables {
    pub fn from_realizable_trace(
        trace: Vec<TraceRecord>,
        axioms: Vec<Rule>,
        always_occurring: HashSet<OgRank>,
    ) -> Self {
        let mut ogrank2predecessors = Vec::new();
        for rule in axioms {
            for (ogrank, tr) in trace.iter().enumerate() {
                ogrank2predecessors.push(PredecessorSet {
                    largest_predecessor: None,
                    delta: Self::apply_rule(&rule, tr, &trace[..ogrank], &trace[ogrank + 1..]),
                });
            }
        }
        Self {
            ogrank2predecessors,
            always_occurring,
        }
    }
    pub fn apply_transitivity(&mut self) {
        let empty = HashSet::new();
        let possible_transitive_predecessors =
            |ogrank: usize, ogrank2predecessors: &[PredecessorSet]| {
                ogrank2predecessors[ogrank]
                    .delta
                    .iter()
                    .filter(|&pred| self.always_occurring.contains(pred))
                    .flat_map(|&pred| pred.all_predecessors_and_self(&ogrank2predecessors, &empty))
                    .chain(ogrank2predecessors[ogrank].delta.iter().copied())
            };
        let len = self.ogrank2predecessors.len();
        for ogrank in 0..len {
            let largest_predecessor =
                possible_transitive_predecessors(ogrank, &self.ogrank2predecessors).max_by_key(
                    |&before_ogrank| {
                        self.ogrank2predecessors[before_ogrank.0 as usize]
                            .delta
                            .len()
                    },
                );
            self.ogrank2predecessors[ogrank].largest_predecessor = largest_predecessor;
            let mut new_delta = HashSet::new();
            for predecessor in possible_transitive_predecessors(ogrank, &self.ogrank2predecessors)
                .filter(|it| {
                    largest_predecessor.is_none()
                        || !self.ogrank2predecessors[largest_predecessor.unwrap().0 as usize]
                            .delta
                            .contains(it)
                })
            {
                new_delta.extend(
                    predecessor.all_predecessors_and_self(&self.ogrank2predecessors, &empty),
                );
            }
            if let Some(largest_predecessor) = largest_predecessor {
                if new_delta.len()
                    > self.ogrank2predecessors[largest_predecessor.idx()]
                        .delta
                        .len()
                        / 8
                {
                    self.ogrank2predecessors[largest_predecessor.idx()].largest_predecessor = None;
                    new_delta.extend(
                        self.ogrank2predecessors[largest_predecessor.idx()]
                            .delta
                            .iter()
                            .copied(),
                    );
                }
            }
            self.ogrank2predecessors[ogrank].delta = new_delta;
        }
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
                "Tracepoint {:?} followed by {:?} is a counterexample to the axiom {:?}",
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
