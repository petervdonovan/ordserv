use std::collections::HashMap;

use crate::{event::ProjectToTrait, AtomTrait, EventTrait, Nary};
#[derive(Debug)]
pub struct ByFuel<Ab: Abstraction>(pub Vec<Vec<(Conc<Ab>, Ab)>>)
where
    [(); Ab::N - 1]:,
    [(); Ab::M - 1]:; // TODO: no pub

#[allow(type_alias_bounds)] // it looks like a bug that this is necessary
pub type ConcAbst<Ab: Abstraction> = (Conc<Ab>, Ab);
#[allow(type_alias_bounds)] // it looks like a bug that this is necessary
pub type Conc<Ab: Abstraction>
where
    [(); Ab::N - 1]:,
    [(); Ab::M - 1]:,
= Nary<
    Ab::AtomN,
    Ab::AtomM,
    Ab::ConcEvent,
    { Ab::N },
    { Ab::M },
    Ab::Ctx,
    Ab::ProjectTo,
    Ab::Atom1,
    Ab::Atom2,
>;
pub trait Abstraction: Sized + Clone {
    type ConcEvent: EventTrait;
    type Ctx: std::fmt::Debug + Clone;
    type ProjectTo: ProjectToTrait;
    type Atom1: AtomTrait<1, Self::ConcEvent, Self::Ctx, Self::ProjectTo, Self::Atom1, Self::Atom2>;
    type Atom2: AtomTrait<2, Self::ConcEvent, Self::Ctx, Self::ProjectTo, Self::Atom1, Self::Atom2>;
    type AtomN: AtomTrait<
        { Self::N },
        Self::ConcEvent,
        Self::Ctx,
        Self::ProjectTo,
        Self::Atom1,
        Self::Atom2,
    >
    where
        [(); Self::N]:;
    type AtomM: AtomTrait<
        { Self::M },
        Self::ConcEvent,
        Self::Ctx,
        Self::ProjectTo,
        Self::Atom1,
        Self::Atom2,
    >
    where
        [(); Self::M]:;
    const N: usize;
    const M: usize;
    fn fact(fact: &Conc<Self>) -> Self
    where
        [(); Self::N - 1]:,
        [(); Self::M - 1]:;
    fn and(
        concterms: impl Iterator<Item = Conc<Self>> + Clone,
        absterms: impl Iterator<Item = Self> + Clone,
    ) -> Option<ConcAbst<Self>>
    where
        [(); Self::N - 1]:,
        [(); Self::M - 1]:;
    fn or(
        concterms: impl Iterator<Item = Conc<Self>> + Clone,
        absterms: impl Iterator<Item = Self> + Clone,
    ) -> Option<ConcAbst<Self>>
    where
        [(); Self::N - 1]:,
        [(); Self::M - 1]:;
    fn not(&self, concterm: &Conc<Self>) -> Option<ConcAbst<Self>>
    where
        [(); Self::N - 1]:,
        [(); Self::M - 1]:;
}
#[derive(Debug, Clone)]
pub struct SimpleAbstraction<Ab: Abstraction>
where
    [(); Ab::N - 1]:,
    [(); Ab::M - 1]:,
{
    pub predicate2powerbool: HashMap<Conc<Ab>, PowerBool>,
}

#[derive(Debug, Clone, Copy)]
pub struct PowerBool {
    pub maybe_true: bool,
    pub maybe_false: bool,
}

impl<T: Abstraction> Default for ByFuel<T>
where
    [(); T::N - 1]:,
    [(); T::M - 1]:,
    [(); T::N]:,
    [(); T::M]:,
{
    fn default() -> Self {
        Self(vec![])
    }
}

impl<Ab: Abstraction> Default for SimpleAbstraction<Ab>
where
    [(); Ab::N - 1]:,
    [(); Ab::M - 1]:,
{
    fn default() -> Self {
        Self {
            predicate2powerbool: HashMap::default(),
        }
    }
}

