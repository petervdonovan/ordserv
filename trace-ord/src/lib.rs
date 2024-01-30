#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use std::fmt::{Display, Formatter};

use enum_iterator::Sequence;

pub mod axioms;
pub mod conninfo;
pub mod enumerate;
pub mod lfenumerate;
pub mod lflib;
pub mod serde;
#[derive(Debug, Clone)]
pub enum Nary<AtomN, AtomM, Event, const N: usize, const M: usize, Ctx>
where
    AtomN: AtomTrait<N, Event, Ctx>,
    AtomM: AtomTrait<M, Event, Ctx>,
    Event: EventTrait,
    [(); N - 1]:,
    [(); M - 1]:,
{
    Atom(AtomN),
    IsFirst(Box<Self>),
    IsFirstForFederate(Box<Self>),
    And(Box<[Self]>),
    Or(Box<[Self]>),
    Not(Box<Self>),
    BoundMary(Box<([Event; M - 1], Nary<AtomM, AtomN, Event, M, N, Ctx>)>),
}
impl<AtomN, AtomM, Event, const N: usize, const M: usize, Ctx> PartialEq
    for Nary<AtomN, AtomM, Event, N, M, Ctx>
where
    AtomN: AtomTrait<N, Event, Ctx>,
    AtomM: AtomTrait<M, Event, Ctx>,
    Event: EventTrait,
    [(); N - 1]:,
    [(); M - 1]:,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Atom(l0), Self::Atom(r0)) => l0 == r0,
            (Self::IsFirst(l0), Self::IsFirst(r0)) => l0 == r0,
            (Self::IsFirstForFederate(l0), Self::IsFirstForFederate(r0)) => l0 == r0,
            (Self::And(l0), Self::And(r0)) => l0 == r0,
            (Self::Or(l0), Self::Or(r0)) => l0 == r0,
            (Self::Not(l0), Self::Not(r0)) => l0 == r0,
            (Self::BoundMary(l0), Self::BoundMary(r0)) => l0 == r0,
            _ => false,
        }
    }
}
impl<AtomN, AtomM, Event, const N: usize, const M: usize, Ctx> Eq
    for Nary<AtomN, AtomM, Event, N, M, Ctx>
where
    AtomN: AtomTrait<N, Event, Ctx>,
    AtomM: AtomTrait<M, Event, Ctx>,
    Event: EventTrait,
    [(); N - 1]:,
    [(); M - 1]:,
{
}
impl<AtomN, AtomM, Event, const N: usize, const M: usize, Ctx> std::hash::Hash
    for Nary<AtomN, AtomM, Event, N, M, Ctx>
where
    AtomN: AtomTrait<N, Event, Ctx>,
    AtomM: AtomTrait<M, Event, Ctx>,
    Event: EventTrait,
    [(); N - 1]:,
    [(); M - 1]:,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Atom(atom) => {
                state.write_u8(0);
                atom.hash(state);
            }
            Self::IsFirst(relation) => {
                state.write_u8(1);
                relation.hash(state);
            }
            Self::IsFirstForFederate(relation) => {
                state.write_u8(2);
                relation.hash(state);
            }
            Self::And(relations) => {
                state.write_u8(3);
                relations.hash(state);
            }
            Self::Or(relations) => {
                state.write_u8(4);
                relations.hash(state);
            }
            Self::Not(relation) => {
                state.write_u8(5);
                relation.hash(state);
            }
            Self::BoundMary(bound) => {
                state.write_u8(6);
                bound.hash(state);
            }
        }
    }
}
impl<AtomN, AtomM, Event, const N: usize, const M: usize, Ctx> Nary<AtomN, AtomM, Event, N, M, Ctx>
where
    AtomN: AtomTrait<N, Event, Ctx>,
    AtomM: AtomTrait<M, Event, Ctx>,
    Event: EventTrait,
    [(); N - 1]:,
    [(); M - 1]:,
{
    fn arbitrary_numeric_kind(&self) -> u8 {
        match self {
            Self::Atom(_) => 0,
            Self::IsFirst(_) => 1,
            Self::IsFirstForFederate(_) => 2,
            Self::And(_) => 3,
            Self::Or(_) => 4,
            Self::Not(_) => 5,
            Self::BoundMary(_) => 6,
        }
    }
}
impl<AtomN, AtomM, Event, const N: usize, const M: usize, Ctx> PartialOrd
    for Nary<AtomN, AtomM, Event, N, M, Ctx>
