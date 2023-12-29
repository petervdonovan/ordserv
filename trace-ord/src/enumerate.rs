use std::collections::{HashMap, HashSet};

use crate::{BinaryRelation, EventKind, Predicate, Rule};

#[derive(Debug, Default)]
pub struct PredicatesWithFuel {
    predicates_by_fuel: Vec<Vec<(Predicate, PredicateAbstraction)>>,
}
#[derive(Debug, Default, Clone)]
pub struct PredicateAbstraction {
    pub possible_events: Option<HashSet<EventKind>>,
    pub predicate2powerbool: HashMap<Predicate, PowerBool>,
}
#[derive(Debug, Default)]
pub struct PredicatesWithBoundBinariesWithFuel(PredicatesWithFuel);
#[derive(Debug, Default)]
pub struct BinaryRelationsWithFuel {
    upper_bounded_binary_relations_by_fuel: Vec<Vec<BinaryRelation>>,
    lower_bounded_binary_relations_by_fuel: Vec<Vec<BinaryRelation>>,
    compact_binary_relations_by_fuel: Vec<Vec<BinaryRelation>>,
}
#[derive(Debug, Default)]
pub struct BinaryRelationsWithUnariesWithFuel(BinaryRelationsWithFuel);
#[derive(Debug, Default)]
pub struct RulesWithFuel {
    rules_by_fuel: Vec<Vec<Rule>>,
}
#[derive(Debug, Clone, Copy)]
pub struct PowerBool {
    pub maybe_true: bool,
    pub maybe_false: bool,
}

impl PredicatesWithFuel {
    pub fn advance(
        &mut self,
        fuel: usize,
    ) -> impl Iterator<Item = &(Predicate, PredicateAbstraction)> {
        fn exact_fuel(
            predicates: &PredicatesWithFuel,
            fuel: usize,
        ) -> Vec<(Predicate, PredicateAbstraction)> {
            if fuel == 0 {
                let mut ret = vec![
                    (
                        Predicate::FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag,
                        PredicateAbstraction::fact(
                            Predicate::FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag,
                        ),
                    ),
                    (
                        Predicate::TagNonzero,
                        PredicateAbstraction::fact(Predicate::TagNonzero),
                    ),
                    (
                        Predicate::TagFinite,
                        PredicateAbstraction::fact(Predicate::TagFinite),
                    ),
                ];
                for kind in enum_iterator::all::<EventKind>() {
                    ret.push((Predicate::EventIs(kind), PredicateAbstraction::event(kind)));
                }
                ret
            } else {
                let mut ret = vec![];
                // add And, Or, and Not, but not IsFirst or BoundBinary
                for (predicate, abstraction) in predicates.predicates_by_fuel[fuel - 1]
                    .iter()
                    .filter(|&(predicate, _)| {
                        // no double negation
                        !matches!(predicate, Predicate::Not(_))
                    })
                {
                    ret.push((
                        Predicate::Not(Box::new(predicate.clone())),
                        abstraction.not(),
                    ));
                }
                let inexact_combinations =
                    inexact_combinations(&predicates.predicates_by_fuel, fuel);
                // println!("inexact_combinations: {:?}", inexact_combinations);
                for combination in inexact_combinations.into_iter() {
                    let bslice = || {
                        combination
                            .iter()
                            .map(|it| it.0.clone())
                            .collect::<Vec<_>>()
                            .into_boxed_slice()
                    };
                    let conniter = || combination.iter().map(|it| it.1.clone());
                    ret.push((
                        Predicate::And(bslice()),
                        PredicateAbstraction::and(conniter()),
                    ));
                    ret.push((
                        Predicate::Or(bslice()),
                        PredicateAbstraction::or(conniter()),
                    ));
                }
                ret
            }
        }
        let mut ret: Box<dyn Iterator<Item = &(Predicate, PredicateAbstraction)>> =
            Box::new(std::iter::empty());
        let len = self.predicates_by_fuel.len();
        for fuel in len..=fuel {
            let exact = exact_fuel(self, fuel);
            self.predicates_by_fuel.push(exact);
        }
        for fuel in len..=fuel {
            ret = Box::new(ret.chain(self.predicates_by_fuel[fuel].iter()));
        }
        ret
    }
}

