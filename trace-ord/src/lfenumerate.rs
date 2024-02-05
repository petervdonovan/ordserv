use std::collections::HashSet;

use crate::conninfo::{ConnInfo, FedId};
use crate::enumerate::{Abstraction, ByFuel, Conc, ConcAbst, SimpleAbstraction};
use crate::lflib::{BinaryRelationAtom, EventKind, Rule, UnaryRelation, UnaryRelationAtom};

#[derive(Debug, Default, Clone)]
pub struct UnaryRelationAbstraction {
    pub possible_events: Option<HashSet<EventKind>>,
    pub sabs: SimpleAbstraction<UnaryRelationAbstraction>,
}
#[derive(Debug)]
pub struct UnaryRelationsWithBoundBinariesWithFuel(ByFuel<UnaryRelationAbstraction>);
#[derive(Debug, Default)]
pub struct RulesWithFuel {
    rules_by_fuel: Vec<Vec<Rule>>,
}

impl Abstraction for UnaryRelationAbstraction {
    type AtomN = UnaryRelationAtom;

    type AtomM = BinaryRelationAtom;

    type ConcEvent = crate::lflib::ConcEvent;

    const N: usize = 1;

    const M: usize = 2;

    type Ctx = ConnInfo;

    type ProjectTo = FedId;

    type Atom1 = UnaryRelationAtom;

    type Atom2 = BinaryRelationAtom;

    fn fact(predicate: &UnaryRelation) -> Self {
        match predicate {
            UnaryRelation::Atom(UnaryRelationAtom::EventIs(kind)) => Self::event(*kind),
            _ => Self {
                possible_events: None,
                sabs: SimpleAbstraction::fact(predicate),
            },
        }
    }

    fn and(
        concterms: impl Iterator<Item = Conc<Self>> + Clone,
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
        let ret = Self {
            possible_events,
            sabs,
        };
        if ret.uninhabitable() {
            return None;
        }
        Some((
            UnaryRelation::And(concterms.into_iter().collect::<Vec<_>>().into_boxed_slice()),
            ret,
        ))
    }

    fn or(
        concterms: impl Iterator<Item = Conc<Self>> + Clone,
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
            UnaryRelation::Or(concterms.into_iter().collect::<Vec<_>>().into_boxed_slice()),
            Self {
                possible_events,
                sabs,
            },
        ))
    }

    fn not(&self, concterm: &UnaryRelation) -> Option<ConcAbst<Self>> {
        if matches!(
            concterm,
            &UnaryRelation::Atom(UnaryRelationAtom::EventIs(_))
        ) {
            // heuristic: usually is not helpful to match negations of eventis.
            // note: we still hit all equivalence classes because not(some event) is the same as or(all other events), except with undesirably much lower fuel
            return None;
        }
        let sabs = self.sabs.not(concterm)?;
        Some((
            UnaryRelation::Not(Box::new(concterm.clone())),
            Self {
                possible_events: None,
                sabs,
            },
        ))
    }
}

impl UnaryRelationAbstraction {
    fn uninhabitable(&self) -> bool {
        self.possible_events
            .as_ref()
            .is_some_and(|it| it.is_empty())
            || self.sabs.uninhabitable()
    }
}

impl UnaryRelationAbstraction {
    fn event(kind: EventKind) -> Self {
        Self {
            possible_events: Some(vec![kind].into_iter().collect()),
            sabs: SimpleAbstraction::default(),
        }
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_predicates_with_fuel() {
        let mut predicates = ByFuel::<UnaryRelationAbstraction>::default();
        let predicates: Vec<_> = predicates.advance(5).collect();
        // TODO: account for implications between atomic predicates. e.g., if a implies b, then or(a, b) is equivalent to b.
        let expect = expect_test::expect![[r#"
            Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)
            Or([And([Atom(EventIs(SendTaggedMsg))]), Atom(EventIs(RecvStopReq)), Atom(EventIs(RecvTimestamp))])
            Or([And([Atom(EventIs(RecvTimestamp)), Atom(TagNonzero), Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)]), Atom(EventIs(SendTag)), Atom(EventIs(SendTimestamp))])
            And([Or([Atom(EventIs(SendTaggedMsg)), Atom(EventIs(SendAck)), Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)]), Atom(EventIs(SendTaggedMsg)), Atom(EventIs(SendTaggedMsg))])
            And([Or([Atom(EventIs(RecvStopReq)), Atom(EventIs(SendTaggedMsg)), Atom(EventIs(RecvTimestamp))]), Atom(TagFinite), Atom(TagFinite)])
            Or([And([Atom(EventIs(SendStopGrn)), Atom(TagFinite), Atom(TagNonzero)]), Atom(EventIs(SendTimestamp)), Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)])
            And([Or([Not(Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)), Atom(TagNonzero), Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)]), Atom(EventIs(SendPtag)), Atom(TagFinite)])
            Or([And([Not(Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)), Atom(TagFinite), Atom(EventIs(SendPortAbs))]), Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)])
            And([Or([Not(Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)), Atom(EventIs(RecvTimestamp)), Atom(EventIs(RecvStopReq))]), Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)])
            And([Or([Not(Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)), Atom(EventIs(SendTag)), Atom(EventIs(SendPtag))]), Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), Atom(TagFinite)])
            Or([And([Not(Atom(TagNonzero)), Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), Atom(EventIs(SendTimestamp))]), Atom(EventIs(RecvNet)), Atom(EventIs(RecvPortAbs))])
            Or([And([Not(Atom(TagNonzero)), Atom(TagFinite), Atom(EventIs(SendTaggedMsg))]), Atom(EventIs(RecvNet)), Atom(EventIs(SendTaggedMsg))])
            Or([And([Not(Atom(TagNonzero)), Atom(EventIs(RecvNet)), Atom(EventIs(RecvNet))]), Atom(EventIs(SendStopReq)), Atom(EventIs(SendTag))])
            Or([And([Not(Atom(TagNonzero)), Atom(EventIs(RecvStopReq)), Atom(TagFinite)]), Atom(EventIs(SendStopGrn)), Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)])
            Or([And([Not(Atom(TagFinite)), Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag), Atom(EventIs(SendPortAbs))]), Atom(EventIs(SendStopReq)), Atom(TagNonzero)])
            Or([And([Not(Atom(TagFinite)), Atom(TagNonzero), Atom(EventIs(RecvStopReqRep))]), Atom(EventIs(SendPortAbs)), Atom(TagNonzero)])
            Or([And([Not(Atom(TagFinite)), Atom(EventIs(SendPortAbs)), Atom(EventIs(SendPortAbs))]), Atom(EventIs(RecvFedId)), Atom(TagNonzero)])
            Or([And([Not(Atom(TagFinite)), Atom(EventIs(SendStopReq)), Atom(TagNonzero)]), Atom(EventIs(RecvTimestamp)), Atom(EventIs(SendPortAbs))])
            Or([And([Not(Atom(TagNonzero)), Not(Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)), Atom(EventIs(SendTag))]), Atom(TagNonzero), Atom(EventIs(RecvLtc))])
            Or([And([Not(Atom(TagFinite)), Not(Atom(TagNonzero)), Atom(EventIs(RecvFedId))]), Atom(EventIs(SendStopReq)), Atom(EventIs(SendPtag))])
            Or([And([Not(Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)), Atom(EventIs(SendTaggedMsg))]), Atom(EventIs(SendPortAbs)), Atom(EventIs(SendPtag))])
            Or([And([Not(Atom(TagFinite)), Atom(FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag)]), Atom(TagNonzero), Atom(EventIs(SendPtag))])
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