impl<Ab: Abstraction> SimpleAbstraction<Ab>
where
    [(); Ab::N - 1]:,
    [(); Ab::M - 1]:,
{
    // TODO: move the filtering into here. This includes uninhabitable filtering perhaps (not sure), and definitely and/or/not filtering. Move it here from the advance fn
    pub fn fact(fact: &Conc<Ab>) -> Self {
        Self {
            predicate2powerbool: vec![(fact.clone(), PowerBool::new_true())]
                .into_iter()
                .collect(),
        }
    }

    pub fn and(
        concterms: impl Iterator<Item = Conc<Ab>> + Clone,
        absterms: impl Iterator<Item = Self> + Clone,
    ) -> Option<Self> {
        let predicate2powerbool = absterms.zip(concterms).fold(
            Some(HashMap::<Conc<Ab>, PowerBool>::default()),
            |mut acc, (abst, conct)| {
                // if conct.kind() == NaryRelationKind::And {
                //     return None;
                // }
                if let Nary::And(_) = conct {
                    return None;
                }
                if let Some(ref mut acc) = acc {
                    for (predicate, powerbool) in abst.predicate2powerbool.iter() {
                        let entry = acc.entry(predicate.clone()).or_default();
                        entry.and(powerbool);
                        if entry.uninhabitable() {
                            return None;
                        }
                    }
                }
                acc
            },
        )?;
        Some(Self {
            predicate2powerbool,
        })
    }

    pub fn or(
        concterms: impl Iterator<Item = Conc<Ab>> + Clone,
        absterms: impl Iterator<Item = Self> + Clone,
    ) -> Option<Self> {
        let predicate2powerbool = absterms.zip(concterms).fold(
            Some(HashMap::<Conc<Ab>, PowerBool>::default()),
            |mut acc, (abst, conct)| {
                if let Some(ref mut acc) = acc {
                    if let Nary::Or(_) = conct {
                        return None;
                    }
                    for (predicate, powerbool) in abst.predicate2powerbool.iter() {
                        // do not keep entries that map to top after being or'ed
                        let entry = acc.entry(predicate.clone()).or_default();
                        entry.or(powerbool);
                        if entry.is_top() {
                            acc.remove(predicate);
                        }
                    }
                }
                acc
            },
        )?;
        Some(Self {
            predicate2powerbool,
        })
    }

    pub fn not(&self, concterm: &Conc<Ab>) -> Option<Self> {
        if let Nary::Not(_) = concterm {
            return None;
        }
        let predicate2powerbool = self
            .predicate2powerbool
            .iter()
            .map(|(predicate, powerbool)| (predicate.clone(), powerbool.not()))
            .filter(|(_, pb)| !pb.is_top())
            .collect();
        Some(Self {
            predicate2powerbool,
        })
    }

    pub fn uninhabitable(&self) -> bool {
        self.predicate2powerbool
            .iter()
            .any(|(_, pb)| pb.uninhabitable())
    }
}

impl<Ab: crate::enumerate::Abstraction> ByFuel<Ab>
where
    [(); Ab::N - 1]:,
    [(); Ab::M - 1]:,
{
    pub fn advance(&mut self, fuel: usize) -> impl Iterator<Item = &ConcAbst<Ab>> {
        let mut ret: Box<dyn Iterator<Item = &ConcAbst<Ab>>> = Box::new(std::iter::empty());
        let len = self.0.len();
        for fuel in len..=fuel {
            let exact = self.exact_fuel(fuel);
            self.0.push(exact);
        }
        for fuel in len..=fuel {
            ret = Box::new(ret.chain(self.0[fuel].iter()));
        }
        ret
    }
    fn exact_fuel(&self, fuel: usize) -> Vec<ConcAbst<Ab>>
    where
        [(); Ab::N - 1]:,
        [(); Ab::M - 1]:,
    {
        if fuel == 0 {
            enum_iterator::all::<Ab::AtomN>()
                .map(|it| {
                    let it = Nary::Atom(it);
                    let ab = Ab::fact(&it);
                    (it, ab)
                })
                .collect()
        } else {
            let mut ret = vec![];
            // add And, Or, and Not, but not IsFirst or BoundBinary
            for (predicate, abstraction) in self.0[fuel - 1].iter() {
                if let Some(concabst) = abstraction.not(predicate) {
                    ret.push(concabst);
                }
            }
            let inexact_combinations = crate::enumerate::inexact_combinations(&self.0, fuel);
            // println!("inexact_combinations: {:?}", inexact_combinations);
            for combination in inexact_combinations.into_iter() {
                let bslice = combination.iter().map(|it| it.0.clone());
                let conniter = || combination.iter().map(|it| it.1.clone());
                let concaband = Ab::and(bslice.clone(), conniter());
                let concabor = Ab::or(bslice, conniter());
                if let Some(concaband) = concaband {
                    ret.push(concaband);
                }
                if let Some(concabor) = concabor {
                    ret.push(concabor);
                }
            }
            ret
        }
    }
}

