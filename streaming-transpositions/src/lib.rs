use std::{collections::HashSet, fmt::Display};

use rand::Rng;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OgRank(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CurRank(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NTraces(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CumSum(pub u32);
pub struct OgRank2CurRank(pub Vec<CurRank>);
impl OgRank2CurRank {
    fn unpack(&self) -> Vec<(OgRank, CurRank)> {
        self.0
            .iter()
            .enumerate()
            .map(|(idx, currank)| (OgRank(idx as u32), *currank))
            .collect::<Vec<_>>()
    }
}
impl OgRank {
    pub fn idx(&self) -> usize {
        self.0 as usize
    }
}

pub struct Orderings<'a> {
    pub before_and_afters: &'a [HashSet<OgRank>],
}
fn fmt_ogrank_set(f: &mut std::fmt::Formatter<'_>, set: &HashSet<OgRank>) -> std::fmt::Result {
    write!(f, "{{")?;
    let mut sorted = set.iter().collect::<Vec<_>>();
    sorted.sort();
    for (idx, ogrank) in sorted.iter().enumerate() {
        if idx != 0 {
            write!(f, ", ")?;
        }
        write!(f, "{:?}", ogrank)?;
    }
    write!(f, "}}")
}
impl<'a> Display for Orderings<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, before_and_afters) in self.before_and_afters.iter().enumerate() {
            write!(f, "before_and_afters[{}]: ", idx)?;
            fmt_ogrank_set(f, before_and_afters)?;
            writeln!(f)?;
        }
        Ok(())
    }
}
type RelatedOgranksGiver<'a> = dyn Fn(Orderings<'a>) -> &'a [HashSet<OgRank>];
impl<'a> Orderings<'a> {
    pub fn projections<'b>() -> Vec<(&'static str, Box<RelatedOgranksGiver<'b>>)> {
        vec![
            // ("Before", Box::new(|it: Orderings<'_>| it.befores)),
            ("Before and After", Box::new(|it| it.before_and_afters)),
        ]
    }
}
#[derive(Debug, Clone)]
pub struct StreamingTranspositions {
    og_trace_length: usize,
    search_radius: i32,
    traces_recorded: NTraces,
    save_cumsum_when_cumsum_increases_by: f32,
    cumsum: CumSum,
    cumsums: Vec<(NTraces, CumSum)>,
    before_and_afters: Vec<HashSet<OgRank>>,
}

impl StreamingTranspositions {
    pub fn new(
        og_trace_length: usize,
        search_radius: i32,
        save_cumsum_when_cumsum_increases_by: f32,
    ) -> Self {
        let mut before_and_afters = Vec::with_capacity(og_trace_length);

        for _ in 0..og_trace_length {
            before_and_afters.push(HashSet::new());
        }

        Self {
            og_trace_length,
            search_radius,
            traces_recorded: NTraces(0),
            save_cumsum_when_cumsum_increases_by,
            cumsum: CumSum(0),
            cumsums: Vec::new(),
            before_and_afters,
        }
    }
    pub fn record(&mut self, trace: OgRank2CurRank) {
        let mut ogrank_currank_pairs = trace.unpack();
        ogrank_currank_pairs.sort_by_key(|it| it.1);
        for idx in 0..self.og_trace_length {
            let ogrank = ogrank_currank_pairs[idx].0;
            let left_bound = (idx as i32 - self.search_radius).max(0) as usize;
            for (other_idx, _currank) in ogrank_currank_pairs[left_bound..idx]
                .iter()
                .filter(|(other_idx, _currank)| other_idx > &ogrank)
            {
                self.before_and_afters[ogrank.idx()].insert(*other_idx);
                self.before_and_afters[other_idx.idx()].insert(ogrank);
                self.cumsum.0 += 1;
            }
        }
        self.traces_recorded.0 += 1;
        let last_n_cumsums = self.cumsums.last().map(|it| it.1 .0).unwrap_or(0);
        if self.cumsum.0 - last_n_cumsums
            >= (self.save_cumsum_when_cumsum_increases_by * (last_n_cumsums as f32)) as u32
        {
            self.cumsums.push((self.traces_recorded, self.cumsum));
        }
    }
    pub fn record_all(&mut self, traces: impl Iterator<Item = OgRank2CurRank>) {
        for trace in traces {
            self.record(trace);
        }
    }
    pub fn par_record_all(
        self,
        traces: impl ParallelIterator<Item = impl Iterator<Item = OgRank2CurRank>>,
    ) -> Self {
        let mut mapped: Vec<_> = traces
            .map(|trace| {
                let mut st = self.empty();
                st.record_all(trace);
                st
            })
            .collect();
        let empty = self.empty();
        mapped.push(self);
        mapped.into_par_iter().reduce(
            || empty.clone(),
            |mut a, mut b| {
                if a.before_and_afters.len() < b.before_and_afters.len() {
                    b.merge(a);
                    b
                } else {
                    a.merge(b);
                    a
                }
            },
        )
    }
    fn empty(&self) -> Self {
        Self::new(
            self.og_trace_length,
            self.search_radius,
            self.save_cumsum_when_cumsum_increases_by,
        )
    }
    pub fn orderings(&self) -> Orderings {
        Orderings {
            before_and_afters: &self.before_and_afters,
        }
    }
    pub fn cumsums(&self) -> &[(NTraces, CumSum)] {
        &self.cumsums
    }
    pub fn traces_recorded(&self) -> NTraces {
        self.traces_recorded
    }
    pub fn check_invariants_expensive(&self) {
        for (idx, before_and_after) in self.before_and_afters.iter().enumerate() {
            for other in before_and_after.iter() {
                if !self.before_and_afters[other.idx()].contains(&OgRank(idx as u32)) {
                    panic!(
                        "before_and_after[{}] contains {} but before_and_after[{}] does not contain {}",
                        idx, other.idx(), other.idx(), idx
                    );
                }
            }
        }
    }
    pub fn merge(&mut self, other: Self) {
        for (idx, other_before_and_after) in other.before_and_afters.iter().enumerate() {
            for other in other_before_and_after.iter() {
                self.before_and_afters[idx].insert(*other);
            }
        }
        self.traces_recorded.0 += other.traces_recorded.0;
        self.update_cumsums_if_needed();
    }
    fn update_cumsums_if_needed(&mut self) {
        let last_n_cumsums = self.cumsums.last().map(|it| it.1 .0).unwrap_or(0);
        if self.cumsum.0 - last_n_cumsums
            >= (self.save_cumsum_when_cumsum_increases_by * (last_n_cumsums as f32)) as u32
        {
            self.cumsums.push((self.traces_recorded, self.cumsum));
        }
    }
}

