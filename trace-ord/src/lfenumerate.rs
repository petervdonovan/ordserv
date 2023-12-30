use std::collections::{HashMap, HashSet};

use crate::{
    enumerate::{Abstraction, ByFuel, ConcAbst, NaryRelation, NaryRelationKind, PowerBool},
    BinaryRelation, EventKind, Predicate, Rule,
};

#[derive(Debug, Default, Clone)]
pub struct PredicateAbstraction {
    pub possible_events: Option<HashSet<EventKind>>,
    pub predicate2powerbool: HashMap<Predicate, PowerBool>,
}
#[derive(Debug)]
pub struct PredicatesWithBoundBinariesWithFuel(ByFuel<PredicateAbstraction>);
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

impl NaryRelation for Predicate {
    fn atoms() -> Vec<Self>
    where
        Self: std::marker::Sized,
    {
        let mut ret = vec![
            Predicate::FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag,
            Predicate::TagNonzero,
            Predicate::TagFinite,
        ];
        for kind in enum_iterator::all::<EventKind>() {
            ret.push(Predicate::EventIs(kind));
        }
        ret
    }

    fn kind(&self) -> crate::enumerate::NaryRelationKind {
        match self {
            Predicate::And(_) => crate::enumerate::NaryRelationKind::And,
            Predicate::Or(_) => crate::enumerate::NaryRelationKind::Or,
            Predicate::Not(_) => crate::enumerate::NaryRelationKind::Not,
            _ => crate::enumerate::NaryRelationKind::Other,
        }
    }

    // fn and(terms: Box<[Self]>) -> Self {
    //     Predicate::And(terms)
    // }

    // fn or(terms: Box<[Self]>) -> Self {
    //     Predicate::Or(terms)
    // }

    // fn not(&self) -> Self {
    //     Predicate::Not(Box::new(self.clone()))
    // }
}

impl Abstraction for PredicateAbstraction {
    type R = Predicate;
    fn fact(predicate: &Predicate) -> Self {
        match predicate {
            Predicate::EventIs(kind) => Self::event(*kind),
            _ => Self {
                possible_events: None,
                predicate2powerbool: vec![(predicate.clone(), PowerBool::new_true())]
                    .into_iter()
                    .collect(),
            },
        }
    }

    fn and(
        concterms: impl Fn() -> Box<[Self::R]>,
        absterms: impl Iterator<Item = Self> + Clone,
    ) -> Option<ConcAbst<Self>> {
        let possible_events: Option<HashSet<EventKind>> = absterms
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
            absterms.fold(HashMap::<Predicate, PowerBool>::default(), |mut acc, it| {
                for (predicate, powerbool) in it.predicate2powerbool.iter() {
                    let entry = acc.entry(predicate.clone()).or_default();
                    entry.and(powerbool);
                }
                acc
            });
        Some((
            Predicate::And(concterms()),
            Self {
                possible_events,
                predicate2powerbool,
            },
        ))
    }