impl PredicateAbstraction {
    pub fn fact(predicate: Predicate) -> Self {
        Self {
            possible_events: None,
            predicate2powerbool: vec![(predicate, PowerBool::new_true())]
                .into_iter()
                .collect(),
        }
    }
    pub fn event(kind: EventKind) -> Self {
        Self {
            possible_events: Some(vec![kind].into_iter().collect()),
            predicate2powerbool: HashMap::default(),
        }
    }
    pub fn and(terms: impl Iterator<Item = PredicateAbstraction> + Clone) -> Self {
        let possible_events: Option<HashSet<EventKind>> = terms
            .clone()
            .filter_map(|it| it.possible_events)
            .fold(None, |acc, it| {
                if let Some(acc) = acc {
                    Some(acc.intersection(&it).cloned().collect())
                } else {
                    Some(it.clone())
                }
            });
        let predicate2powerbool =
            terms.fold(HashMap::<Predicate, PowerBool>::default(), |mut acc, it| {
                for (predicate, powerbool) in it.predicate2powerbool.iter() {
                    let entry = acc.entry(predicate.clone()).or_default();
                    entry.and(powerbool);
                }
                acc
            });
        Self {
            possible_events,
            predicate2powerbool,
        }
    }
    pub fn or(terms: impl Iterator<Item = PredicateAbstraction> + Clone) -> Self {
        let possible_events: Option<HashSet<EventKind>> = terms
            .clone()
            .filter_map(|it| it.possible_events)
            .fold(None, |acc, it| {
                if let Some(acc) = acc {
                    Some(acc.union(&it).cloned().collect())
                } else {
                    Some(it.clone())
                }
            });
        let predicate2powerbool =
            terms.fold(HashMap::<Predicate, PowerBool>::default(), |mut acc, it| {
                for (predicate, powerbool) in it.predicate2powerbool.iter() {
                    // do not keep entries that map to top after being or'ed
                    let entry = acc.entry(predicate.clone()).or_default();
                    entry.or(powerbool);
                    if entry.is_top() {
                        acc.remove(predicate);
                    }
                }
                acc
            });
        Self {
            possible_events,
            predicate2powerbool,
        }
    }
    pub fn not(&self) -> Self {
        let predicate2powerbool = self
            .predicate2powerbool
            .iter()
            .map(|(predicate, powerbool)| (predicate.clone(), powerbool.not()))
            .collect();
        Self {
            possible_events: None,
            predicate2powerbool,
        }
    }
    pub fn uninhabitable(&self) -> bool {
        self.possible_events
            .as_ref()
            .is_some_and(|it| it.is_empty())
            || self
                .predicate2powerbool
                .iter()
                .any(|(_, pb)| pb.uninhabitable())
    }
}

