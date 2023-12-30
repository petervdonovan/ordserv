use std::collections::{HashMap, HashSet};

use crate::{
    enumerate::{Abstraction, ByFuel, ConcAbst, NaryRelation, PowerBool, SimpleAbstraction},
    BinaryRelation, EventKind, Predicate, Rule,
};

#[derive(Debug, Default, Clone)]
pub struct PredicateAbstraction {
    pub possible_events: Option<HashSet<EventKind>>,
    pub sabs: SimpleAbstraction<Predicate>,
}
#[derive(Debug)]
pub struct PredicatesWithBoundBinariesWithFuel(ByFuel<PredicateAbstraction>);
// #[derive(Debug, Default)]
// pub struct BinaryRelationsWithUnariesWithFuel(ByFuel<BinaryRelationAbstraction>);
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
}

impl Abstraction for PredicateAbstraction {
    type R = Predicate;
    fn fact(predicate: &Predicate) -> Self {
        match predicate {
            Predicate::EventIs(kind) => Self::event(*kind),
            _ => Self {
                possible_events: None,
                sabs: SimpleAbstraction::fact(predicate),
            },
        }
    }

    fn and(
        concterms: impl Iterator<Item = Self::R> + Clone,
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
        let sabs = SimpleAbstraction::and(concterms.clone(), absterms.map(|it| it.sabs))?;
        Some((
            Predicate::And(concterms.into_iter().collect::<Vec<_>>().into_boxed_slice()),
            Self {
                possible_events,
                sabs,
            },
        ))
    }

    fn or(
        concterms: impl Iterator<Item = Self::R> + Clone,
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
        let sabs = SimpleAbstraction::or(concterms.clone(), absterms.map(|it| it.sabs))?;
        Some((
            Predicate::Or(concterms.into_iter().collect::<Vec<_>>().into_boxed_slice()),
            Self {
                possible_events,
                sabs,
            },
        ))
    }

    fn not(&self, concterm: &Predicate) -> Option<ConcAbst<Self>> {
        if matches!(concterm, &Predicate::EventIs(_)) {
            // heuristic: usually is not helpful to match negations of eventis.
            // note: we still hit all equivalence classes because not(some event) is the same as or(all other events), except with undesirably much lower fuel
            return None;
        }
        let sabs = self.sabs.not(concterm)?;
        Some((
            Predicate::Not(Box::new(concterm.clone())),
            Self {
                possible_events: None,
                sabs,
            },
        ))
    }
    fn uninhabitable(&self) -> bool {
        self.possible_events
            .as_ref()
            .is_some_and(|it| it.is_empty())
            || self.sabs.uninhabitable()
    }
}

impl PredicateAbstraction {
    fn event(kind: EventKind) -> Self {
        Self {
            possible_events: Some(vec![kind].into_iter().collect()),
            sabs: SimpleAbstraction::default(),
        }
    }
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