pub fn random_traces(
    trace_len: usize,
    n_traces: usize,
    reordering_times: usize,
    reordering_radius: usize,
) -> Vec<Vec<CurRank>> {
    let mut rng = rand::thread_rng();
    let mut traces = Vec::new();
    let og_trace = (0..trace_len as u32).map(CurRank).collect::<Vec<_>>();
    for _ in 0..n_traces {
        let mut trace = og_trace.clone();
        for _ in 0..reordering_times {
            let i = rng.gen_range(0..trace.len());
            let j = rng.gen_range(i..trace.len().min(i + reordering_radius));
            trace.swap(i, j);
        }
        traces.push(trace);
    }
    traces
}

#[cfg(test)]
pub mod tests {
    use expect_test::expect;
    use rayon::iter::IntoParallelIterator;

    use super::*;

    #[test]
    fn smoketest() {
        let traces = vec![
            vec![CurRank(0), CurRank(1), CurRank(2), CurRank(3)],
            vec![CurRank(0), CurRank(2), CurRank(1), CurRank(3)],
            vec![CurRank(0), CurRank(3), CurRank(2), CurRank(1)],
            vec![CurRank(0), CurRank(1), CurRank(3), CurRank(2)],
        ];
        let expected_before_and_after = expect![[r#"
            before_and_afters[0]: {}
            before_and_afters[1]: {OgRank(2)}
            before_and_afters[2]: {OgRank(1), OgRank(3)}
            before_and_afters[3]: {OgRank(2)}
        "#]];
        let expected_cumsums = expect![[r#"
            [
                (
                    NTraces(
                        1,
                    ),
                    CumSum(
                        0,
                    ),
                ),
                (
                    NTraces(
                        2,
                    ),
                    CumSum(
                        1,
                    ),
                ),
                (
                    NTraces(
                        3,
                    ),
                    CumSum(
                        3,
                    ),
                ),
                (
                    NTraces(
                        4,
                    ),
                    CumSum(
                        4,
                    ),
                ),
            ]
        "#]];
        let mut st = StreamingTranspositions::new(4, 1, 0.1);
        st.record_all(traces.into_iter().map(OgRank2CurRank));
        expected_before_and_after.assert_eq(&st.orderings().to_string());
        expected_cumsums.assert_debug_eq(&st.cumsums());
    }

    #[test]
    pub fn randomized_test() {
        let traces = random_traces(100, 100, 30, 10);
        let mut st = StreamingTranspositions::new(100, 1, 0.1);
        let st_parallel = StreamingTranspositions::new(100, 1, 0.1);
        st.record_all(traces.iter().map(|it| OgRank2CurRank(it.clone())));
        st.check_invariants_expensive();
        let st_parallel = st_parallel.par_record_all((0..10).into_par_iter().map(|start| {
            traces[start * 10..(start * 10 + 10)]
                .iter()
                .map(|it| OgRank2CurRank(it.clone()))
        }));
        st_parallel.check_invariants_expensive();
        for (before_and_after, par_before_and_after) in st
            .orderings()
            .before_and_afters
            .iter()
            .zip(st_parallel.orderings().before_and_afters.iter())
        {
            println!(
                "size difference = {} out of {}",
                before_and_after.len() - par_before_and_after.len(),
                before_and_after.len()
            );
            for other in before_and_after.iter() {
                assert!(par_before_and_after.contains(other));
            }
            for other in par_before_and_after.iter() {
                assert!(before_and_after.contains(other));
            }
        }
    }
}