fn inexact_combinations<T>(lists_by_subfuel: &[Vec<T>], fuel: usize) -> Vec<Vec<T>>
where
    T: Clone,
{
    if fuel <= (1 << 1) + 1 {
        return vec![];
    }
    let max_subfuels = subfuels(fuel);
    let lesser_subfuels = subfuels(fuel - 1);
    let mut combinations: Vec<Vec<T>> = vec![];
    for last_envelope_break_location in 0..max_subfuels.len() {
        println!(
            "DEBUG: last_envelope_break_location: {}; fuel: {}; max_subfuels: {:?}",
            last_envelope_break_location, fuel, max_subfuels
        );
        let ranges: Vec<(usize, usize)> = max_subfuels
            .iter()
            .enumerate()
            .filter_map(|(idx, &subfuel)| {
                if idx <= last_envelope_break_location {
                    Some((
                        *lesser_subfuels
                            .get(last_envelope_break_location)
                            .unwrap_or(&0),
                        subfuel,
                    ))
                } else {
                    lesser_subfuels
                        .get(idx)
                        .map(|&lesser_subfuel| (0, lesser_subfuel))
                }
            })
            .collect::<Vec<_>>();
        println!("DEBUG: ranges: {:?}", ranges);
        let mut next_combinations = inexact_combinations_with_init(
            lists_by_subfuel,
            ranges.iter().map(|(_, b)| b).cloned().collect(),
            ranges.iter().map(|(a, _)| a).cloned().collect(),
        );
        combinations.append(&mut next_combinations);
    }
    return combinations;
    /// get the fuels of the constituent parts of an arbitrary-length item of the given fuel
    fn subfuels(fuel: usize) -> Vec<usize> {
        // return a geometrically decreasing sequence of subfuels where the maximum subfuel is no greater than the given fuel
        let mut subfuels = vec![];
        let mut subfuel = fuel;
        while subfuel > 0 {
            subfuels.push(subfuel);
            subfuel >>= 1;
        }
        subfuels
    }
    fn inexact_combinations_with_init<T>(
        lists_by_subfuel: &[Vec<T>],
        max_subfuels: Vec<usize>,
        init: Vec<usize>,
    ) -> Vec<Vec<T>>
    where
        T: Clone,
    {
        let mut exact_subfuels = init;
        let mut increment_idx = 0;
        let mut incrementables = max_subfuels
            .iter()
            .enumerate()
            .filter_map(|(idx, &max_subfuel)| {
                let diff = max_subfuel - exact_subfuels[idx];
                if diff > 1 {
                    Some((idx, diff))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let mut combinations: Vec<Vec<T>> = vec![];
        loop {
            println!(
                "DEBUG: exact_subfuels: {:?}; increment_idx: {}",
                exact_subfuels, increment_idx
            );
            let next_combinations = exact_combinations(lists_by_subfuel, &exact_subfuels);
            combinations.extend(next_combinations);
            if incrementables.is_empty() {
                break;
            }
            exact_subfuels[incrementables[increment_idx].0] += 1;
            incrementables[increment_idx].1 -= 1;
            if incrementables[increment_idx].1 == 1 {
                incrementables.remove(increment_idx);
                if incrementables.is_empty() {
                    break;
                }
            }
            increment_idx = (increment_idx + 1) % incrementables.len();
        }
        return combinations;
        /// get all combinations of predicates with fuel exactly matching the exact_subfuel
        fn exact_combinations<T>(
            lists_by_subfuel: &[Vec<T>],
            exact_subfuels: &[usize],
        ) -> Vec<Vec<T>>
        where
            T: Clone,
        {
            let mut combinations: Vec<Vec<T>> = vec![]; // invariant: each vector is strictly decreasing in idx
            let mut idxs: Vec<usize> = vec![];
            let mut last_subfuel = 0;
            for &subfuel in exact_subfuels.iter() {
                let mut next_combinations: Vec<Vec<T>> = vec![];
                let mut next_idxs: Vec<usize> = vec![];
                if combinations.is_empty() {
                    combinations = lists_by_subfuel[subfuel]
                        .iter()
                        .cloned()
                        .map(|it| vec![it])
                        .collect();
                    idxs = (0..combinations.len()).collect::<Vec<_>>();
                    last_subfuel = subfuel;
                    continue;
                }
                for (combination, strictmax_idx) in combinations.iter().zip(idxs.iter()) {
                    for (idx, item) in lists_by_subfuel[subfuel]
                        .iter()
                        .take(if last_subfuel == subfuel {
                            *strictmax_idx
                        } else {
                            usize::MAX
                        })
                        .enumerate()
                    {
                        let mut next_combination = combination.clone();
                        next_combination.push(item.clone());
                        next_combinations.push(next_combination);
                        next_idxs.push(idx);
                    }
                }
                combinations = next_combinations;
                idxs = next_idxs;
            }
            combinations
        }
    }
}
impl Default for PowerBool {
    fn default() -> Self {
        Self {
            maybe_true: true,
            maybe_false: true,
        }
    }
}
impl PowerBool {
    fn and(&mut self, other: &Self) {
        self.maybe_true &= other.maybe_true;
        self.maybe_false &= other.maybe_false;
    }
    fn or(&mut self, other: &Self) {
        self.maybe_true |= other.maybe_true;
        self.maybe_false |= other.maybe_false;
    }
    fn not(&self) -> Self {
        if self.is_false() {
            Self::new_true()
        } else if self.is_true() {
            Self::new_false()
        } else {
            Self::default()
        }
    }
    fn is_top(&self) -> bool {
        self.maybe_true && self.maybe_false
    }
    fn is_true(&self) -> bool {
        self.maybe_true && !self.maybe_false
    }
    fn is_false(&self) -> bool {
        !self.maybe_true && self.maybe_false
    }
    fn new_true() -> Self {
        Self {
            maybe_true: true,
            maybe_false: false,
        }
    }
    fn new_false() -> Self {
        Self {
            maybe_true: false,
            maybe_false: true,
        }
    }
    fn uninhabitable(&self) -> bool {
        !self.maybe_true && !self.maybe_false
    }
}
mod tests {
    use super::*;

    #[test]
    fn test_predicates_with_fuel() {
        let mut predicates = PredicatesWithFuel::default();
        let predicates: Vec<_> = predicates.advance(4).collect();
        let expect = expect_test::expect![[r#"
            FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag
            And([Not(EventIs(SendTimestamp)), Not(TagFinite)])
            And([Not(EventIs(SendTaggedMsg)), Not(EventIs(RecvNet))])
            And([Not(EventIs(SendStopReq)), Not(EventIs(RecvNet))])
            And([Not(EventIs(RecvLtc)), Not(EventIs(RecvPortAbs))])
            And([EventIs(RecvTimestamp), TagFinite, TagNonzero])
            And([EventIs(SendPortAbs), EventIs(RecvTimestamp), TagNonzero])
            And([EventIs(SendPtag), TagFinite, TagNonzero])
            And([EventIs(SendTaggedMsg), EventIs(SendAck), TagNonzero])
            And([EventIs(RecvTaggedMsg), TagFinite, TagNonzero])
            And([EventIs(RecvTaggedMsg), EventIs(SendPtag), EventIs(RecvNet)])
            And([EventIs(SendTag), EventIs(RecvPortAbs), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            And([EventIs(RecvStopReq), EventIs(SendAck), TagFinite])
            And([EventIs(RecvStopReq), EventIs(SendTaggedMsg), EventIs(RecvFedId)])
            And([EventIs(SendStopReq), EventIs(RecvTimestamp), TagFinite])
            And([EventIs(SendStopReq), EventIs(RecvTaggedMsg), TagNonzero])
            And([EventIs(RecvStopReqRep), EventIs(SendTimestamp), TagFinite])
            And([EventIs(RecvStopReqRep), EventIs(SendTaggedMsg), EventIs(RecvNet)])
            And([EventIs(RecvStopReqRep), EventIs(SendStopReq), EventIs(RecvNet)])
            And([EventIs(SendStopGrn), EventIs(RecvPortAbs), EventIs(RecvTimestamp)])
            And([EventIs(SendStopGrn), EventIs(RecvStopReq), TagNonzero])
            And([EventIs(RecvLtc), EventIs(SendAck), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            And([EventIs(RecvLtc), EventIs(SendTaggedMsg), TagNonzero])
            And([EventIs(RecvLtc), EventIs(SendStopReq), TagNonzero])
            And([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(RecvFedId)])
            And([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), TagFinite, EventIs(SendStopReq)])
            And([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendTimestamp), EventIs(SendPortAbs)])
            And([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendPortAbs), TagNonzero])
            And([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendPtag), EventIs(SendTag)])
            And([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendTag), EventIs(RecvTimestamp)])
            And([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendStopReq), EventIs(RecvLtc)])
            And([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(RecvLtc), EventIs(SendTaggedMsg)])
            And([Not(TagNonzero), TagFinite, EventIs(SendAck)])
            And([Not(TagNonzero), EventIs(SendAck), EventIs(RecvStopReqRep)])
            And([Not(TagNonzero), EventIs(RecvNet), EventIs(RecvPortAbs)])
            And([Not(TagNonzero), EventIs(SendPtag), TagFinite])
            And([Not(TagNonzero), EventIs(RecvTaggedMsg), EventIs(RecvStopReq)])
            And([Not(TagNonzero), EventIs(SendStopReq), EventIs(RecvNet)])
            And([Not(TagNonzero), EventIs(RecvLtc), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            And([Not(TagFinite), TagNonzero, EventIs(RecvTaggedMsg)])
            And([Not(TagFinite), EventIs(SendAck), EventIs(SendTimestamp)])
            And([Not(TagFinite), EventIs(RecvTimestamp), EventIs(SendStopGrn)])
            And([Not(TagFinite), EventIs(RecvPortAbs), EventIs(SendPtag)])
            And([Not(TagFinite), EventIs(RecvTaggedMsg), EventIs(RecvFedId)])
            And([Not(TagFinite), EventIs(RecvStopReq), EventIs(SendStopReq)])
            And([Not(TagFinite), EventIs(SendStopGrn), EventIs(SendPortAbs)])
            And([Not(EventIs(RecvFedId)), TagNonzero, TagNonzero])
            And([Not(EventIs(RecvFedId)), EventIs(RecvFedId), EventIs(SendTag)])
            And([Not(EventIs(RecvFedId)), EventIs(RecvTimestamp), EventIs(RecvTimestamp)])
            And([Not(EventIs(RecvFedId)), EventIs(SendPortAbs), EventIs(RecvLtc)])
            And([Not(EventIs(RecvFedId)), EventIs(SendTaggedMsg), EventIs(SendTaggedMsg)])
            And([Not(EventIs(RecvFedId)), EventIs(RecvStopReq), EventIs(SendAck)])
            And([Not(EventIs(RecvFedId)), EventIs(RecvStopReqRep), EventIs(RecvStopReqRep)])
            And([Not(EventIs(SendAck)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(RecvPortAbs)])
            And([Not(EventIs(SendAck)), EventIs(RecvFedId), TagFinite])
            And([Not(EventIs(SendAck)), EventIs(SendTimestamp), EventIs(RecvStopReq)])
            And([Not(EventIs(SendAck)), EventIs(SendPortAbs), EventIs(RecvNet)])
            And([Not(EventIs(SendAck)), EventIs(SendTaggedMsg), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            And([Not(EventIs(SendAck)), EventIs(SendTag), EventIs(RecvTaggedMsg)])
            And([Not(EventIs(SendAck)), EventIs(RecvStopReqRep), EventIs(SendTimestamp)])
            And([Not(EventIs(SendAck)), EventIs(RecvLtc), EventIs(SendStopGrn)])
            And([Not(EventIs(SendTimestamp)), TagFinite, EventIs(SendPtag)])
            And([Not(EventIs(SendTimestamp)), EventIs(SendTimestamp), EventIs(RecvFedId)])
            And([Not(EventIs(SendTimestamp)), EventIs(RecvNet), EventIs(SendStopReq)])
            And([Not(EventIs(SendTimestamp)), EventIs(SendPtag), EventIs(SendPortAbs)])
            And([Not(EventIs(SendTimestamp)), EventIs(SendTag), TagNonzero])
            And([Not(EventIs(SendTimestamp)), EventIs(SendStopReq), EventIs(SendTag)])
            And([Not(EventIs(SendTimestamp)), EventIs(RecvLtc), EventIs(RecvTimestamp)])
            And([Not(EventIs(RecvTimestamp)), TagNonzero, EventIs(RecvLtc)])
            And([Not(EventIs(RecvTimestamp)), EventIs(SendAck), EventIs(SendTaggedMsg)])
            And([Not(EventIs(RecvTimestamp)), EventIs(RecvNet), EventIs(SendAck)])
            And([Not(EventIs(RecvTimestamp)), EventIs(RecvPortAbs), EventIs(RecvStopReqRep)])
            And([Not(EventIs(RecvTimestamp)), EventIs(RecvTaggedMsg), EventIs(RecvPortAbs)])
            And([Not(EventIs(RecvTimestamp)), EventIs(SendStopReq), TagFinite])
            And([Not(EventIs(RecvTimestamp)), EventIs(SendStopGrn), EventIs(RecvStopReq)])
            And([Not(EventIs(RecvNet)), TagNonzero, EventIs(RecvNet)])
            And([Not(EventIs(RecvNet)), EventIs(SendAck), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            And([Not(EventIs(RecvNet)), EventIs(RecvTimestamp), EventIs(RecvTaggedMsg)])
            And([Not(EventIs(RecvNet)), EventIs(RecvPortAbs), EventIs(SendTimestamp)])
            And([Not(EventIs(RecvNet)), EventIs(SendTaggedMsg), EventIs(SendStopGrn)])
            And([Not(EventIs(RecvNet)), EventIs(RecvStopReq), EventIs(SendPtag)])
            And([Not(EventIs(RecvNet)), EventIs(SendStopGrn), EventIs(RecvFedId)])
            And([Not(EventIs(SendPortAbs)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(SendStopReq)])
            And([Not(EventIs(SendPortAbs)), EventIs(RecvFedId), EventIs(SendPortAbs)])
            And([Not(EventIs(SendPortAbs)), EventIs(RecvTimestamp), TagNonzero])
            And([Not(EventIs(SendPortAbs)), EventIs(SendPortAbs), EventIs(SendTag)])
            And([Not(EventIs(SendPortAbs)), EventIs(SendTaggedMsg), EventIs(RecvTimestamp)])
            And([Not(EventIs(SendPortAbs)), EventIs(SendTag), EventIs(RecvLtc)])
            And([Not(EventIs(SendPortAbs)), EventIs(RecvStopReqRep), EventIs(SendTaggedMsg)])
            And([Not(EventIs(RecvPortAbs)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(SendAck)])
            And([Not(EventIs(RecvPortAbs)), TagFinite, EventIs(RecvStopReqRep)])
            And([Not(EventIs(RecvPortAbs)), EventIs(SendTimestamp), EventIs(RecvPortAbs)])
            And([Not(EventIs(RecvPortAbs)), EventIs(SendPortAbs), TagFinite])
            And([Not(EventIs(RecvPortAbs)), EventIs(SendPtag), EventIs(RecvStopReq)])
            And([Not(EventIs(RecvPortAbs)), EventIs(SendTag), EventIs(RecvNet)])
            And([Not(EventIs(RecvPortAbs)), EventIs(RecvStopReqRep), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            And([Not(EventIs(RecvPortAbs)), EventIs(RecvLtc), EventIs(RecvTaggedMsg)])
            And([Not(EventIs(SendPtag)), TagFinite, EventIs(SendTimestamp)])
            And([Not(EventIs(SendPtag)), EventIs(SendAck), EventIs(SendStopGrn)])
            And([Not(EventIs(SendPtag)), EventIs(RecvNet), EventIs(SendPtag)])
            And([Not(EventIs(SendPtag)), EventIs(SendPtag), EventIs(RecvFedId)])
            And([Not(EventIs(SendPtag)), EventIs(RecvTaggedMsg), EventIs(SendStopReq)])
            And([Not(EventIs(SendPtag)), EventIs(SendStopReq), EventIs(SendPortAbs)])
            And([Not(EventIs(SendPtag)), EventIs(RecvLtc), TagNonzero])
            And([Not(EventIs(SendTaggedMsg)), TagNonzero, EventIs(SendTag)])
            And([Not(EventIs(SendTaggedMsg)), EventIs(SendAck), EventIs(RecvTimestamp)])
            And([Not(EventIs(SendTaggedMsg)), EventIs(RecvTimestamp), EventIs(RecvLtc)])
            And([Not(EventIs(SendTaggedMsg)), EventIs(RecvPortAbs), EventIs(SendTaggedMsg)])
            And([Not(EventIs(SendTaggedMsg)), EventIs(RecvTaggedMsg), EventIs(SendAck)])
            And([Not(EventIs(SendTaggedMsg)), EventIs(RecvStopReq), EventIs(RecvStopReqRep)])
            And([Not(EventIs(SendTaggedMsg)), EventIs(SendStopGrn), EventIs(RecvPortAbs)])
            And([Not(EventIs(RecvTaggedMsg)), TagNonzero, TagFinite])
            And([Not(EventIs(RecvTaggedMsg)), EventIs(RecvFedId), EventIs(RecvStopReq)])
            And([Not(EventIs(RecvTaggedMsg)), EventIs(RecvTimestamp), EventIs(RecvNet)])
            And([Not(EventIs(RecvTaggedMsg)), EventIs(RecvPortAbs), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            And([Not(EventIs(RecvTaggedMsg)), EventIs(SendTaggedMsg), EventIs(RecvTaggedMsg)])
            And([Not(EventIs(RecvTaggedMsg)), EventIs(RecvStopReq), EventIs(SendTimestamp)])
            And([Not(EventIs(RecvTaggedMsg)), EventIs(RecvStopReqRep), EventIs(SendStopGrn)])
            And([Not(EventIs(SendTag)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(SendPtag)])
            And([Not(EventIs(SendTag)), EventIs(RecvFedId), EventIs(RecvFedId)])
            And([Not(EventIs(SendTag)), EventIs(SendTimestamp), EventIs(SendStopReq)])
            And([Not(EventIs(SendTag)), EventIs(SendPortAbs), EventIs(SendPortAbs)])
            And([Not(EventIs(SendTag)), EventIs(SendTaggedMsg), TagNonzero])
            And([Not(EventIs(SendTag)), EventIs(SendTag), EventIs(SendTag)])
            And([Not(EventIs(SendTag)), EventIs(RecvStopReqRep), EventIs(RecvTimestamp)])
            And([Not(EventIs(SendTag)), EventIs(RecvLtc), EventIs(RecvLtc)])
            And([Not(EventIs(RecvStopReq)), TagFinite, EventIs(SendTaggedMsg)])
            And([Not(EventIs(RecvStopReq)), EventIs(SendTimestamp), EventIs(SendAck)])
            And([Not(EventIs(RecvStopReq)), EventIs(RecvNet), EventIs(RecvStopReqRep)])
            And([Not(EventIs(RecvStopReq)), EventIs(SendPtag), EventIs(RecvPortAbs)])
            And([Not(EventIs(RecvStopReq)), EventIs(SendTag), TagFinite])
            And([Not(EventIs(RecvStopReq)), EventIs(SendStopReq), EventIs(RecvStopReq)])
            And([Not(EventIs(RecvStopReq)), EventIs(RecvLtc), EventIs(RecvNet)])
            And([Not(EventIs(SendStopReq)), TagFinite, FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            And([Not(EventIs(SendStopReq)), EventIs(SendAck), EventIs(RecvTaggedMsg)])
            And([Not(EventIs(SendStopReq)), EventIs(RecvNet), EventIs(SendTimestamp)])
            And([Not(EventIs(SendStopReq)), EventIs(RecvPortAbs), EventIs(SendStopGrn)])
            And([Not(EventIs(SendStopReq)), EventIs(RecvTaggedMsg), EventIs(SendPtag)])
            And([Not(EventIs(SendStopReq)), EventIs(SendStopReq), EventIs(RecvFedId)])
            And([Not(EventIs(SendStopReq)), EventIs(SendStopGrn), EventIs(SendStopReq)])
            And([Not(EventIs(RecvStopReqRep)), TagNonzero, EventIs(SendPortAbs)])
            And([Not(EventIs(RecvStopReqRep)), EventIs(SendAck), TagNonzero])
            And([Not(EventIs(RecvStopReqRep)), EventIs(RecvTimestamp), EventIs(SendTag)])
            And([Not(EventIs(RecvStopReqRep)), EventIs(RecvPortAbs), EventIs(RecvTimestamp)])
            And([Not(EventIs(RecvStopReqRep)), EventIs(SendTaggedMsg), EventIs(RecvLtc)])
            And([Not(EventIs(RecvStopReqRep)), EventIs(RecvStopReq), EventIs(SendTaggedMsg)])
            And([Not(EventIs(RecvStopReqRep)), EventIs(SendStopGrn), EventIs(SendAck)])
            And([Not(EventIs(SendStopGrn)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(RecvStopReqRep)])
            And([Not(EventIs(SendStopGrn)), EventIs(RecvFedId), EventIs(RecvPortAbs)])
            And([Not(EventIs(SendStopGrn)), EventIs(RecvTimestamp), TagFinite])
            And([Not(EventIs(SendStopGrn)), EventIs(SendPortAbs), EventIs(RecvStopReq)])
            And([Not(EventIs(SendStopGrn)), EventIs(SendTaggedMsg), EventIs(RecvNet)])
            And([Not(EventIs(SendStopGrn)), EventIs(RecvStopReq), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            And([Not(EventIs(SendStopGrn)), EventIs(RecvStopReqRep), EventIs(RecvTaggedMsg)])
            And([Not(EventIs(RecvLtc)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(SendTimestamp)])
            And([Not(EventIs(RecvLtc)), TagFinite, EventIs(SendStopGrn)])
            And([Not(EventIs(RecvLtc)), EventIs(SendTimestamp), EventIs(SendPtag)])
            And([Not(EventIs(RecvLtc)), EventIs(SendPortAbs), EventIs(RecvFedId)])
            And([Not(EventIs(RecvLtc)), EventIs(SendPtag), EventIs(SendStopReq)])
            And([Not(EventIs(RecvLtc)), EventIs(SendTag), EventIs(SendPortAbs)])
            And([Not(EventIs(RecvLtc)), EventIs(RecvStopReqRep), TagNonzero])
            And([Not(EventIs(RecvLtc)), EventIs(RecvLtc), EventIs(SendTag)])
            And([Not(TagFinite), Not(TagNonzero), EventIs(RecvTimestamp)])
            And([Not(EventIs(RecvFedId)), Not(TagNonzero), EventIs(RecvLtc)])
            And([Not(EventIs(SendAck)), Not(TagNonzero), EventIs(SendTaggedMsg)])
            And([Not(EventIs(SendTimestamp)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendAck)])
            And([Not(EventIs(SendTimestamp)), Not(TagFinite), EventIs(RecvStopReqRep)])
            And([Not(EventIs(RecvTimestamp)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(RecvPortAbs)])
            And([Not(EventIs(RecvTimestamp)), Not(EventIs(RecvFedId)), TagFinite])
            And([Not(EventIs(RecvTimestamp)), Not(EventIs(SendTimestamp)), EventIs(RecvStopReq)])
            And([Not(EventIs(RecvNet)), Not(TagFinite), EventIs(RecvNet)])
            And([Not(EventIs(RecvNet)), Not(EventIs(SendTimestamp)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            And([Not(EventIs(SendPortAbs)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(RecvTaggedMsg)])
            And([Not(EventIs(SendPortAbs)), Not(EventIs(RecvFedId)), EventIs(SendTimestamp)])
            And([Not(EventIs(SendPortAbs)), Not(EventIs(SendTimestamp)), EventIs(SendStopGrn)])
            And([Not(EventIs(RecvPortAbs)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendPtag)])
            And([Not(EventIs(RecvPortAbs)), Not(EventIs(RecvFedId)), EventIs(RecvFedId)])
            And([Not(EventIs(RecvPortAbs)), Not(EventIs(SendTimestamp)), EventIs(SendStopReq)])
            And([Not(EventIs(RecvPortAbs)), Not(EventIs(SendPortAbs)), EventIs(SendPortAbs)])
            And([Not(EventIs(SendPtag)), Not(TagFinite), TagNonzero])
            And([Not(EventIs(SendPtag)), Not(EventIs(SendAck)), EventIs(SendTag)])
            And([Not(EventIs(SendPtag)), Not(EventIs(RecvNet)), EventIs(RecvTimestamp)])
            And([Not(EventIs(SendPtag)), Not(EventIs(RecvPortAbs)), EventIs(RecvLtc)])
            And([Not(EventIs(SendTaggedMsg)), Not(TagFinite), EventIs(SendTaggedMsg)])
            And([Not(EventIs(SendTaggedMsg)), Not(EventIs(SendTimestamp)), EventIs(SendAck)])
            And([Not(EventIs(SendTaggedMsg)), Not(EventIs(RecvNet)), EventIs(RecvStopReqRep)])
            And([Not(EventIs(SendTaggedMsg)), Not(EventIs(SendPtag)), EventIs(RecvPortAbs)])
            And([Not(EventIs(RecvTaggedMsg)), Not(TagFinite), TagFinite])
            And([Not(EventIs(RecvTaggedMsg)), Not(EventIs(SendAck)), EventIs(RecvStopReq)])
            And([Not(EventIs(RecvTaggedMsg)), Not(EventIs(RecvNet)), EventIs(RecvNet)])
            And([Not(EventIs(RecvTaggedMsg)), Not(EventIs(SendPtag)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            And([Not(EventIs(SendTag)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(RecvTaggedMsg)])
            And([Not(EventIs(SendTag)), Not(EventIs(RecvFedId)), EventIs(SendTimestamp)])
            And([Not(EventIs(SendTag)), Not(EventIs(SendTimestamp)), EventIs(SendStopGrn)])
            And([Not(EventIs(SendTag)), Not(EventIs(SendPortAbs)), EventIs(SendPtag)])
            And([Not(EventIs(SendTag)), Not(EventIs(SendTaggedMsg)), EventIs(RecvFedId)])
            And([Not(EventIs(RecvStopReq)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendStopReq)])
            And([Not(EventIs(RecvStopReq)), Not(EventIs(RecvFedId)), EventIs(SendPortAbs)])
            And([Not(EventIs(RecvStopReq)), Not(EventIs(RecvTimestamp)), TagNonzero])
            And([Not(EventIs(RecvStopReq)), Not(EventIs(SendPortAbs)), EventIs(SendTag)])
            And([Not(EventIs(RecvStopReq)), Not(EventIs(SendTaggedMsg)), EventIs(RecvTimestamp)])
            And([Not(EventIs(RecvStopReq)), Not(EventIs(SendTag)), EventIs(RecvLtc)])
            And([Not(EventIs(SendStopReq)), Not(TagFinite), EventIs(SendTaggedMsg)])
            And([Not(EventIs(SendStopReq)), Not(EventIs(SendTimestamp)), EventIs(SendAck)])
            And([Not(EventIs(SendStopReq)), Not(EventIs(RecvNet)), EventIs(RecvStopReqRep)])
            And([Not(EventIs(SendStopReq)), Not(EventIs(SendPtag)), EventIs(RecvPortAbs)])
            And([Not(EventIs(SendStopReq)), Not(EventIs(SendTag)), TagFinite])
            And([Not(EventIs(RecvStopReqRep)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(RecvStopReq)])
            And([Not(EventIs(RecvStopReqRep)), Not(EventIs(RecvFedId)), EventIs(RecvNet)])
            And([Not(EventIs(RecvStopReqRep)), Not(EventIs(RecvTimestamp)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            And([Not(EventIs(RecvStopReqRep)), Not(EventIs(SendPortAbs)), EventIs(RecvTaggedMsg)])
            And([Not(EventIs(RecvStopReqRep)), Not(EventIs(SendTaggedMsg)), EventIs(SendTimestamp)])
            And([Not(EventIs(RecvStopReqRep)), Not(EventIs(SendTag)), EventIs(SendStopGrn)])
            And([Not(EventIs(SendStopGrn)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendPtag)])
            And([Not(EventIs(SendStopGrn)), Not(EventIs(RecvFedId)), EventIs(RecvFedId)])
            And([Not(EventIs(SendStopGrn)), Not(EventIs(SendTimestamp)), EventIs(SendStopReq)])
            And([Not(EventIs(SendStopGrn)), Not(EventIs(SendPortAbs)), EventIs(SendPortAbs)])
            And([Not(EventIs(SendStopGrn)), Not(EventIs(SendTaggedMsg)), TagNonzero])
            And([Not(EventIs(SendStopGrn)), Not(EventIs(SendTag)), EventIs(SendTag)])
            And([Not(EventIs(SendStopGrn)), Not(EventIs(RecvStopReqRep)), EventIs(RecvTimestamp)])
            And([Not(EventIs(RecvLtc)), Not(TagNonzero), EventIs(RecvLtc)])
            And([Not(EventIs(RecvLtc)), Not(EventIs(SendAck)), EventIs(SendTaggedMsg)])
            And([Not(EventIs(RecvLtc)), Not(EventIs(RecvNet)), EventIs(SendAck)])
            And([Not(EventIs(RecvLtc)), Not(EventIs(RecvPortAbs)), EventIs(RecvStopReqRep)])
            And([Not(EventIs(RecvLtc)), Not(EventIs(RecvTaggedMsg)), EventIs(RecvPortAbs)])
            And([Not(EventIs(RecvLtc)), Not(EventIs(SendStopReq)), TagFinite])
            And([Not(EventIs(RecvLtc)), Not(EventIs(SendStopGrn)), EventIs(RecvStopReq)])
            And([Not(TagFinite), EventIs(RecvNet)])
            And([Not(EventIs(SendTimestamp)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            And([Not(EventIs(RecvNet)), EventIs(RecvTaggedMsg)])
            And([Not(EventIs(SendPtag)), EventIs(SendTimestamp)])
            And([Not(EventIs(RecvTaggedMsg)), EventIs(SendStopGrn)])
            And([Not(EventIs(SendStopReq)), EventIs(SendPtag)])
            And([Not(EventIs(RecvLtc)), EventIs(RecvFedId)])
        "#]];
        expect.assert_eq(
            &predicates
                .iter()
                .step_by(100)
                .fold(String::new(), |a, b| a + &format!("{:?}\n", b.0)),
        );
        let expected_n = expect_test::expect!["23332"];
        expected_n.assert_eq(&predicates.len().to_string());
    }
}