where
    AtomN: AtomTrait<N, Event, Ctx>,
    AtomM: AtomTrait<M, Event, Ctx>,
    Event: EventTrait,
    [(); N - 1]:,
    [(); M - 1]:,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<AtomN, AtomM, Event, const N: usize, const M: usize, Ctx> Ord
    for Nary<AtomN, AtomM, Event, N, M, Ctx>
where
    AtomN: AtomTrait<N, Event, Ctx>,
    AtomM: AtomTrait<M, Event, Ctx>,
    Event: EventTrait,
    [(); N - 1]:,
    [(); M - 1]:,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Atom(l0), Self::Atom(r0)) => l0.cmp(r0),
            (Self::IsFirst(l0), Self::IsFirst(r0)) => l0.cmp(r0),
            (Self::IsFirstForFederate(l0), Self::IsFirstForFederate(r0)) => l0.cmp(r0),
            (Self::And(l0), Self::And(r0)) => l0.cmp(r0),
            (Self::Or(l0), Self::Or(r0)) => l0.cmp(r0),
            (Self::Not(l0), Self::Not(r0)) => l0.cmp(r0),
            (Self::BoundMary(l0), Self::BoundMary(r0)) => l0.cmp(r0),
            _ => self
                .arbitrary_numeric_kind()
                .cmp(&other.arbitrary_numeric_kind()),
        }
    }
}
pub type UnaryRelation<Atom1, Atom2, Event, Ctx> = Nary<Atom1, Atom2, Event, 1, 2, Ctx>;
pub type BinaryRelation<Atom1, Atom2, Event, Ctx> = Nary<Atom2, Atom1, Event, 2, 1, Ctx>;
pub trait EventTrait: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord + Display {}
impl<T> EventTrait for T where T: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord + Display {}
pub trait AtomTrait<const N: usize, Event: EventTrait, Ctx>:
    std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord + Display + Sequence
{
    fn holds(&self, e: &[Event; N], ctx: &Ctx) -> bool;
}
// impl<T> AtomTrait for T where
//     T: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord + Display + Sequence
// {
// }

impl<AtomN, AtomM, Event, const N: usize, const M: usize, Ctx> Display
    for Nary<AtomN, AtomM, Event, N, M, Ctx>
where
    AtomN: AtomTrait<N, Event, Ctx>,
    AtomM: AtomTrait<M, Event, Ctx>,
    Event: EventTrait,
    [(); N - 1]:,
    [(); M - 1]:,
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
            Nary::BoundMary(bound) if M == 2 => {
                write!(f, "(for e = {}, {})", bound.0[0], bound.1)
            }
            Nary::BoundMary(bound) if M == 1 => write!(f, "{}", bound.1),
            Nary::BoundMary(_bound) => panic!("not (yet) implemented"),
        }
    }
}
impl<AtomN, AtomM, Event, const N: usize, const M: usize, Ctx> Nary<AtomN, AtomM, Event, N, M, Ctx>
where
    AtomN: AtomTrait<N, Event, Ctx>,
    AtomM: AtomTrait<M, Event, Ctx>,
    Event: EventTrait,
    [(); N - 1]:,
    [(); M - 1]:,
{
    pub fn holds(&self, chronological_events: &[Event; N], ctx: &Ctx) -> bool {
        match self {
            Nary::Atom(atom) => atom.holds(chronological_events, ctx),
            Nary::IsFirst(r) => {
                if true
                //let Event::First(other) = &chronological_events[0]
                {
                    // other == &Nary::BoundMary(Box::new(([chronological_events[1..]], *r.clone())))
                    // maybe not the most efficient
                    // TODO: consider logical equivalence?
                } else {
                    false
                }
            }
            Nary::IsFirstForFederate(r) => {
                if true
                //let Event::FirstForFederate(_, other_r) = &chronological_events[0]
                {
                    true
                    // other_r
                    //     == &UnaryRelation::BoundMary(Box::new((
                    //         [chronological_events[1..]],
                    //         *r.clone(),
                    //     )))
                } else {
                    false
                }
            }
            Nary::And(relations) => relations
                .iter()
                .all(|rel| rel.holds(chronological_events, ctx)),
            Nary::Or(relations) => relations
                .iter()
                .any(|rel| rel.holds(chronological_events, ctx)),
            Nary::Not(_relation) => panic!("this might never be necessary to implement"),
            Nary::BoundMary(p) => {
                let mut all = chronological_events.to_vec();
                all.extend_from_slice(&p.0);
                p.1.holds(all[..M].try_into().unwrap(), ctx)
            }
        }
    }
}
