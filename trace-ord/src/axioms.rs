use crate::lflib::{EventKind, Rule};

use crate::lflib::BinaryRelationAtom::{
    Equal, FederateDirectlyUpstreamOf, FederateEquals, FederateZeroDelayDirectlyUpstreamOf,
    GreaterThanOrEqual, LessThan, LessThanOrEqual,
};
use crate::lflib::DelayTerm::*;
use crate::lflib::EventKind::*;
use crate::lflib::PredicateAtom::*;
use crate::lflib::Term::*;
use crate::BinaryRelation;
use crate::BinaryRelation::{And, Unary};
use crate::Predicate;
use crate::Predicate::*;
pub fn axioms() -> Vec<Rule> {
    vec![
        // The following are for LTCs.
        Rule {
            // LTCs to the same federate are monotonic
            preceding_event: And(Box::new([
                Unary(Box::new(Atom(EventIs(RecvLtc)))),
                BinaryRelation::Atom(FederateEquals),
                BinaryRelation::Atom(LessThan(Tag, Tag)),
            ])),
            event: Atom(EventIs(RecvLtc)),
        },
        // Rule {
        //     preceding_event: And(Box::new([
        //         Unary(Box::new(EventIs(RecvLtc))),
        //         // Unary(Box::new(Atom(TagNonzero))),
        //         BinaryRelation::Atom(FederateEquals),
        //         TagStrictPlusDelayFromAllImmUpstreamFedsLessThan,
        //     ])),
        //     event: EventIs(RecvPortAbs),
        // }, // ditto to below
        // Rule {
        //     preceding_event: And(Box::new([
        //         Unary(Box::new(EventIs(RecvLtc))),
        //         BinaryRelation::Atom(FederateEquals),
        //         TagPlusDelayFromAllImmUpstreamFedsLessThan,
        //     ])),
        //     event: EventIs(RecvTaggedMsg),
        // }, // disabled because it isn't true: the conninfo records the shortest delay, but there could be longer delays that put messages arbitrarily far into the future -- even though the full structure of the program, which is not yet encoded in the conninfo, would not allow that.
        // The following are for handling of NETs.
        Rule {
            // you should have received a net or a ltc or something that lets you know that a TAG or PTAG is needed before you send one
            preceding_event: BinaryRelation::IsFirst(Box::new(And(Box::new([
                BinaryRelation::Or(Box::new([
                    Unary(Box::new(Atom(EventIs(SendStopGrn)))),
                    Unary(Box::new(Atom(EventIs(EventKind::RecvLtc)))),
                    BinaryRelation::And(Box::new([
                        Unary(Box::new(Or(Box::new([
                            Atom(EventIs(RecvNet)),
                            Atom(EventIs(SendTaggedMsg)),
                        ])))),
                        // BinaryRelation::Atom(FederateEquals),
                    ])),
                ])),
                BinaryRelation::Atom(Equal(Tag, Tag)),
                Unary(Box::new(Predicate::And(Box::new([
                    Atom(TagFinite),
                    Atom(TagNonzero), // instead of tag nonzero it should be "tag greater than min input delay to federate"
                ])))),
            ])))),
            event: Predicate::And(Box::new([
                Or(Box::new([Atom(EventIs(SendTag)), Atom(EventIs(SendPtag))])),
                // Atom(TagNonzero), // ???
            ])),
        },
        Rule {
            // Once you receive an LTC for a tag, you will never receive a PortAbsent nor TaggedMessage for any earlier or equal tag
            preceding_event: And(Box::new([
                Unary(Box::new(Predicate::Or(Box::new([
                    Atom(EventIs(RecvPortAbs)),
                    Atom(EventIs(RecvTaggedMsg)),
                ])))),
                BinaryRelation::Atom(FederateEquals),
                BinaryRelation::Atom(LessThanOrEqual(Tag, Tag)),
            ])),
            event: Predicate::And(Box::new([Atom(EventIs(RecvLtc))])),
        },
        Rule {
            // Once you receive an LTC for a tag, you will never receive a NET for any earlier or equal tag
            preceding_event: And(Box::new([
                Unary(Box::new(Atom(EventIs(RecvNet)))),
                BinaryRelation::Atom(FederateEquals),
                BinaryRelation::Atom(LessThanOrEqual(Tag, Tag)),
            ])),
            event: Predicate::And(Box::new([Atom(EventIs(RecvLtc)), Atom(TagNonzero)])),
        },
        // The following are for lower-bounding receive times or portabs and tagged messages.
        Rule {
            // Once you receive port absent or tagged message for a tag, you cannot receive an LTC for a tag that is so early that it plus the max delay of all outgoing connections is less than the tag of the port absent or tagged message
            preceding_event: And(Box::new([
                Unary(Box::new(Atom(EventIs(RecvLtc)))),
                BinaryRelation::Atom(FederateEquals),
                BinaryRelation::Atom(LessThan(TagPlusDelay(LargestDelayFrom), Tag)),
            ])),
            event: Predicate::Or(Box::new([
                Atom(EventIs(RecvPortAbs)),
                Atom(EventIs(RecvTaggedMsg)),
            ])),
        },
        Rule {
            // Once you receive port absent or tagged message for a tag, you cannot send the very first tag/ptag for a tag that is so early that it plus the max delay of all outgoing connections is less than or equal to the tag of the port absent or tagged message
            preceding_event: BinaryRelation::IsFirst(Box::new(And(Box::new([
                Unary(Box::new(Or(Box::new([
                    Atom(EventIs(SendTag)),
                    Atom(EventIs(SendPtag)),
                ])))),
                BinaryRelation::Atom(FederateEquals),
                BinaryRelation::Atom(GreaterThanOrEqual(TagPlusDelay(LargestDelayFrom), Tag)),
            ])))),
            event: Predicate::And(Box::new([
                Predicate::Or(Box::new([
                    Atom(EventIs(RecvPortAbs)),
                    Atom(EventIs(RecvTaggedMsg)),
                ])),
                Not(Box::new(Atom(
                    FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag,
                ))),
            ])),
        },
        // The following is for handling of PTAGs and TAGs.
        Rule {
            // tags and ptags to the same federate are monotonic
            preceding_event: And(Box::new([
                Unary(Box::new(Or(Box::new([
                    Atom(EventIs(SendPtag)),
                    Atom(EventIs(SendTag)),
                ])))),
                BinaryRelation::Atom(FederateEquals),
                BinaryRelation::Atom(LessThan(Tag, Tag)),
            ])),
            event: Or(Box::new([Atom(EventIs(SendPtag)), Atom(EventIs(SendTag))])),
        },
        Rule {
            // PTAGs before TAGs
            preceding_event: And(Box::new([
                Unary(Box::new(Atom(EventIs(SendPtag)))),
                BinaryRelation::Atom(FederateEquals),
                BinaryRelation::Atom(LessThanOrEqual(Tag, Tag)),
            ])),
            event: Atom(EventIs(SendTag)),
        },
        Rule {
            // you can't grant a TAG until you have received a high enough LTC from any upstream federate or you have granted a strictly higher TAG to an upstream federate
            preceding_event: BinaryRelation::IsFirstForFederate(Box::new(BinaryRelation::Or(
                Box::new([
                    And(Box::new([
                        Unary(Box::new(Atom(EventIs(RecvLtc)))),
                        BinaryRelation::Atom(FederateZeroDelayDirectlyUpstreamOf),
                        BinaryRelation::Atom(GreaterThanOrEqual(Tag, Tag)),
                    ])),
                    And(Box::new([
                        Unary(Box::new(Or(Box::new([
                            Atom(EventIs(SendTag)),
                            Atom(EventIs(RecvNet)),
                            Atom(EventIs(SendStopGrn)),
                        ])))),
                        BinaryRelation::Atom(FederateZeroDelayDirectlyUpstreamOf),
                        BinaryRelation::Atom(GreaterThanOrEqual(Tag, Tag)),
                    ])),
                ]),
            ))),
            event: Predicate::And(Box::new([Atom(EventIs(SendTag)), Atom(TagNonzero)])),
        },
        Rule {
            // in particular, you can't send a PTAG until either you have sent an equal PTAG to an upstream federate that is upstream with only zero delay, or you have received an equal NET from the same federate
            preceding_event: BinaryRelation::IsFirst(Box::new(BinaryRelation::Or(Box::new([
                And(Box::new([
                    Unary(Box::new(Atom(EventIs(SendPtag)))),
                    BinaryRelation::Atom(FederateZeroDelayDirectlyUpstreamOf),
                    BinaryRelation::Atom(Equal(Tag, Tag)),
                ])),
                And(Box::new([
                    Unary(Box::new(Or(Box::new([
                        Atom(EventIs(RecvNet)),
                        Atom(EventIs(SendStopGrn)),
                    ])))),
                    BinaryRelation::Or(Box::new([
                        BinaryRelation::Atom(FederateEquals),
                        BinaryRelation::Atom(FederateDirectlyUpstreamOf),
                    ])),
                    BinaryRelation::Atom(Equal(Tag, Tag)),
                ])),
            ])))),
            event: Predicate::And(Box::new([Atom(EventIs(SendPtag)), Atom(TagNonzero)])),
        },
        // Rule {
        //     preceding_event: BinaryRelation::IsFirst(Box::new(And(Box::new([
        //         Unary(Box::new(Or(Box::new([
        //             Atom(EventIs(SendTag)),
        //             Atom(EventIs(SendPtag)),
        //         ])))),
        //         GreaterThanOrBinaryRelation::Atom(Equal(Tag, Tag)),
        //         BinaryRelation::Atom(FederateEquals),
        //     ])))),
        //     event: Or(Box::new([Atom(EventIs(RecvPortAbs)), Atom(EventIs(RecvTaggedMsg))])),
        // },
        // The following are for receive/forward dependencies.
        Rule {
            preceding_event: BinaryRelation::IsFirst(Box::new(And(Box::new([
                Unary(Box::new(Atom(EventIs(RecvPortAbs)))),
                BinaryRelation::Atom(FederateZeroDelayDirectlyUpstreamOf),
                BinaryRelation::Atom(Equal(Tag, Tag)),
            ])))),
            event: Atom(EventIs(SendPortAbs)),
        },
        Rule {
            preceding_event: BinaryRelation::IsFirst(Box::new(And(Box::new([
                Unary(Box::new(Atom(EventIs(RecvTaggedMsg)))),
                BinaryRelation::Atom(FederateDirectlyUpstreamOf),
                BinaryRelation::Atom(Equal(Tag, Tag)),
            ])))),
            event: Atom(EventIs(SendTaggedMsg)),
        },
        // The following pertain to when a federate can receive a message.
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(Or(Box::new([
                    Atom(EventIs(RecvPortAbs)),
                    Atom(EventIs(RecvTaggedMsg)),
                ])))),
                BinaryRelation::Atom(FederateEquals),
                BinaryRelation::Atom(LessThanOrEqual(Tag, Tag)), // Want: tag minus smallest delay less than or equal
            ])),
            event: Or(Box::new([Atom(EventIs(RecvLtc))])),
        },
        // Rule {
        //     preceding_event: And(Box::new([
        //         Unary(Box::new(Or(Box::new([Atom(EventIs(RecvLtc))])))),
        //         BinaryRelation::Atom(FederateEquals),
        //         BinaryRelation::Atom(LessThan(Tag, Tag)),
        //     ])),
        //     event: Or(Box::new([Atom(EventIs(SendPortAbs)), Atom(EventIs(SendTaggedMsg))])),
        // },  // Not true, even though it seems like maybe it should be (we have no backpressure, unbounded buffers?)
        // The following encode the startup sequence in which a federate connects to RTI.
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(Atom(EventIs(RecvFedId)))),
                BinaryRelation::Atom(FederateEquals),
            ])),
            event: Atom(EventIs(SendAck)),
        },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(Atom(EventIs(SendAck)))),
                BinaryRelation::Atom(FederateEquals),
            ])),
            event: Atom(EventIs(RecvTimestamp)),
        },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(Atom(EventIs(RecvTimestamp)))),
                BinaryRelation::Atom(FederateEquals),
            ])),
            event: Atom(EventIs(SendTimestamp)),
        },
        Rule {
            preceding_event: And(Box::new([
                Unary(Box::new(Atom(EventIs(SendTimestamp)))),
                BinaryRelation::Atom(FederateEquals),
            ])),
            event: Predicate::And(Box::new([
                Atom(EventIs(RecvNet)),
                Not(Box::new(Atom(TagNonzero))),
            ])),
        },
        Rule {
            preceding_event: And(Box::new([Unary(Box::new(Atom(EventIs(RecvTimestamp))))])),
            // preceding_event: And(Box::new([Unary(Box::new(Predicate::And(Box::new([
            //     Atom(EventIs(RecvNet)),
            //     Not(Box::new(Atom(TagNonzero))),
            // ]))))])),
            event: Or(Box::new([
                Atom(EventIs(RecvLtc)),
                Atom(EventIs(RecvPortAbs)),
                Atom(EventIs(RecvTaggedMsg)),
                Atom(EventIs(SendTag)),
                Atom(EventIs(SendPtag)),
                Atom(EventIs(SendPortAbs)),
                Atom(EventIs(SendTaggedMsg)),
                Atom(EventIs(SendStopGrn)),
                Atom(EventIs(SendStopReq)),
                Atom(EventIs(RecvStopReq)),
                Atom(EventIs(RecvStopReqRep)),
            ])),
        },
    ]
}
