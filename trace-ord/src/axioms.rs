use crate::{Event, EventType, Relation, Rule};

pub fn axioms() -> Vec<Rule> {
    vec![
        // The following is boilerplate for "first"s that are elaborated in.
        Rule {
            preceding_event: Event::Recv(EventType::FirstNet),
            event: Event::Recv(EventType::Net),
            relations: vec![Relation::FederateEqual, Relation::TagEqual],
        },
        Rule {
            preceding_event: Event::Recv(EventType::FirstPortAbsOrTaggedMsg),
            event: Event::Recv(EventType::PortAbs),
            relations: vec![Relation::FederateEqual, Relation::TagEqual],
        },
        Rule {
            preceding_event: Event::Recv(EventType::FirstPortAbsOrTaggedMsg),
            event: Event::Recv(EventType::TaggedMsg),
            relations: vec![Relation::FederateEqual, Relation::TagEqual],
        },
        Rule {
            preceding_event: Event::Send(EventType::FirstTagOrPtag),
            event: Event::Send(EventType::Tag),
            relations: vec![Relation::FederateEqual, Relation::TagEqual],
        },
        Rule {
            preceding_event: Event::Send(EventType::FirstTagOrPtag),
            event: Event::Send(EventType::Ptag),
            relations: vec![Relation::FederateEqual, Relation::TagEqual],
        },
        // The following are for LTCs.
        Rule {
            preceding_event: Event::Recv(EventType::TaggedMsg),
            event: Event::Recv(EventType::Ltc),
            relations: vec![Relation::TagLessThanOrEqual, Relation::FederateEqual],
        },
        Rule {
            preceding_event: Event::Recv(EventType::PortAbs),
            event: Event::Recv(EventType::Ltc),
            relations: vec![Relation::TagLessThanOrEqual, Relation::FederateEqual],
        },
        Rule {
            preceding_event: Event::Recv(EventType::Ltc),
            event: Event::Recv(EventType::FirstPortAbsOrTaggedMsg),
            relations: vec![Relation::TagLessThan, Relation::FederateEqual],
        },
        // The following are for handling of NETs.
        Rule {
            preceding_event: Event::Recv(EventType::FirstNet),
            event: Event::Send(EventType::FirstTagOrPtag),
            relations: vec![
                Relation::TagEqual,
                Relation::FederateEqual,
                Relation::TagFinite,
            ],
        },
        Rule {
            preceding_event: Event::Recv(EventType::Ltc),
            event: Event::Send(EventType::Net),
            relations: vec![Relation::TagLessThan, Relation::FederateEqual],
        },
        Rule {
            preceding_event: Event::Send(EventType::Ptag),
            event: Event::Recv(EventType::Ltc),
            relations: vec![Relation::TagLessThanOrEqual, Relation::FederateEqual],
        },
        Rule {
            preceding_event: Event::Send(EventType::Tag),
            event: Event::Recv(EventType::Ltc),
            relations: vec![Relation::TagLessThanOrEqual, Relation::FederateEqual],
        },
        // The following is for handling of PTAGs and TAGs.
        Rule {
            preceding_event: Event::Send(EventType::Ptag),
            event: Event::Send(EventType::Tag),
            relations: vec![Relation::TagLessThanOrEqual, Relation::FederateEqual],
        },
        Rule {
            preceding_event: Event::Send(EventType::FirstTagOrPtag),
            event: Event::Recv(EventType::FirstPortAbsOrTaggedMsg),
            relations: vec![Relation::TagEqual, Relation::FederateEqual],
        },
        // The following encode the startup sequence in which a federate connects to RTI.
        Rule {
            preceding_event: Event::Recv(EventType::FedId),
            event: Event::Send(EventType::Ack),
            relations: vec![Relation::FederateEqual],
        },
        Rule {
            preceding_event: Event::Send(EventType::Ack),
            event: Event::Recv(EventType::Timestamp),
            relations: vec![Relation::FederateEqual],
        },
        Rule {
            preceding_event: Event::Recv(EventType::Timestamp),
            event: Event::Send(EventType::Timestamp),
            relations: vec![Relation::FederateEqual],
        },
        Rule {
            preceding_event: Event::Send(EventType::Timestamp),
            event: Event::Recv(EventType::Net),
            relations: vec![Relation::FederateEqual],
        },
    ]
}
