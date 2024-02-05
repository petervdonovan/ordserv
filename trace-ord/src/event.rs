use crate::{AtomTrait, EventTrait, UnaryRelation};
use std::{fmt::Display, hash::Hash};

pub trait ProjectToTrait:
    PartialEq + Eq + Hash + PartialOrd + Ord + Clone + Display + std::fmt::Debug
{
}
impl<T> ProjectToTrait for T where
    T: PartialEq + Eq + Hash + PartialOrd + Ord + Clone + Display + std::fmt::Debug
{
}

pub trait CtxTrait: Clone {}
impl<T> CtxTrait for T where T: Clone {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Event<UnaryRelation, ConcEvent, ProjectTo>
where
    ConcEvent: EventTrait,
    ProjectTo: ProjectToTrait,
    UnaryRelation: Display + Clone,
{
    Concrete(ConcEvent),
    First(UnaryRelation),
    FirstInEquivClass {
        proj: ProjectTo, // assume existence of a single canonical projection to ProjectTo; can be ensured using disjoint unions
        set: UnaryRelation,
    },
}

impl<UnaryRelation, ConcEvent, ProjectTo> Display for Event<UnaryRelation, ConcEvent, ProjectTo>
where
    ConcEvent: EventTrait,
    ProjectTo: ProjectToTrait,
    UnaryRelation: Display + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Concrete(ce) => write!(f, "{}", ce),
            Self::First(ur) => write!(f, "First({})", ur),
            Self::FirstInEquivClass { proj, set } => {
                write!(f, "FirstInEquivClass {{ proj: {}, set: {} }}", proj, set)
            }
        }
    }
}

// impl<Atom1, Atom2, ConcEvent, Ctx, ProjectTo> Ord for Event<Atom1, Atom2, ConcEvent, Ctx, ProjectTo>
// where
//     Atom1: AtomTrait<1, ConcEvent, Ctx>,
//     Atom2: AtomTrait<2, ConcEvent, Ctx>,
//     ConcEvent: EventTrait,
// {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//         match (self, other) {
//             (Self::Concrete(l0), Self::Concrete(r0)) => l0.cmp(r0),
//             (Self::First(l0), Self::First(r0)) => l0.cmp(r0),
//             (
//                 Self::FirstInEquivClass { proj: _, set: l0 },
//                 Self::FirstInEquivClass { proj: _, set: r0 },
//             ) => l0.cmp(r0),
//             _ => self
//                 .arbitrary_numeric_kind()
//                 .cmp(&other.arbitrary_numeric_kind()),
//         }
//     }
// }
