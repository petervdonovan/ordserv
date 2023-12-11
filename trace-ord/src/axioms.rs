use std::f32::consts::E;

use crate::conninfo::Tag;
use crate::{Event, EventKind, Rule};

use crate::BinaryRelation::{
    And, FederateDirectlyUpstreamOf, FederateEquals, FederateZeroDelayDirectlyUpstreamOf,
    TagEquals, TagGreaterThanOrEqual, TagLessThan, TagLessThanOrEqual, TagPlusDelay2FedEquals,
    TagPlusDelay2FedGreaterThanOrEquals, TagPlusDelay2FedLessThan, TagPlusDelay2FedLessThanOrEqual,
    TagPlusLargestDelayGreaterThanOrEqual, TagPlusLargestDelayLessThan,
    TagPlusLargestDelayLessThanOrEqual, TagStrictPlusDelay2FedLessThan,
    TagStrictPlusDelayFromSomeImmUpstreamFedGreaterThanOrEquals, Unary,
};
use crate::EventKind::*;
use crate::Predicate::*;
use crate::{BinaryRelation, Predicate};
pub fn axioms() -> Vec<Rule> {
    vec![
        // The following are for LTCs.
        Rule {
            // LTCs to the same federate are monotonic
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(RecvLtc))),
                FederateEquals,
                TagLessThan,
            ])),
            event: EventIs(RecvLtc),
        },
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
        // The following are for handling of NETs.
        Rule {
            // you should have received a net or a ltc or something that lets you know that a TAG or PTAG is needed before you send one
            preceding_event: BinaryRelation::IsFirst(Box::new(And(Box::new([
                BinaryRelation::Or(Box::new([
                    Unary(Box::new(EventIs(SendStopGrn))),
                    Unary(Box::new(EventIs(EventKind::RecvLtc))),
                    BinaryRelation::And(Box::new([
                        Unary(Box::new(Or(Box::new([
                            EventIs(RecvNet),
                            EventIs(SendTaggedMsg),
                        ])))),
                        // FederateEquals,
                    ])),
                ])),
                TagEquals,
                Unary(Box::new(Predicate::And(Box::new([
                    TagFinite,
                    TagNonzero, // instead of tag nonzero it should be "tag greater than min input delay to federate"
                ])))),
            ])))),
            event: Predicate::And(Box::new([
                Or(Box::new([EventIs(SendTag), EventIs(SendPtag)])),
                // TagNonzero, // ???
            ])),
        },
        Rule {
            // Once you receive an LTC for a tag, you will never receive a PortAbsent nor TaggedMessage for any earlier or equal tag
            preceding_event: And(Box::new([
                Unary(Box::new(Predicate::Or(Box::new([
                    EventIs(RecvPortAbs),
                    EventIs(RecvTaggedMsg),
                ])))),
                FederateEquals,
                TagLessThanOrEqual,
            ])),
            event: Predicate::And(Box::new([EventIs(RecvLtc)])),
        },
        Rule {
            // Once you receive an LTC for a tag, you will never receive a NET for any earlier or equal tag
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(RecvNet))),
                FederateEquals,
                TagLessThanOrEqual,
            ])),
            event: Predicate::And(Box::new([EventIs(RecvLtc), TagNonzero])),
        },
        // The following are for lower-bounding receive times or portabs and tagged messages.
        Rule {
            // Once you receive port absent or tagged message for a tag, you cannot receive an LTC for a tag that is so early that it plus the max delay of all outgoing connections is less than the tag of the port absent or tagged message
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(RecvLtc))),
                FederateEquals,
                TagPlusLargestDelayLessThan,
            ])),
            event: Predicate::Or(Box::new([EventIs(RecvPortAbs), EventIs(RecvTaggedMsg)])),
        },
        Rule {
            // Once you receive port absent or tagged message for a tag, you cannot send the very first tag/ptag for a tag that is so early that it plus the max delay of all outgoing connections is less than or equal to the tag of the port absent or tagged message
            preceding_event: BinaryRelation::IsFirst(Box::new(And(Box::new([
                Unary(Box::new(Or(Box::new([
                    EventIs(SendTag),
                    EventIs(SendPtag),
                ])))),
                FederateEquals,
                TagPlusLargestDelayGreaterThanOrEqual,
            ])))),
            event: Predicate::And(Box::new([
                Predicate::Or(Box::new([EventIs(RecvPortAbs), EventIs(RecvTaggedMsg)])),
                Not(Box::new(
                    FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag,
                )),
            ])),
        },
        // The following is for handling of PTAGs and TAGs.
        Rule {
            // tags and ptags to the same federate are monotonic
            preceding_event: And(Box::new([
                Unary(Box::new(Or(Box::new([
                    EventIs(SendPtag),
                    EventIs(SendTag),
                ])))),
                FederateEquals,
                TagLessThan,
            ])),
            event: Or(Box::new([EventIs(SendPtag), EventIs(SendTag)])),
        },
        Rule {
            // PTAGs before TAGs
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(SendPtag))),
                FederateEquals,
                TagLessThanOrEqual,
            ])),
            event: EventIs(SendTag),
        },
        Rule {
            // you can't send a TAG nor PTAG until you have received a high enough NET from any upstream federate
            preceding_event: BinaryRelation::IsFirstForFederate(Box::new(And(Box::new([
                Unary(Box::new(EventIs(RecvNet))),
                TagPlusDelay2FedGreaterThanOrEquals,
            ])))),
            event: Predicate::And(Box::new([
                Or(Box::new([EventIs(SendPtag), EventIs(SendTag)])),
                TagNonzero,
                Not(Box::new(
                    FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag,
                )), // should be tag greater than min input delay to federate
            ])),
        }, // this rule does not seem very helpful; it may be redundant with those that follow it
        Rule {
            // you can't grant a TAG until you have received a high enough LTC from any upstream federate or you have granted a strictly higher TAG to an upstream federate
            preceding_event: BinaryRelation::IsFirstForFederate(Box::new(BinaryRelation::Or(
                Box::new([
                    And(Box::new([
                        Unary(Box::new(EventIs(RecvLtc))),
                        FederateZeroDelayDirectlyUpstreamOf,
                        TagGreaterThanOrEqual,
                    ])),
                    And(Box::new([
                        Unary(Box::new(Or(Box::new([
                            EventIs(SendTag),
                            EventIs(RecvNet),
                            EventIs(SendStopGrn),
                        ])))),
                        FederateZeroDelayDirectlyUpstreamOf,
                        TagGreaterThanOrEqual,
                    ])),
                ]),
            ))),
            event: Predicate::And(Box::new([EventIs(SendTag), TagNonzero])),
        },
        Rule {
            // in particular, you can't send a PTAG until either you have sent an equal PTAG to an upstream federate that is upstream with only zero delay, or you have received an equal NET from the same federate
            preceding_event: BinaryRelation::IsFirst(Box::new(BinaryRelation::Or(Box::new([
                And(Box::new([
                    Unary(Box::new(EventIs(SendPtag))),
                    FederateZeroDelayDirectlyUpstreamOf,
                    TagEquals,
                ])),
                And(Box::new([
                    Unary(Box::new(Or(Box::new([
                        EventIs(RecvNet),
                        EventIs(SendStopGrn),
                    ])))),
                    BinaryRelation::Or(Box::new([FederateEquals, FederateDirectlyUpstreamOf])),
                    TagEquals,
                ])),
            ])))),
            event: Predicate::And(Box::new([EventIs(SendPtag), TagNonzero])),
        },
        // Rule {
        //     preceding_event: BinaryRelation::IsFirst(Box::new(And(Box::new([
        //         Unary(Box::new(Or(Box::new([
        //             EventIs(SendTag),
        //             EventIs(SendPtag),
        //         ])))),
        //         TagGreaterThanOrEqual,
        //         FederateEquals,
        //     ])))),
        //     event: Or(Box::new([EventIs(RecvPortAbs), EventIs(RecvTaggedMsg)])),
        // },
        // The following are for receive/forward dependencies.
        Rule {
            preceding_event: BinaryRelation::IsFirst(Box::new(And(Box::new([
                Unary(Box::new(EventIs(RecvPortAbs))),
                FederateZeroDelayDirectlyUpstreamOf,
                TagEquals,
            ])))),
            event: EventIs(SendPortAbs),
        },
        Rule {
            preceding_event: BinaryRelation::IsFirst(Box::new(And(Box::new([
                Unary(Box::new(EventIs(RecvTaggedMsg))),
                FederateDirectlyUpstreamOf,
                TagEquals,
            ])))),
            event: EventIs(SendTaggedMsg),
        },
        // The following pertain to when a federate can receive a message.
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(Or(Box::new([
                    // EventIs(RecvPortAbs),
                    // EventIs(RecvTaggedMsg),  // I do not think these are correct? But there is no counterexample.
                    EventIs(SendPortAbs),
                    EventIs(SendTaggedMsg),
                ])))),
                FederateEquals,
                TagLessThanOrEqual,
            ])),
            event: Or(Box::new([EventIs(RecvLtc)])),
        },
        // Rule {
        //     preceding_event: And(Box::new([
        //         Unary(Box::new(Or(Box::new([EventIs(RecvLtc)])))),
        //         FederateEquals,
        //         TagLessThan,
        //     ])),
        //     event: Or(Box::new([EventIs(SendPortAbs), EventIs(SendTaggedMsg)])),
        // },  // Not true, even though it seems like maybe it should be (we have no backpressure, unbounded buffers?)
        // The following encode the startup sequence in which a federate connects to RTI.
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(RecvFedId))),
                FederateEquals,
            ])),
            event: EventIs(SendAck),
        },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(SendAck))),
                FederateEquals,
            ])),
            event: EventIs(RecvTimestamp),
        },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(RecvTimestamp))),
                FederateEquals,
            ])),
            event: EventIs(SendTimestamp),
        },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(EventIs(SendTimestamp))),
                FederateEquals,
            ])),
            event: Predicate::And(Box::new([EventIs(RecvNet), Not(Box::new(TagNonzero))])),
        },
        Rule {
            preceding_event: And(Box::new([Unary(Box::new(EventIs(RecvTimestamp)))])),
            // preceding_event: And(Box::new([Unary(Box::new(Predicate::And(Box::new([
            //     EventIs(RecvNet),
            //     Not(Box::new(TagNonzero)),
            // ]))))])),
            event: Or(Box::new([
                EventIs(RecvLtc),
                EventIs(RecvPortAbs),
                EventIs(RecvTaggedMsg),
                EventIs(SendTag),
                EventIs(SendPtag),
                EventIs(SendPortAbs),
                EventIs(SendTaggedMsg),
                EventIs(SendStopGrn),
                EventIs(SendStopReq),
                EventIs(RecvStopReq),
                EventIs(RecvStopReqRep),
            ])),
        },
    ]
}
