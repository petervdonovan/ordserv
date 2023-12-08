use crate::{Event, Relation, Rule};

pub fn axioms() -> Vec<Rule> {
    vec![
        // The following is boilerplate for "first"s that are elaborated in. TODO: automate this.
        Rule {
            preceding_event: Event::FirstRecvNetOrSendStopGrnOrRecvLtc,
            event: Event::RecvNet,
            relations: vec![Relation::FederateEqual, Relation::TagPlusDelayEqual],
        },
        Rule {
            preceding_event: Event::FirstRecvNetOrSendStopGrnOrRecvLtc,
            event: Event::SendStopGrn,
            relations: vec![Relation::FederateEqual, Relation::TagPlusDelayEqual],
        },
        Rule {
            preceding_event: Event::RecvFirstPortAbsOrTaggedMsg,
            event: Event::RecvPortAbs,
            relations: vec![Relation::FederateEqual, Relation::TagPlusDelayEqual],
        },
        Rule {
            preceding_event: Event::RecvFirstPortAbsOrTaggedMsg,
            event: Event::RecvTaggedMsg,
            relations: vec![Relation::FederateEqual, Relation::TagPlusDelayEqual],
        },
        Rule {
            preceding_event: Event::SendFirstTagOrPtag,
            event: Event::SendTag,
            relations: vec![Relation::FederateEqual, Relation::TagPlusDelayEqual],
        },
        Rule {
            preceding_event: Event::SendFirstTagOrPtag,
            event: Event::SendPtag,
            relations: vec![Relation::FederateEqual, Relation::TagPlusDelayEqual],
        },
        // The following are for LTCs.
        Rule {
            preceding_event: Event::RecvTaggedMsg,
            event: Event::RecvLtc,
            relations: vec![
                Relation::TagPlusDelayLessThanOrEqual,
                Relation::FederateEqual,
            ],
        },
        Rule {
            preceding_event: Event::RecvPortAbs,
            event: Event::RecvLtc,
            relations: vec![
                Relation::TagPlusDelayLessThanOrEqual,
                Relation::FederateEqual,
            ],
        },
        Rule {
            preceding_event: Event::RecvLtc,
            event: Event::RecvFirstPortAbsOrTaggedMsg,
            relations: vec![Relation::TagPlusDelayLessThan, Relation::FederateEqual],
        },
        // The following are for handling of NETs.
        Rule {
            preceding_event: Event::FirstRecvNetOrSendStopGrnOrRecvLtc,
            event: Event::SendFirstTagOrPtag,
            relations: vec![
                Relation::TagPlusDelayEqual,
                Relation::FederateEqual,
                Relation::TagFinite,
                Relation::FirstTagNonzero,
            ],
        },
        Rule {
            preceding_event: Event::RecvLtc,
            event: Event::RecvNet,
            relations: vec![Relation::TagPlusDelayLessThan, Relation::FederateEqual],
        },
        Rule {
            preceding_event: Event::SendPtag,
            event: Event::RecvLtc,
            relations: vec![
                Relation::TagPlusDelayLessThanOrEqual,
                Relation::FederateEqual,
            ],
        },
        Rule {
            preceding_event: Event::SendTag,
            event: Event::RecvLtc,
            relations: vec![
                Relation::TagPlusDelayLessThanOrEqual,
                Relation::FederateEqual,
            ],
        },
        // The following is for handling of PTAGs and TAGs.
        Rule {
            preceding_event: Event::SendPtag,
            event: Event::SendTag,
            relations: vec![
                Relation::TagPlusDelayLessThanOrEqual,
                Relation::FederateEqual,
            ],
        },
        Rule {
            preceding_event: Event::SendFirstTagOrPtag,
            event: Event::RecvFirstPortAbsOrTaggedMsg,
            relations: vec![Relation::TagPlusDelayEqual, Relation::FederateEqual],
        },
        // The following encode the startup sequence in which a federate connects to RTI.
        Rule {
            preceding_event: Event::RecvFedId,
            event: Event::SendAck,
            relations: vec![Relation::FederateEqual],
        },
        Rule {
            preceding_event: Event::SendAck,
            event: Event::RecvTimestamp,
            relations: vec![Relation::FederateEqual],
        },
        Rule {
            preceding_event: Event::RecvTimestamp,
            event: Event::SendTimestamp,
            relations: vec![Relation::FederateEqual],
        },
        Rule {
            preceding_event: Event::SendTimestamp,
            event: Event::RecvNet,
            relations: vec![Relation::FederateEqual],
        },
    ]
}