pub fn inexact_combinations<T>(lists_by_subfuel: &[Vec<T>], fuel: usize) -> Vec<Vec<T>>
where
    T: Clone,
{
    if fuel <= (1 << 1) + 1 {
        return vec![];
    }
    let max_subfuels = subfuels(fuel);
    let lesser_subfuels = subfuels(fuel - 1);
    let mut combinations: Vec<Vec<T>> = vec![];
    for last_envelope_break_location in 0..max_subfuels.len() {
        println!(
            "DEBUG: last_envelope_break_location: {}; fuel: {}; max_subfuels: {:?}",
            last_envelope_break_location, fuel, max_subfuels
        );
        let ranges: Vec<(usize, usize)> = max_subfuels
            .iter()
            .enumerate()
            .filter_map(|(idx, &subfuel)| {
                if idx <= last_envelope_break_location {
                    Some((
                        *lesser_subfuels
                            .get(last_envelope_break_location)
                            .unwrap_or(&0),
                        subfuel,
                    ))
                } else {
                    lesser_subfuels
                        .get(idx)
                        .map(|&lesser_subfuel| (0, lesser_subfuel))
                }
            })
            .collect::<Vec<_>>();
        println!("DEBUG: ranges: {:?}", ranges);
        let mut next_combinations = inexact_combinations_with_init(
            lists_by_subfuel,
            ranges.iter().map(|(_, b)| b).cloned().collect(),
            ranges.iter().map(|(a, _)| a).cloned().collect(),
        );
        combinations.append(&mut next_combinations);
    }
    return combinations;
    /// get the fuels of the constituent parts of an arbitrary-length item of the given fuel
    fn subfuels(fuel: usize) -> Vec<usize> {
        // return a geometrically decreasing sequence of subfuels where the maximum subfuel is no greater than the given fuel
        let mut subfuels = vec![];
        let mut subfuel = fuel;
        while subfuel > 0 {
            subfuels.push(subfuel);
            subfuel >>= 1;
        }
        subfuels
    }
    fn inexact_combinations_with_init<T>(
        lists_by_subfuel: &[Vec<T>],
        max_subfuels: Vec<usize>,
        init: Vec<usize>,
    ) -> Vec<Vec<T>>
    where
        T: Clone,
    {
        let mut exact_subfuels = init;
        let mut increment_idx = 0;
        let mut incrementables = max_subfuels
            .iter()
            .enumerate()
            .filter_map(|(idx, &max_subfuel)| {
                let diff = max_subfuel - exact_subfuels[idx];
                if diff > 1 {
                    Some((idx, diff))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let mut combinations: Vec<Vec<T>> = vec![];
        loop {
            println!(
                "DEBUG: exact_subfuels: {:?}; increment_idx: {}",
                exact_subfuels, increment_idx
            );
            let next_combinations = exact_combinations(lists_by_subfuel, &exact_subfuels);
            combinations.extend(next_combinations);
            if incrementables.is_empty() {
                break;
            }
            exact_subfuels[incrementables[increment_idx].0] += 1;
            incrementables[increment_idx].1 -= 1;
            if incrementables[increment_idx].1 == 1 {
                incrementables.remove(increment_idx);
                if incrementables.is_empty() {
                    break;
                }
            }
            increment_idx = (increment_idx + 1) % incrementables.len();
        }
        return combinations;
        /// get all combinations of predicates with fuel exactly matching the exact_subfuel
        fn exact_combinations<T>(
            lists_by_subfuel: &[Vec<T>],
            exact_subfuels: &[usize],
        ) -> Vec<Vec<T>>
        where
            T: Clone,
        {
            let mut combinations: Vec<Vec<T>> = vec![]; // invariant: each vector is strictly decreasing in idx
            let mut idxs: Vec<usize> = vec![];
            let mut last_subfuel = 0;
            for &subfuel in exact_subfuels.iter() {
                let mut next_combinations: Vec<Vec<T>> = vec![];
                let mut next_idxs: Vec<usize> = vec![];
                if combinations.is_empty() {
                    combinations = lists_by_subfuel[subfuel]
                        .iter()
                        .cloned()
                        .map(|it| vec![it])
                        .collect();
                    idxs = (0..combinations.len()).collect::<Vec<_>>();
                    last_subfuel = subfuel;
                    continue;
                }
                for (combination, strictmax_idx) in combinations.iter().zip(idxs.iter()) {
                    for (idx, item) in lists_by_subfuel[subfuel]
                        .iter()
                        .take(if last_subfuel == subfuel {
                            *strictmax_idx
                        } else {
                            usize::MAX
                        })
                        .enumerate()
                    {
                        let mut next_combination = combination.clone();
                        next_combination.push(item.clone());
                        next_combinations.push(next_combination);
                        next_idxs.push(idx);
                    }
                }
                combinations = next_combinations;
                idxs = next_idxs;
            }
            combinations
        }
    }
}
impl Default for PowerBool {
    fn default() -> Self {
        Self {
            maybe_true: true,
            maybe_false: true,
        }
    }
}
impl PowerBool {
    pub fn and(&mut self, other: &Self) {
        self.maybe_true &= other.maybe_true;
        self.maybe_false &= other.maybe_false;
    }
    pub fn or(&mut self, other: &Self) {
        self.maybe_true |= other.maybe_true;
        self.maybe_false |= other.maybe_false;
    }
    pub fn not(&self) -> Self {
        if self.is_false() {
            Self::new_true()
        } else if self.is_true() {
            Self::new_false()
        } else {
            Self::default()
        }
    }
    pub fn is_top(&self) -> bool {
        self.maybe_true && self.maybe_false
    }
    pub fn is_true(&self) -> bool {
        self.maybe_true && !self.maybe_false
    }
    pub fn is_false(&self) -> bool {
        !self.maybe_true && self.maybe_false
    }
    pub fn new_true() -> Self {
        Self {
            maybe_true: true,
            maybe_false: false,
        }
    }
    pub fn new_false() -> Self {
        Self {
            maybe_true: false,
            maybe_false: true,
        }
    }
    pub fn uninhabitable(&self) -> bool {
        !self.maybe_true && !self.maybe_false
    }
}
