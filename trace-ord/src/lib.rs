#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(const_trait_impl)]

use std::fmt::{Display, Formatter};

use enum_iterator::Sequence;
use event::{CtxTrait, ProjectToTrait};

pub mod axioms;
pub mod conninfo;
pub mod enumerate;
pub mod event;
pub mod lfenumerate;
pub mod lflib;
pub mod serde;
#[derive(Debug, Clone)]
pub enum Nary<AtomN, AtomM, ConcEvent, const N: usize, const M: usize, Ctx, ProjectTo, Atom1, Atom2>
where
    AtomN: AtomTrait<N, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    AtomM: AtomTrait<M, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom1: AtomTrait<1, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom2: AtomTrait<2, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Ctx: CtxTrait,
    ConcEvent: EventTrait,
    ProjectTo: ProjectToTrait,
    [(); N - 1]:,
    [(); M - 1]:,
{
    Atom(AtomN),
    IsFirst(Box<Nary<Atom2, Atom1, ConcEvent, 2, 1, Ctx, ProjectTo, Atom1, Atom2>>),
    IsFirstForFederate(Box<Nary<Atom2, Atom1, ConcEvent, 2, 1, Ctx, ProjectTo, Atom1, Atom2>>),
    And(Box<[Self]>),
    Or(Box<[Self]>),
    Not(Box<Self>),
    BoundMary(
        Box<(
            [crate::event::Event<
                UnaryRelation<Atom1, Atom2, ConcEvent, Ctx, ProjectTo>,
                ConcEvent,
                ProjectTo,
            >; M - 1],
            Nary<AtomM, AtomN, ConcEvent, M, N, Ctx, ProjectTo, Atom1, Atom2>,
        )>,
    ),
}
impl<AtomN, AtomM, ConcEvent, const N: usize, const M: usize, Ctx, ProjectTo, Atom1, Atom2>
    PartialEq for Nary<AtomN, AtomM, ConcEvent, N, M, Ctx, ProjectTo, Atom1, Atom2>
where
    AtomN: AtomTrait<N, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    AtomM: AtomTrait<M, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom1: AtomTrait<1, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom2: AtomTrait<2, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Ctx: CtxTrait,
    ConcEvent: EventTrait,
    ProjectTo: ProjectToTrait,
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
impl<AtomN, AtomM, ConcEvent, const N: usize, const M: usize, Ctx, ProjectTo, Atom1, Atom2> Eq
    for Nary<AtomN, AtomM, ConcEvent, N, M, Ctx, ProjectTo, Atom1, Atom2>
where
    AtomN: AtomTrait<N, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    AtomM: AtomTrait<M, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom1: AtomTrait<1, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom2: AtomTrait<2, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Ctx: CtxTrait,
    ConcEvent: EventTrait,
    ProjectTo: ProjectToTrait,
    [(); N - 1]:,
    [(); M - 1]:,
{
}
impl<AtomN, AtomM, ConcEvent, const N: usize, const M: usize, Ctx, ProjectTo, Atom1, Atom2>
    std::hash::Hash for Nary<AtomN, AtomM, ConcEvent, N, M, Ctx, ProjectTo, Atom1, Atom2>
where
    AtomN: AtomTrait<N, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    AtomM: AtomTrait<M, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom1: AtomTrait<1, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom2: AtomTrait<2, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Ctx: CtxTrait,
    ConcEvent: EventTrait,
    ProjectTo: ProjectToTrait,
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
impl<AtomN, AtomM, ConcEvent, const N: usize, const M: usize, Ctx, ProjectTo, Atom1, Atom2>
    Nary<AtomN, AtomM, ConcEvent, N, M, Ctx, ProjectTo, Atom1, Atom2>
where
    AtomN: AtomTrait<N, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    AtomM: AtomTrait<M, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom1: AtomTrait<1, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom2: AtomTrait<2, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Ctx: CtxTrait,
    ConcEvent: EventTrait,
    ProjectTo: ProjectToTrait,
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
impl<AtomN, AtomM, ConcEvent, const N: usize, const M: usize, Ctx, ProjectTo, Atom1, Atom2>
    PartialOrd for Nary<AtomN, AtomM, ConcEvent, N, M, Ctx, ProjectTo, Atom1, Atom2>
where
    AtomN: AtomTrait<N, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    AtomM: AtomTrait<M, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom1: AtomTrait<1, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom2: AtomTrait<2, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Ctx: CtxTrait,
    ConcEvent: EventTrait,
    ProjectTo: ProjectToTrait,
    [(); N - 1]:,
    [(); M - 1]:,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<AtomN, AtomM, ConcEvent, const N: usize, const M: usize, Ctx, ProjectTo, Atom1, Atom2> Ord
    for Nary<AtomN, AtomM, ConcEvent, N, M, Ctx, ProjectTo, Atom1, Atom2>
