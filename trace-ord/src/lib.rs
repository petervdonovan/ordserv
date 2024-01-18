pub mod axioms;
pub mod conninfo;
pub mod enumerate;
pub mod lfenumerate;
pub mod lflib;
pub mod serde;
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Predicate<PAtom, BAtom, Event>
where
    PAtom: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord,
    BAtom: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord,
    Event: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord,
{
    Atom(PAtom),
    IsFirst(Box<Predicate<PAtom, BAtom, Event>>),
    And(Box<[Predicate<PAtom, BAtom, Event>]>),
    Or(Box<[Predicate<PAtom, BAtom, Event>]>),
    Not(Box<Predicate<PAtom, BAtom, Event>>),
    BoundBinary(Box<(Event, BinaryRelation<PAtom, BAtom, Event>)>),
}
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum BinaryRelation<PAtom, BAtom, Event>
where
    PAtom: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord,
    BAtom: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord,
    Event: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord,
{
    Atom(BAtom),
    IsFirst(Box<BinaryRelation<PAtom, BAtom, Event>>),
    IsFirstForFederate(Box<BinaryRelation<PAtom, BAtom, Event>>),
    And(Box<[BinaryRelation<PAtom, BAtom, Event>]>),
    Or(Box<[BinaryRelation<PAtom, BAtom, Event>]>),
    Unary(Box<Predicate<PAtom, BAtom, Event>>),
}
