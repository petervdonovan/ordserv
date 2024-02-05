use crate::{AtomTrait, EventTrait, UnaryRelation};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Event<UnaryRelation, ConcEvent, ProjectTo>
where
    // Atom1: AtomTrait<1, ConcEvent, Ctx>,
    // Atom2: AtomTrait<2, ConcEvent, Ctx>,
    ConcEvent: EventTrait,
{
    Concrete(ConcEvent),
    First(UnaryRelation),
    FirstInEquivClass {
        proj: ProjectTo, // assume existence of a single canonical projection to ProjectTo; can be ensured using disjoint unions
        set: UnaryRelation,
    },
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
