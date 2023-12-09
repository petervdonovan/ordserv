use crate::{conninfo::Tag, EventKind, Rule};

use crate::BinaryRelation::{
    And, FederateEquals, FederateZeroDelayDirectlyUpstreamOf, TagPlusDelay2FedEquals,
    TagPlusDelay2FedLessThan, TagPlusDelay2FedLessThanOrEqual,
    TagPlusDelayFromAllImmUpstreamFedsLessThan, TagStrictPlusDelayFromAllImmUpstreamFedsLessThan,
    TagStrictPlusDelayFromSomeImmUpstreamFedGreaterThanOrEquals, Unary,
};
use crate::Predicate::*;
use crate::{BinaryRelation, Predicate};
use crate::{Event, EventKind::*};
pub fn axioms() -> Vec<Rule> {
    vec![
        // // The following are for LTCs.
        // Rule {
        //     preceding_event: EventKind::RecvTaggedMsg,
        //     event: EventKind::RecvLtc,
        //     relations: vec![
        //         Relation::TagPlusDelayLessThanOrEqual,
        //         Relation::FederateEqual,
        //     ],
        // },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(RecvTaggedMsg))),
                FederateEquals,
                TagPlusDelay2FedLessThanOrEqual,
            ])),
            event: EventIs(RecvLtc),
        }, // Rule {
        //     preceding_event: EventKind::RecvPortAbs,
        //     event: EventKind::RecvLtc,
        //     relations: vec![
        //         Relation::TagPlusDelayLessThanOrEqual,
        //         Relation::FederateEqual,
        //     ],
        // },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(RecvPortAbs))),
                FederateEquals,
                TagPlusDelay2FedLessThanOrEqual,
            ])),
            event: EventIs(RecvLtc),
        },
        // Rule {
        //     preceding_event: EventKind::RecvLtc,
        //     event: EventKind::RecvFirstPortAbsOrTaggedMsg,
        //     relations: vec![Relation::TagPlusDelayLessThan, Relation::FederateEqual],
        // },
        // Rule {
        //     preceding_event: And(Box::new([
        //         Unary(Box::new(EventIs(RecvLtc))),
        //         // Unary(Box::new(TagNonzero)),
        //         FederateEquals,
        //         TagStrictPlusDelayFromAllImmUpstreamFedsLessThan,
        //     ])),
        //     event: EventIs(RecvPortAbs),
        // }, // ditto to below
        // Rule {
        //     preceding_event: And(Box::new([
        //         Unary(Box::new(EventIs(RecvLtc))),
        //         FederateEquals,
        //         TagPlusDelayFromAllImmUpstreamFedsLessThan,
        //     ])),
        //     event: EventIs(RecvTaggedMsg),
        // }, // disabled because it isn't true: the conninfo records the shortest delay, but there could be longer delays that put messages arbitrarily far into the future -- even though the full structure of the program, which is not yet encoded in the conninfo, would not allow that.
        // // The following are for handling of NETs.
        // Rule {
        //     preceding_event: EventKind::FirstRecvNetOrSendStopGrnOrRecvLtc,
        //     event: EventKind::SendFirstTagOrPtag,
        //     relations: vec![
        //         Relation::TagPlusDelayEqual,
        //         Relation::FederateEqual,
        //         Relation::TagFinite,
        //         Relation::FirstTagNonzero,
        //     ],
        // },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(IsFirst(Box::new(Or(Box::new([
                    EventIs(SendStopGrn),
                    EventIs(EventKind::RecvLtc),
                    EventIs(RecvNet),
                ])))))),
                FederateEquals,
                TagPlusDelay2FedEquals,
                Unary(Box::new(TagFinite)),
            ])),
            event: Predicate::And(Box::new([
                Or(Box::new([EventIs(SendTag), EventIs(SendPtag)])),
                // TagNonzero, // ???
            ])),
        },
        // Rule {
        //     preceding_event: EventKind::RecvLtc,
        //     event: EventKind::RecvNet,
        //     relations: vec![Relation::TagPlusDelayLessThan, Relation::FederateEqual],
        // },
        // Rule {
        //     preceding_event: And(Box::new([
        //         Unary(Box::new(EventIs(RecvLtc))),
        //         FederateEquals,
        //         TagPlusDelay2FedLessThan,
        //     ])),
        //     event: EventIs(RecvNet),
        // },  // Not true cuz nets are not monotonic.
        // Rule {
        //     preceding_event: EventKind::SendPtag,
        //     event: EventKind::RecvLtc,
        //     relations: vec![
        //         Relation::TagPlusDelayLessThanOrEqual,
        //         Relation::FederateEqual,
        //     ],
        // },
        Rule {
            preceding_event: BinaryRelation::IsFirst(Box::new(And(Box::new([
                Unary(Box::new(Or(Box::new([
                    EventIs(SendPtag),
                    EventIs(SendTag),
                ])))),
                FederateEquals,
                TagPlusDelay2FedLessThanOrEqual,
            ])))),
            event: EventIs(RecvLtc),
        },
        // Rule {
        //     preceding_event: EventKind::SendTag,
        //     event: EventKind::RecvLtc,
        //     relations: vec![
        //         Relation::TagPlusDelayLessThanOrEqual,
        //         Relation::FederateEqual,
        //     ],
        // },
        // Rule {
        //     preceding_event: And(Box::new([
        //         Unary(Box::new(EventIs(SendTag))),
        //         FederateEquals,
        //         TagPlusDelay2FedLessThanOrEqual,
        //     ])),
        //     event: EventIs(RecvLtc),
        // }, // subsumed by the previous rule.
        // // The following is for handling of PTAGs and TAGs.
        // Rule {
        //     preceding_event: EventKind::SendPtag,
        //     event: EventKind::SendTag,
        //     relations: vec![
        //         Relation::TagPlusDelayLessThanOrEqual,
        //         Relation::FederateEqual,
        //     ],
        // },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(SendPtag))),
                FederateEquals,
                TagPlusDelay2FedLessThanOrEqual,
            ])),
            event: EventIs(SendTag),
        },
        // Rule {
        //     preceding_event: EventKind::SendFirstTagOrPtag,
        //     event: EventKind::RecvFirstPortAbsOrTaggedMsg,
        //     relations: vec![Relation::TagPlusDelayEqual, Relation::FederateEqual],
        // },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(IsFirst(Box::new(Or(Box::new([
                    EventIs(SendTag),
                    EventIs(SendPtag),
                ])))))),
                FederateEquals,
                TagPlusDelay2FedEquals,
            ])),
            event: Or(Box::new([EventIs(RecvPortAbs), EventIs(RecvTaggedMsg)])),
        },
        // // The following encode the startup sequence in which a federate connects to RTI.
        // Rule {
        //     preceding_event: EventKind::RecvFedId,
        //     event: EventKind::SendAck,
        //     relations: vec![Relation::FederateEqual],
        // },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(RecvFedId))),
                FederateEquals,
            ])),
            event: EventIs(SendAck),
        },
        // Rule {
        //     preceding_event: EventKind::SendAck,
        //     event: EventKind::RecvTimestamp,
        //     relations: vec![Relation::FederateEqual],
        // },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(SendAck))),
                FederateEquals,
            ])),
            event: EventIs(RecvTimestamp),
        },
        // Rule {
        //     preceding_event: EventKind::RecvTimestamp,
        //     event: EventKind::SendTimestamp,
        //     relations: vec![Relation::FederateEqual],
        // },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(RecvTimestamp))),
                FederateEquals,
            ])),
            event: EventIs(SendTimestamp),
        },
        // Rule {
        //     preceding_event: EventKind::SendTimestamp,
        //     event: EventKind::RecvNet,
        //     relations: vec![Relation::FederateEqual],
        // },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(SendTimestamp))),
                FederateEquals,
            ])),
            event: EventIs(RecvNet),
        },
    ]
}
