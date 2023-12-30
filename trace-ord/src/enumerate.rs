use std::collections::{HashMap, HashSet};

use crate::{BinaryRelation, EventKind, Predicate, Rule};
#[derive(Debug)]
pub struct ByFuel<Ab: Abstraction>(pub Vec<Vec<(Ab::R, Ab)>>); // TODO: no pub

pub trait NaryRelation: Sized + Clone {
    fn atoms() -> Vec<Self>;
    fn kind(&self) -> NaryRelationKind;
    // fn and(terms: Box<[Self]>) -> Self;
    // fn or(terms: Box<[Self]>) -> Self;
    // fn not(&self) -> Self;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NaryRelationKind {
    And,
    Or,
    Not,
    Other,
}
#[allow(type_alias_bounds)] // it looks like a bug that this is necessary
pub type ConcAbst<Ab: Abstraction> = (Ab::R, Ab);
pub trait Abstraction: Sized + Clone {
    type R: NaryRelation;
    fn fact(fact: &Self::R) -> Self;
    fn and(
        concterms: impl Fn() -> Box<[Self::R]>,
        absterms: impl Iterator<Item = Self> + Clone,
    ) -> Option<ConcAbst<Self>>;
    fn or(
        concterms: impl Fn() -> Box<[Self::R]>,
        absterms: impl Iterator<Item = Self> + Clone,
    ) -> Option<ConcAbst<Self>>;
    fn not(&self, concterm: &Self::R) -> Option<ConcAbst<Self>>;
    fn uninhabitable(&self) -> bool;
}

#[derive(Debug, Clone, Copy)]
pub struct PowerBool {
    pub maybe_true: bool,
    pub maybe_false: bool,
}

impl<T: Abstraction> Default for ByFuel<T> {
    fn default() -> Self {
        Self(vec![])
    }
}

impl<Ab: crate::enumerate::Abstraction> ByFuel<Ab> {
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
    fn exact_fuel(&self, fuel: usize) -> Vec<ConcAbst<Ab>> {
        if fuel == 0 {
            Ab::R::atoms()
                .into_iter()
                .map(|it| {
                    let ab = Ab::fact(&it);
                    (it, ab)
                })
                .collect()
        } else {
            let mut ret = vec![];
            // add And, Or, and Not, but not IsFirst or BoundBinary
            for (predicate, abstraction) in self.0[fuel - 1].iter().filter(|&(predicate, _)| {
                // no double negation
                predicate.kind() != NaryRelationKind::Not
            }) {
                if let Some(concabst) = abstraction.not(predicate) {
                    ret.push(concabst);
                }
            }
            let inexact_combinations = crate::enumerate::inexact_combinations(&self.0, fuel);
            // println!("inexact_combinations: {:?}", inexact_combinations);
            for combination in inexact_combinations.into_iter() {
                let bslice = || {
                    combination
                        .iter()
                        .map(|it| it.0.clone())
                        .collect::<Vec<_>>()
                        .into_boxed_slice()
                };
                let conniter = || combination.iter().map(|it| it.1.clone());
                let concaband = Ab::and(bslice, conniter());
                let concabor = Ab::or(bslice, conniter());
                if let Some(concaband) = concaband {
                    if !concaband.1.uninhabitable()
                        && !combination
                            .iter()
                            .any(|it| it.0.kind() == NaryRelationKind::And)
                    {
                        ret.push(concaband);
                    }
                }
                if let Some(concabor) = concabor {
                    if !concabor.1.uninhabitable()
                        && !combination
                            .iter()
                            .any(|it| it.0.kind() == NaryRelationKind::Or)
                    {
                        ret.push(concabor);
                    }
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