    fn or(
        concterms: impl Fn() -> Box<[Self::R]>,
        absterms: impl Iterator<Item = Self> + Clone,
    ) -> Option<ConcAbst<Self>> {
        let possible_events: Option<HashSet<EventKind>> = absterms
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
            absterms.fold(HashMap::<Predicate, PowerBool>::default(), |mut acc, it| {
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
        Some((
            Predicate::Or(concterms()),
            Self {
                possible_events,
                predicate2powerbool,
            },
        ))
    }

    fn not(&self, concterm: &Predicate) -> Option<ConcAbst<Self>> {
        if matches!(concterm, &Predicate::EventIs(_)) {
            // heuristic: usually is not helpful to match negations of eventis.
            // note: we still hit all equivalence classes because not(some event) is the same as or(all other events), except with undesirably much lower fuel
            return None;
        }
        let predicate2powerbool = self
            .predicate2powerbool
            .iter()
            .map(|(predicate, powerbool)| (predicate.clone(), powerbool.not()))
            .collect();
        Some((
            Predicate::Not(Box::new(concterm.clone())),
            Self {
                possible_events: None,
                predicate2powerbool,
            },
        ))
    }
    fn uninhabitable(&self) -> bool {
        self.possible_events
            .as_ref()
            .is_some_and(|it| it.is_empty())
            || self
                .predicate2powerbool
                .iter()
                .any(|(_, pb)| pb.uninhabitable())
    }
}

impl PredicateAbstraction {
    fn event(kind: EventKind) -> Self {
        Self {
            possible_events: Some(vec![kind].into_iter().collect()),
            predicate2powerbool: HashMap::default(),
        }
    }
    // pub fn and(terms: impl Iterator<Item = PredicateAbstraction> + Clone) -> Self {

    // }
    // pub fn or(terms: impl Iterator<Item = PredicateAbstraction> + Clone) -> Self {

    // }
    // pub fn not(&self) -> Self {

    // }
    // pub fn uninhabitable(&self) -> bool {
    //     self.possible_events
    //         .as_ref()
    //         .is_some_and(|it| it.is_empty())
    //         || self
    //             .predicate2powerbool
    //             .iter()
    //             .any(|(_, pb)| pb.uninhabitable())
    // }
}

mod tests {
    use super::*;

    #[test]
    fn test_predicates_with_fuel() {
        let mut predicates = ByFuel::<PredicateAbstraction>::default();
        let predicates: Vec<_> = predicates.advance(5).collect();
        // TODO: account for implications between atomic predicates. e.g., if a implies b, then or(a, b) is equivalent to b.
        let expect = expect_test::expect![[r#"
          FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag
          Or([And([EventIs(SendTaggedMsg)]), EventIs(RecvStopReq), EventIs(RecvTimestamp)])
          Or([And([EventIs(RecvTimestamp), TagNonzero, FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag]), EventIs(SendTag), EventIs(SendTimestamp)])
          And([Or([EventIs(SendTaggedMsg), EventIs(SendAck), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag]), EventIs(SendTaggedMsg), EventIs(SendTaggedMsg)])
          And([Or([EventIs(RecvStopReq), EventIs(SendTaggedMsg), EventIs(RecvTimestamp)]), TagFinite, TagFinite])
          Or([And([EventIs(SendStopGrn), TagFinite, TagNonzero]), EventIs(SendTimestamp), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
          And([Or([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), TagNonzero, FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag]), EventIs(SendPtag), TagFinite])
          Or([And([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), TagFinite, EventIs(SendPortAbs)]), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
          And([Or([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(RecvTimestamp), EventIs(RecvStopReq)]), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
          And([Or([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendTag), EventIs(SendPtag)]), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, TagFinite])
          Or([And([Not(TagNonzero), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(SendTimestamp)]), EventIs(RecvNet), EventIs(RecvPortAbs)])
          Or([And([Not(TagNonzero), TagFinite, EventIs(SendTaggedMsg)]), EventIs(RecvNet), EventIs(SendTaggedMsg)])
          Or([And([Not(TagNonzero), EventIs(RecvNet), EventIs(RecvNet)]), EventIs(SendStopReq), EventIs(SendTag)])
          Or([And([Not(TagNonzero), EventIs(RecvStopReq), TagFinite]), EventIs(SendStopGrn), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag])
          Or([And([Not(TagFinite), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag, EventIs(SendPortAbs)]), EventIs(SendStopReq), TagNonzero])
          Or([And([Not(TagFinite), TagNonzero, EventIs(RecvStopReqRep)]), EventIs(SendPortAbs), TagNonzero])
          Or([And([Not(TagFinite), EventIs(SendPortAbs), EventIs(SendPortAbs)]), EventIs(RecvFedId), TagNonzero])
          Or([And([Not(TagFinite), EventIs(SendStopReq), TagNonzero]), EventIs(RecvTimestamp), EventIs(SendPortAbs)])
          Or([And([Not(TagNonzero), Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendTag)]), TagNonzero, EventIs(RecvLtc)])
          Or([And([Not(TagFinite), Not(TagNonzero), EventIs(RecvFedId)]), EventIs(SendStopReq), EventIs(SendPtag)])
          Or([And([Not(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), EventIs(SendTaggedMsg)]), EventIs(SendPortAbs), EventIs(SendPtag)])
          Or([And([Not(TagFinite), FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag]), TagNonzero, EventIs(SendPtag)])
      "#]];
        expect.assert_eq(
            &predicates
                .iter()
                .step_by(10000)
                .fold(String::new(), |a, b| a + &format!("{:?}\n", b.0)),
        );
        let expected_n = expect_test::expect!["217178"];
        expected_n.assert_eq(&predicates.len().to_string());
    }
}
