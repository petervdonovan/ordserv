use crate::{BinaryRelation, Event, EventKind, Predicate, Rule};

#[derive(Debug, Default)]
pub struct PredicatesWithFuel {
    predicates_by_fuel: Vec<Vec<Predicate>>,
}
#[derive(Debug, Default)]
pub struct PredicatesWithBoundBinariesWithFuel(PredicatesWithFuel);
#[derive(Debug, Default)]
pub struct BinaryRelationsWithFuel {
    upper_bounded_binary_relations_by_fuel: Vec<Vec<BinaryRelation>>,
    lower_bounded_binary_relations_by_fuel: Vec<Vec<BinaryRelation>>,
    compact_binary_relations_by_fuel: Vec<Vec<BinaryRelation>>,
}
#[derive(Debug, Default)]
pub struct BinaryRelationsWithUnariesWithFuel(BinaryRelationsWithFuel);
#[derive(Debug, Default)]
pub struct RulesWithFuel {
    rules_by_fuel: Vec<Vec<Rule>>,
}

impl PredicatesWithFuel {
    pub fn advance(&mut self, fuel: usize) -> impl Iterator<Item = &Predicate> {
        /// get the fuels of the constituent parts of an arbitrary-length item of the given fuel
        fn subfuels(fuel: usize) -> Vec<usize> {
            (1..fuel - 1)
                .map(|i| fuel - 1 - i)
                .map(|i| 1 << i)
                .collect::<Vec<_>>()
        }
        /// get all combinations of predicates with fuel exactly matching the exact_subfuel
        fn exact_combinations<T>(
            lists_by_subfuel: &[Vec<T>],
            exact_subfuels: &[usize],
        ) -> Vec<Vec<T>>
        where
            T: Clone,
        {
            let mut combinations: Vec<Vec<T>> = vec![];
            for &subfuel in exact_subfuels.iter() {
                let mut next_combinations: Vec<Vec<T>> = vec![];
                for item in lists_by_subfuel[subfuel].iter() {
                    for combination in combinations.iter() {
                        let mut next_combination = combination.clone();
                        next_combination.push(item.clone());
                        next_combinations.push(next_combination);
                    }
                }
                combinations = next_combinations;
            }
            combinations
        }
        fn inexact_combinations<T>(lists_by_subfuel: &[Vec<T>], fuel: usize) -> Vec<Vec<T>>
        where
            T: Clone,
        {
            if fuel <= (1 << 1) + 1 {
                return vec![];
            }
            let subfuels = subfuels(fuel);
            let mut exact_subfuels = subfuels
                .iter()
                .map(|&fuel| fuel / 2 + 1)
                .collect::<Vec<_>>();
            let mut n_incrementables = subfuels.iter().filter(|it| **it > 1).count();
            let mut increment_idx = n_incrementables - 1;
            let mut combinations: Vec<Vec<T>> = vec![];
            while exact_subfuels != subfuels {
                let next_combinations = exact_combinations(lists_by_subfuel, &exact_subfuels);
                combinations.extend(next_combinations);
                // increment the increment_idx-th element of exact_subfuels
                exact_subfuels[increment_idx] -= 1;
                if exact_subfuels[increment_idx] == subfuels[increment_idx] {
                    n_incrementables -= 1;
                }
                increment_idx = (increment_idx - 1 + n_incrementables) % n_incrementables;
            }
            combinations
        }
        fn exact_fuel(predicates: &PredicatesWithFuel, fuel: usize) -> Vec<Predicate> {
            if fuel == 0 {
                let mut ret = vec![
                    Predicate::FedHasNoneUpstreamWithDelayLessThanOrEqualCurrentTag,
                    Predicate::TagNonzero,
                    Predicate::TagFinite,
                ];
                for kind in enum_iterator::all::<EventKind>() {
                    ret.push(Predicate::EventIs(kind));
                }
                ret
            } else {
                let mut ret = vec![];
                // add And, Or, and Not, but not IsFirst or BoundBinary
                for predicate in predicates.predicates_by_fuel[fuel - 1].iter() {
                    ret.push(Predicate::Not(Box::new(predicate.clone())));
                }
                let inexact_combinations =
                    inexact_combinations(&predicates.predicates_by_fuel, fuel);
                for combination in inexact_combinations.into_iter() {
                    ret.push(Predicate::And(combination.clone().into_boxed_slice()));
                    ret.push(Predicate::Or(combination.into_boxed_slice()));
                }
                ret
            }
        }
        let mut ret: Box<dyn Iterator<Item = &Predicate>> = Box::new(std::iter::empty());
        let len = self.predicates_by_fuel.len();
        for fuel in len..=fuel {
            let exact = exact_fuel(self, fuel);
            self.predicates_by_fuel.push(exact);
        }
        for fuel in len..=fuel {
            ret = Box::new(ret.chain(self.predicates_by_fuel[fuel].iter()));
        }
        ret
    }
}
