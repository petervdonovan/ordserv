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
                    let aband = PredicateAbstraction::and(conniter());
                    let abor = PredicateAbstraction::or(conniter());
                    if !aband.uninhabitable() {
                        ret.push((Predicate::And(bslice()), aband));
                    }
                    if !abor.uninhabitable() {
                        ret.push((Predicate::Or(bslice()), abor));
                    }
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
            Or([EventIs(RecvTimestamp), EventIs(SendTimestamp), TagNonzero])
            And([EventIs(SendPtag), TagFinite, FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            Or([EventIs(SendTaggedMsg), EventIs(SendPtag), EventIs(RecvTimestamp)])
            Or([EventIs(SendTag), EventIs(RecvNet), EventIs(RecvFedId)])
            Or([EventIs(RecvStopReq), EventIs(RecvPortAbs), EventIs(RecvNet)])
            Or([EventIs(SendStopReq), EventIs(SendPtag), EventIs(SendAck)])
            Or([EventIs(RecvStopReqRep), EventIs(RecvPortAbs), EventIs(SendTimestamp)])
            Or([EventIs(SendStopGrn), EventIs(RecvTimestamp), EventIs(RecvFedId)])
            Or([EventIs(SendStopGrn), EventIs(SendStopReq), EventIs(SendTag)])
            Or([EventIs(RecvLtc), EventIs(SendTag), TagNonzero])
            And([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), TagNonzero, EventIs(SendAck)])
            Or([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendAck), EventIs(SendPortAbs)])
            And([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(RecvPortAbs), TagNonzero])
            Or([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendTag), EventIs(SendTaggedMsg)])
            And([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(RecvLtc), TagFinite])
            And([Not(TagNonzero), TagFinite, EventIs(SendTag)])
            And([Not(TagNonzero), EventIs(RecvNet), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            Or([Not(TagNonzero), EventIs(SendTaggedMsg), EventIs(SendPtag)])
            Or([Not(TagNonzero), EventIs(RecvStopReqRep), TagNonzero])
            Or([Not(TagFinite), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(RecvLtc)])
            Or([Not(TagFinite), EventIs(SendAck), EventIs(RecvLtc)])
            And([Not(TagFinite), EventIs(RecvPortAbs), EventIs(RecvPortAbs)])
            Or([Not(TagFinite), EventIs(RecvStopReq), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            Or([Not(TagFinite), EventIs(RecvLtc), EventIs(SendTaggedMsg)])
            Or([Not(EventIs(RecvFedId)), TagFinite, EventIs(RecvNet)])
            Or([Not(EventIs(RecvFedId)), EventIs(RecvTimestamp), EventIs(SendTimestamp)])
            Or([Not(EventIs(RecvFedId)), EventIs(SendPtag), EventIs(RecvTaggedMsg)])
            Or([Not(EventIs(RecvFedId)), EventIs(SendStopReq), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            Or([Not(EventIs(SendAck)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(SendAck)])
            Or([Not(EventIs(SendAck)), TagFinite, EventIs(RecvStopReqRep)])
            Or([Not(EventIs(SendAck)), EventIs(RecvNet), TagNonzero])
            Or([Not(EventIs(SendAck)), EventIs(SendTaggedMsg), EventIs(SendPortAbs)])
            Or([Not(EventIs(SendAck)), EventIs(SendStopReq), EventIs(SendStopReq)])
            Or([Not(EventIs(SendTimestamp)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(SendTag)])
            Or([Not(EventIs(SendTimestamp)), EventIs(RecvFedId), EventIs(RecvPortAbs)])
            Or([Not(EventIs(SendTimestamp)), EventIs(RecvNet), EventIs(SendStopGrn)])
            Or([Not(EventIs(SendTimestamp)), EventIs(RecvTaggedMsg), EventIs(RecvFedId)])
            Or([Not(EventIs(SendTimestamp)), EventIs(RecvStopReqRep), EventIs(SendTaggedMsg)])
            Or([Not(EventIs(RecvTimestamp)), TagNonzero, EventIs(RecvFedId)])
            Or([Not(EventIs(RecvTimestamp)), EventIs(SendAck), EventIs(SendAck)])
            Or([Not(EventIs(RecvTimestamp)), EventIs(SendPortAbs), EventIs(RecvTaggedMsg)])
            Or([Not(EventIs(RecvTimestamp)), EventIs(SendTag), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            Or([Not(EventIs(RecvTimestamp)), EventIs(SendStopGrn), EventIs(RecvTimestamp)])
            Or([Not(EventIs(RecvNet)), TagNonzero, EventIs(RecvTaggedMsg)])
            Or([Not(EventIs(RecvNet)), EventIs(SendTimestamp), TagNonzero])
            Or([Not(EventIs(RecvNet)), EventIs(RecvPortAbs), EventIs(SendPortAbs)])
            Or([Not(EventIs(RecvNet)), EventIs(SendTag), EventIs(SendStopReq)])
            And([Not(EventIs(RecvNet)), EventIs(RecvLtc), TagFinite])
            Or([Not(EventIs(SendPortAbs)), TagFinite, TagFinite])
            Or([Not(EventIs(SendPortAbs)), EventIs(SendTimestamp), EventIs(SendStopGrn)])
            Or([Not(EventIs(SendPortAbs)), EventIs(SendPtag), EventIs(RecvFedId)])
            Or([Not(EventIs(SendPortAbs)), EventIs(RecvStopReq), EventIs(SendTaggedMsg)])
            Or([Not(EventIs(SendPortAbs)), EventIs(RecvLtc), EventIs(RecvLtc)])
            Or([Not(EventIs(RecvPortAbs)), TagFinite, EventIs(SendTaggedMsg)])
            Or([Not(EventIs(RecvPortAbs)), EventIs(RecvTimestamp), EventIs(RecvTaggedMsg)])
            Or([Not(EventIs(RecvPortAbs)), EventIs(SendTaggedMsg), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            Or([Not(EventIs(RecvPortAbs)), EventIs(SendStopReq), EventIs(RecvTimestamp)])
            Or([Not(EventIs(SendPtag)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(SendPortAbs)])
            Or([Not(EventIs(SendPtag)), EventIs(RecvFedId), TagNonzero])
            Or([Not(EventIs(SendPtag)), EventIs(RecvNet), EventIs(RecvNet)])
            Or([Not(EventIs(SendPtag)), EventIs(SendTaggedMsg), EventIs(SendStopReq)])
            And([Not(EventIs(SendPtag)), EventIs(RecvStopReqRep), TagFinite])
            Or([Not(EventIs(SendTaggedMsg)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(SendStopGrn)])
            Or([Not(EventIs(SendTaggedMsg)), EventIs(RecvFedId), EventIs(SendStopGrn)])
            Or([Not(EventIs(SendTaggedMsg)), EventIs(SendPortAbs), EventIs(RecvFedId)])
            Or([Not(EventIs(SendTaggedMsg)), EventIs(RecvTaggedMsg), EventIs(SendTaggedMsg)])
            Or([Not(EventIs(SendTaggedMsg)), EventIs(RecvStopReqRep), EventIs(RecvLtc)])
            Or([Not(EventIs(RecvTaggedMsg)), TagNonzero, EventIs(RecvNet)])
            Or([Not(EventIs(RecvTaggedMsg)), EventIs(SendAck), EventIs(RecvTaggedMsg)])
            Or([Not(EventIs(RecvTaggedMsg)), EventIs(RecvPortAbs), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            Or([Not(EventIs(RecvTaggedMsg)), EventIs(SendTag), EventIs(RecvTimestamp)])
            Or([Not(EventIs(RecvTaggedMsg)), EventIs(SendStopGrn), EventIs(RecvStopReq)])
            Or([Not(EventIs(SendTag)), TagNonzero, EventIs(RecvStopReqRep)])
            Or([Not(EventIs(SendTag)), EventIs(SendTimestamp), EventIs(RecvNet)])
            Or([Not(EventIs(SendTag)), EventIs(RecvPortAbs), EventIs(SendStopReq)])
            And([Not(EventIs(SendTag)), EventIs(RecvStopReq), TagFinite])
            Or([Not(EventIs(SendTag)), EventIs(RecvLtc), EventIs(RecvPortAbs)])
            Or([Not(EventIs(RecvStopReq)), TagFinite, EventIs(RecvTimestamp)])
            Or([Not(EventIs(RecvStopReq)), EventIs(RecvTimestamp), EventIs(RecvFedId)])
            Or([Not(EventIs(RecvStopReq)), EventIs(SendPtag), EventIs(SendPtag)])
            Or([Not(EventIs(RecvStopReq)), EventIs(RecvStopReq), EventIs(RecvLtc)])
            Or([Not(EventIs(SendStopReq)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(RecvFedId)])
            Or([Not(EventIs(SendStopReq)), TagFinite, EventIs(SendStopReq)])
            Or([Not(EventIs(SendStopReq)), EventIs(RecvNet), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            Or([Not(EventIs(SendStopReq)), EventIs(SendTaggedMsg), EventIs(RecvTimestamp)])
            Or([Not(EventIs(SendStopReq)), EventIs(SendStopReq), EventIs(RecvStopReq)])
            Or([Not(EventIs(RecvStopReqRep)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(RecvTaggedMsg)])
            Or([Not(EventIs(RecvStopReqRep)), EventIs(RecvFedId), EventIs(RecvNet)])
            Or([Not(EventIs(RecvStopReqRep)), EventIs(RecvNet), EventIs(SendStopReq)])
            And([Not(EventIs(RecvStopReqRep)), EventIs(RecvTaggedMsg), TagFinite])
            Or([Not(EventIs(RecvStopReqRep)), EventIs(RecvStopReqRep), EventIs(RecvPortAbs)])
            Or([Not(EventIs(SendStopGrn)), TagNonzero, TagFinite])
            Or([Not(EventIs(SendStopGrn)), EventIs(SendAck), EventIs(RecvFedId)])
            Or([Not(EventIs(SendStopGrn)), EventIs(SendPortAbs), EventIs(SendPtag)])
            Or([Not(EventIs(SendStopGrn)), EventIs(RecvTaggedMsg), EventIs(RecvLtc)])
            Or([Not(EventIs(SendStopGrn)), EventIs(SendStopGrn), EventIs(SendAck)])
            Or([Not(EventIs(RecvLtc)), TagNonzero, EventIs(SendTaggedMsg)])
            Or([Not(EventIs(RecvLtc)), EventIs(SendTimestamp), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            Or([Not(EventIs(RecvLtc)), EventIs(RecvPortAbs), EventIs(RecvTimestamp)])
            Or([Not(EventIs(RecvLtc)), EventIs(SendTag), EventIs(SendTag)])
            And([Not(EventIs(RecvLtc)), EventIs(RecvLtc), TagNonzero])
            Or([Not(TagFinite), Not(TagNonzero), EventIs(SendAck)])
            Or([Not(EventIs(RecvFedId)), Not(TagNonzero), EventIs(SendStopGrn)])
            And([Not(EventIs(SendAck)), Not(TagNonzero), EventIs(RecvTaggedMsg)])
            And([Not(EventIs(SendTimestamp)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(RecvTimestamp)])
            And([Not(EventIs(SendTimestamp)), Not(EventIs(RecvFedId)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            Or([Not(EventIs(RecvTimestamp)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(RecvTaggedMsg)])
            Or([Not(EventIs(RecvTimestamp)), Not(EventIs(RecvFedId)), EventIs(RecvTimestamp)])
            Or([Not(EventIs(RecvTimestamp)), Not(EventIs(SendTimestamp)), EventIs(RecvLtc)])
            And([Not(EventIs(RecvNet)), Not(TagFinite), EventIs(SendTag)])
            And([Not(EventIs(RecvNet)), Not(EventIs(SendTimestamp)), EventIs(RecvTimestamp)])
            Or([Not(EventIs(SendPortAbs)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(RecvLtc)])
            Or([Not(EventIs(SendPortAbs)), Not(EventIs(RecvFedId)), EventIs(RecvTaggedMsg)])
            Or([Not(EventIs(SendPortAbs)), Not(EventIs(RecvTimestamp)), EventIs(SendTimestamp)])
            And([Not(EventIs(RecvPortAbs)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(RecvLtc)])
            And([Not(EventIs(RecvPortAbs)), Not(EventIs(RecvFedId)), EventIs(RecvTaggedMsg)])
            And([Not(EventIs(RecvPortAbs)), Not(EventIs(RecvTimestamp)), EventIs(SendTimestamp)])
            And([Not(EventIs(RecvPortAbs)), Not(EventIs(SendPortAbs)), EventIs(SendStopGrn)])
            Or([Not(EventIs(SendPtag)), Not(TagFinite), EventIs(SendTaggedMsg)])
            Or([Not(EventIs(SendPtag)), Not(EventIs(SendTimestamp)), EventIs(SendAck)])
            Or([Not(EventIs(SendPtag)), Not(EventIs(RecvNet)), EventIs(RecvStopReqRep)])
            And([Not(EventIs(SendTaggedMsg)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendPtag)])
            And([Not(EventIs(SendTaggedMsg)), Not(EventIs(RecvFedId)), EventIs(SendAck)])
            And([Not(EventIs(SendTaggedMsg)), Not(EventIs(SendTimestamp)), EventIs(RecvStopReqRep)])
            And([Not(EventIs(SendTaggedMsg)), Not(EventIs(SendPortAbs)), EventIs(RecvPortAbs)])
            Or([Not(EventIs(RecvTaggedMsg)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), TagFinite])
            Or([Not(EventIs(RecvTaggedMsg)), Not(TagFinite), EventIs(SendStopReq)])
            Or([Not(EventIs(RecvTaggedMsg)), Not(EventIs(SendTimestamp)), EventIs(SendPortAbs)])
            Or([Not(EventIs(RecvTaggedMsg)), Not(EventIs(SendPortAbs)), TagNonzero])
            Or([Not(EventIs(RecvTaggedMsg)), Not(EventIs(SendPtag)), EventIs(SendTag)])
            Or([Not(EventIs(SendTag)), Not(TagNonzero), EventIs(RecvNet)])
            And([Not(EventIs(SendTag)), Not(EventIs(SendAck)), TagNonzero])
            And([Not(EventIs(SendTag)), Not(EventIs(RecvTimestamp)), EventIs(SendTag)])
            And([Not(EventIs(SendTag)), Not(EventIs(RecvPortAbs)), EventIs(RecvTimestamp)])
            And([Not(EventIs(SendTag)), Not(EventIs(SendTaggedMsg)), EventIs(RecvLtc)])
            And([Not(EventIs(RecvStopReq)), Not(TagNonzero), EventIs(RecvTaggedMsg)])
            Or([Not(EventIs(RecvStopReq)), Not(EventIs(SendAck)), EventIs(SendTimestamp)])
            Or([Not(EventIs(RecvStopReq)), Not(EventIs(RecvTimestamp)), EventIs(SendStopGrn)])
            Or([Not(EventIs(RecvStopReq)), Not(EventIs(RecvPortAbs)), EventIs(SendPtag)])
            Or([Not(EventIs(RecvStopReq)), Not(EventIs(RecvTaggedMsg)), EventIs(RecvFedId)])
            And([Not(EventIs(SendStopReq)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(RecvStopReqRep)])
            And([Not(EventIs(SendStopReq)), Not(EventIs(RecvFedId)), EventIs(SendPtag)])
            And([Not(EventIs(SendStopReq)), Not(EventIs(RecvTimestamp)), EventIs(RecvFedId)])
            And([Not(EventIs(SendStopReq)), Not(EventIs(SendPortAbs)), EventIs(SendStopReq)])
            And([Not(EventIs(SendStopReq)), Not(EventIs(SendTaggedMsg)), EventIs(SendPortAbs)])
            And([Not(EventIs(SendStopReq)), Not(EventIs(RecvStopReq)), TagNonzero])
            And([Not(EventIs(RecvStopReqRep)), Not(TagNonzero), EventIs(RecvStopReq)])
            Or([Not(EventIs(RecvStopReqRep)), Not(EventIs(SendAck)), EventIs(RecvNet)])
            Or([Not(EventIs(RecvStopReqRep)), Not(EventIs(RecvNet)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            Or([Not(EventIs(RecvStopReqRep)), Not(EventIs(RecvPortAbs)), EventIs(RecvTaggedMsg)])
            Or([Not(EventIs(RecvStopReqRep)), Not(EventIs(RecvTaggedMsg)), EventIs(SendTimestamp)])
            Or([Not(EventIs(RecvStopReqRep)), Not(EventIs(RecvStopReq)), EventIs(SendStopGrn)])
            Or([Not(EventIs(SendStopGrn)), Not(TagNonzero), EventIs(SendTaggedMsg)])
            And([Not(EventIs(SendStopGrn)), Not(EventIs(SendAck)), EventIs(SendTimestamp)])
            And([Not(EventIs(SendStopGrn)), Not(EventIs(RecvTimestamp)), EventIs(SendStopGrn)])
            And([Not(EventIs(SendStopGrn)), Not(EventIs(RecvPortAbs)), EventIs(SendPtag)])
            And([Not(EventIs(SendStopGrn)), Not(EventIs(RecvTaggedMsg)), EventIs(RecvFedId)])
            And([Not(EventIs(SendStopGrn)), Not(EventIs(RecvStopReq)), EventIs(SendStopReq)])
            Or([Not(EventIs(RecvLtc)), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendPortAbs)])
            Or([Not(EventIs(RecvLtc)), Not(EventIs(RecvFedId)), TagFinite])
            Or([Not(EventIs(RecvLtc)), Not(EventIs(SendTimestamp)), EventIs(RecvStopReq)])
            Or([Not(EventIs(RecvLtc)), Not(EventIs(SendPortAbs)), EventIs(RecvNet)])
            Or([Not(EventIs(RecvLtc)), Not(EventIs(SendTaggedMsg)), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
            Or([Not(EventIs(RecvLtc)), Not(EventIs(SendTag)), EventIs(RecvTaggedMsg)])
            Or([Not(EventIs(RecvLtc)), Not(EventIs(RecvStopReqRep)), EventIs(SendTimestamp)])
            And([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(RecvLtc)])
            And([Not(EventIs(RecvFedId)), EventIs(RecvTaggedMsg)])
            And([Not(EventIs(RecvTimestamp)), EventIs(SendTimestamp)])
            And([Not(EventIs(SendPortAbs)), EventIs(SendStopGrn)])
            And([Not(EventIs(SendTaggedMsg)), EventIs(SendPtag)])
            And([Not(EventIs(RecvStopReq)), EventIs(RecvFedId)])
            And([Not(EventIs(RecvStopReqRep)), EventIs(SendStopReq)])
        "#]];
        expect.assert_eq(
            &predicates
                .iter()
                .step_by(100)
                .fold(String::new(), |a, b| a + &format!("{:?}\n", b.0)),
        );
        let expected_n = expect_test::expect!["17684"];
        expected_n.assert_eq(&predicates.len().to_string());
    }
}
