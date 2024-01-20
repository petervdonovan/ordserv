use std::fmt::{Display, Formatter};

use enum_iterator::Sequence;

pub mod axioms;
pub mod conninfo;
pub mod enumerate;
pub mod lfenumerate;
pub mod lflib;
pub mod serde;
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Nary<
    AtomN,
    AtomM,
    Event,
    const N: usize,
    const M: usize,
    const NMinus1: usize,
    const MMinus1: usize,
> where
    AtomN: AtomTrait,
    AtomM: AtomTrait,
    Event: EventTrait,
{
    Atom(AtomN),
    IsFirst(Box<Nary<AtomN, AtomM, Event, N, M, NMinus1, MMinus1>>),
    IsFirstForFederate(Box<Nary<AtomN, AtomM, Event, N, M, NMinus1, MMinus1>>),
    And(Box<[Nary<AtomN, AtomM, Event, N, M, NMinus1, MMinus1>]>),
    Or(Box<[Nary<AtomN, AtomM, Event, N, M, NMinus1, MMinus1>]>),
    Not(Box<Nary<AtomN, AtomM, Event, N, M, NMinus1, MMinus1>>),
    BoundMary(
        Box<(
            [Event; MMinus1],
            Nary<AtomM, AtomN, Event, M, N, MMinus1, NMinus1>,
        )>,
    ),
}
pub type UnaryRelation<Atom1, Atom2, Event> = Nary<Atom1, Atom2, Event, 1, 2, 0, 1>;
pub type BinaryRelation<Atom1, Atom2, Event> = Nary<Atom2, Atom1, Event, 2, 1, 1, 0>;
trait EventTrait: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord + Display {}
impl<T> EventTrait for T where T: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord + Display {}
trait AtomTrait: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord + Sequence {}
impl<T> AtomTrait for T where T: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord + Sequence {}

impl<
        AtomN,
        AtomM,
        Event,
        const N: usize,
        const M: usize,
        const NMinus1: usize,
        const MMinus1: usize,
    > Display for Nary<AtomN, AtomM, Event, N, M, NMinus1, MMinus1>
where
    AtomN: AtomTrait,
    AtomM: AtomTrait,
    Event: EventTrait,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Nary::Atom(atom) => write!(f, "{}", atom),
            Nary::IsFirst(relation) => write!(f, "(FIRST {})", relation),
            Nary::IsFirstForFederate(relation) => write!(f, "(FedwiseFIRST {})", relation),
            Nary::And(relations) => {
                write!(f, "({})", relations[0])?;
                for relation in &relations[1..] {
                    write!(f, " ∧ {}", relation)?;
                }
                write!(f, ")")?;
                Ok(())
            }
            Nary::Or(relations) => {
                write!(f, "({}", relations[0])?;
                for relation in &relations[1..] {
                    write!(f, " ∨ {}", relation)?;
                }
                write!(f, ")")?;
                Ok(())
            }
            Nary::Not(relation) => write!(f, "¬{}", relation),
            Nary::BoundMary(bound) if MMinus1 == 1 => {
                write!(f, "(for e = {}, {})", bound.0[0], bound.1)
            }
            Nary::BoundMary(bound) if MMinus1 == 0 => write!(f, "{}", bound.1),
            Nary::BoundMary(bound) => panic!("not (yet) implemented"),
        }
    }
}