where
    AtomN: AtomTrait<N, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    AtomM: AtomTrait<M, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom1: AtomTrait<1, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom2: AtomTrait<2, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Ctx: CtxTrait,
    ConcEvent: EventTrait,
    ProjectTo: ProjectToTrait,
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
pub type UnaryRelation<Atom1, Atom2, ConcEvent, Ctx, ProjectTo> =
    Nary<Atom1, Atom2, ConcEvent, 1, 2, Ctx, ProjectTo, Atom1, Atom2>;
pub type BinaryRelation<Atom1, Atom2, ConcEvent, Ctx, ProjectTo> =
    Nary<Atom2, Atom1, ConcEvent, 2, 1, Ctx, ProjectTo, Atom1, Atom2>;
pub trait EventTrait: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord + Display {}
impl<T> EventTrait for T where T: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord + Display {}
pub trait AtomTrait<const N: usize, ConcEvent: EventTrait, Ctx, ProjectTo, Atom1, Atom2>:
    std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord + Display + Sequence
where
    Atom1: AtomTrait<1, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom2: AtomTrait<2, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    ProjectTo: ProjectToTrait,
    Ctx: CtxTrait,
{
    fn holds(
        &self,
        e: &[crate::event::Event<
            UnaryRelation<Atom1, Atom2, ConcEvent, Ctx, ProjectTo>,
            ConcEvent,
            ProjectTo,
        >; N],
        ctx: &Ctx,
    ) -> bool;
}
// impl<T> AtomTrait for T where
//     T: std::fmt::Debug + Clone + Eq + std::hash::Hash + Ord + Display + Sequence
// {
// }

impl<AtomN, AtomM, ConcEvent, const N: usize, const M: usize, Ctx, ProjectTo, Atom1, Atom2> Display
    for Nary<AtomN, AtomM, ConcEvent, N, M, Ctx, ProjectTo, Atom1, Atom2>
where
    AtomN: AtomTrait<N, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    AtomM: AtomTrait<M, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom1: AtomTrait<1, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom2: AtomTrait<2, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Ctx: CtxTrait,
    ConcEvent: EventTrait,
    ProjectTo: ProjectToTrait,
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
impl<AtomN, AtomM, ConcEvent, const N: usize, const M: usize, Ctx, ProjectTo, Atom1, Atom2>
    Nary<AtomN, AtomM, ConcEvent, N, M, Ctx, ProjectTo, Atom1, Atom2>
where
    AtomN: AtomTrait<N, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    AtomM: AtomTrait<M, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom1: AtomTrait<1, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Atom2: AtomTrait<2, ConcEvent, Ctx, ProjectTo, Atom1, Atom2>,
    Ctx: CtxTrait,
    ConcEvent: EventTrait,
    ProjectTo: ProjectToTrait,
    [(); N - 1]:,
    [(); M - 1]:,
{
    pub fn holds(
        &self,
        chronological_events: &[crate::event::Event<
            UnaryRelation<Atom1, Atom2, ConcEvent, Ctx, ProjectTo>,
            ConcEvent,
            ProjectTo,
        >; N],
        ctx: &Ctx,
    ) -> bool {
        match self {
            Nary::Atom(atom) => atom.holds(chronological_events, ctx),
            Nary::IsFirst(r) => {
                if let crate::event::Event::First(other) = &chronological_events[0] {
                    // let rest = chronological_events[1..M];
                    if N != 2 {
                        panic!("IsFirst only makes sense when N = 2, but N = {}", N);
                    }
                    let rest: &[crate::event::Event<
                        UnaryRelation<Atom1, Atom2, ConcEvent, Ctx, ProjectTo>,
                        ConcEvent,
                        ProjectTo,
                    >; 1] = (chronological_events[1..N]).try_into().unwrap();
                    let rest = rest.clone();
                    other == &Nary::BoundMary(Box::new((rest, (**r).clone())))
                    // maybe not the most efficient
                    // TODO: consider logical equivalence?
                } else {
                    false
                }
            }
            Nary::IsFirstForFederate(r) => {
                if let crate::event::Event::FirstInEquivClass {
                    proj: _,
                    set: other_r,
                } = &chronological_events[0]
                {
                    if N != 2 {
                        panic!(
                            "IsFirstForFederate only makes sense when N = 2, but N = {}",
                            N
                        );
                    }
                    let rest: &[crate::event::Event<
                        UnaryRelation<Atom1, Atom2, ConcEvent, Ctx, ProjectTo>,
                        ConcEvent,
                        ProjectTo,
                    >; 1] = chronological_events[1..N].try_into().unwrap();
                    other_r == &UnaryRelation::BoundMary(Box::new((rest.clone(), *r.clone())))
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
            Nary::Not(relation) => !relation.holds(chronological_events, ctx),
            Nary::BoundMary(p) => {
                let mut all = chronological_events.to_vec();
                all.extend_from_slice(&p.0);
                p.1.holds(all[..M].try_into().unwrap(), ctx)
            }
        }
    }
}
