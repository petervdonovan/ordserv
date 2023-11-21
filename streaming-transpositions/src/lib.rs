use std::{collections::HashSet, fmt::Display};

use rand::Rng;
use rayon::iter::ParallelIterator;
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
    pub befores: &'a [HashSet<OgRank>],
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
        for (idx, (befores, before_and_afters)) in self
            .befores
            .iter()
            .zip(self.before_and_afters.iter())
            .enumerate()
        {
            write!(f, "befores[{}]: ", idx)?;
            fmt_ogrank_set(f, befores)?;
            write!(f, "\nbefore_and_afters[{}]: ", idx)?;
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
            ("Before", Box::new(|it: Orderings<'_>| it.befores)),
            ("Before and After", Box::new(|it| it.before_and_afters)),
        ]
    }
}

pub struct StreamingTranspositions {
    og_trace_length: usize,
    search_radius: i32,
    traces_recorded: NTraces,
    save_cumsum_when_cumsum_increases_by: f32,
    cumsum: CumSum,
    cumsums: Vec<(NTraces, CumSum)>,
    befores: Vec<HashSet<OgRank>>,
    before_and_afters: Vec<HashSet<OgRank>>,
}

impl StreamingTranspositions {
    pub fn new(
        og_trace_length: usize,
        search_radius: i32,
        save_cumsum_when_cumsum_increases_by: f32,
    ) -> Self {
        let mut befores = Vec::with_capacity(og_trace_length);
        let mut before_and_afters = Vec::with_capacity(og_trace_length);

        for _ in 0..og_trace_length {
            befores.push(HashSet::new());
            before_and_afters.push(HashSet::new());
        }

        Self {
            og_trace_length,
            search_radius,
            traces_recorded: NTraces(0),
            save_cumsum_when_cumsum_increases_by,
            cumsum: CumSum(0),
            cumsums: Vec::new(),
            befores,
            before_and_afters,
        }
    }
    fn grow_before_and_afters_set(&mut self, trace: &OgRank2CurRank) {
        for idx in 0..self.og_trace_length {
            let mut remove_set = Vec::new();
            for before_ogrank in self.befores[idx].iter() {
                if trace.0[before_ogrank.idx()] > trace.0[idx] {
                    self.before_and_afters[idx].insert(*before_ogrank);
                    self.before_and_afters[before_ogrank.idx()].insert(OgRank(idx as u32));
                    remove_set.push(*before_ogrank);
                    self.cumsum.0 += 1;
                }
            }
            for before_ogrank in remove_set {
                self.befores[idx].remove(&before_ogrank);
                self.befores[before_ogrank.idx()].remove(&OgRank(idx as u32));
            }
        }
    }
    pub fn record(&mut self, trace: OgRank2CurRank) {
        let mut ogrank_currank_pairs = trace.unpack();
        ogrank_currank_pairs.sort_by_key(|it| it.1);
        for idx in 0..self.og_trace_length {
            let ogrank = ogrank_currank_pairs[idx].0;
            let left_bound = (idx as i32 - self.search_radius).max(0) as usize;
            for (other_idx, _currank) in
                ogrank_currank_pairs[left_bound..idx]
                    .iter()
                    .filter(|(other_idx, _currank)| {
                        !self.before_and_afters[ogrank.idx()].contains(other_idx)
                    })
            {
                self.befores[ogrank.idx()].insert(*other_idx);
            }
        }
        self.grow_before_and_afters_set(&trace);
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
        &mut self,
        traces: impl ParallelIterator<Item = impl Iterator<Item = OgRank2CurRank>>,
    ) {
        let mapped: Vec<_> = traces
            .map(|trace| {
                let mut st = Self::new(
                    self.og_trace_length,
                    self.search_radius,
                    self.save_cumsum_when_cumsum_increases_by,
                );
                st.record_all(trace);
                st
            })
            .collect();
        for st in mapped {
            self.merge(st); // a parallel reduction that assumes associativity could give nondeterministic (but otherwise correct) results
        }
    }
    pub fn orderings(&self) -> Orderings {
        Orderings {
            befores: &self.befores,
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
        for (idx, (before_and_after, before)) in self
            .before_and_afters
            .iter()
            .zip(self.befores.iter())
            .enumerate()
        {
            for other in before_and_after.iter() {
                if before.contains(other) {
                    panic!(
                        "before_and_after[{}] contains {:?} but before[{}] also contains it",
                        idx, other, idx
                    );
                }
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
        for (idx, (other_before_and_after, other_before)) in other
            .before_and_afters
            .iter()
            .zip(other.befores.iter())
            .enumerate()
        {
            let idx_as_ogrank = OgRank(idx as u32);
            for other in other_before_and_after.iter() {
                if !self.before_and_afters[idx].contains(other) {
                    self.before_and_afters[idx].insert(*other);
                    self.befores[idx].remove(other);
                    self.before_and_afters[other.idx()].insert(idx_as_ogrank);
                    self.befores[other.idx()].remove(&idx_as_ogrank);
                    self.cumsum.0 += 1;
                }
            }
            for other in other_before.iter() {
                if !self.befores[idx].contains(other)
                    && !self.before_and_afters[idx].contains(other)
                {
                    if self.befores[other.idx()].contains(&idx_as_ogrank) {
                        self.before_and_afters[idx].insert(*other);
                        self.befores[idx].remove(other);
                        self.before_and_afters[other.idx()].insert(idx_as_ogrank);
                        self.befores[other.idx()].remove(&idx_as_ogrank);
                        self.cumsum.0 += 1;
                    } else {
                        self.befores[idx].insert(*other);
                    }
                }
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
            befores[0]: {}
            before_and_afters[0]: {}
            befores[1]: {OgRank(0)}
            before_and_afters[1]: {OgRank(2), OgRank(3)}
            befores[2]: {OgRank(0)}
            before_and_afters[2]: {OgRank(1), OgRank(3)}
            befores[3]: {OgRank(0)}
            before_and_afters[3]: {OgRank(1), OgRank(2)}
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
                        3,
                    ),
                ),
            ]
        "#]];
        let mut st = StreamingTranspositions::new(4, 1, 0.1);
        st.record_all(traces.into_iter().map(OgRank2CurRank));
        expected_before_and_after.assert_eq(&st.orderings().to_string());
        expected_cumsums.assert_debug_eq(&st.cumsums());
    }

    struct NaiveStreamingTranspositions(StreamingTranspositions);

    impl NaiveStreamingTranspositions {
        fn new(
            og_trace_length: usize,
            search_radius: i32,
            save_cumsum_when_cumsum_increases_by: f32,
        ) -> Self {
            Self(StreamingTranspositions::new(
                og_trace_length,
                search_radius,
                save_cumsum_when_cumsum_increases_by,
            ))
        }
        fn record(&mut self, trace: OgRank2CurRank) {
            let mut ogrank_currank_pairs = trace.unpack();
            ogrank_currank_pairs.sort_by_key(|it| it.1);
            for idx in 0..self.0.og_trace_length {
                let ogrank = ogrank_currank_pairs[idx].0;
                for (other_idx, _currank) in ogrank_currank_pairs[0..idx].iter() {
                    self.0.befores[ogrank.idx()].insert(*other_idx);
                    if self.0.befores[other_idx.idx()].contains(&ogrank) {
                        self.0.before_and_afters[ogrank.idx()].insert(*other_idx);
                        self.0.before_and_afters[other_idx.idx()].insert(ogrank);
                    }
                }
            }
        }
        fn record_all(&mut self, traces: impl Iterator<Item = OgRank2CurRank>) {
            for trace in traces {
                self.record(trace);
            }
        }
        fn orderings(&self) -> Orderings {
            Orderings {
                befores: &self.0.befores,
                before_and_afters: &self.0.before_and_afters,
            }
        }
    }

    #[test]
    pub fn randomized_test() {
        let traces = random_traces(100, 100, 30, 10);
        let mut st = StreamingTranspositions::new(100, 1, 0.1);
        let mut st_parallel = StreamingTranspositions::new(100, 1, 0.1);
        let mut st_naive = NaiveStreamingTranspositions::new(100, 1, 0.1);
        st.record_all(traces.iter().map(|it| OgRank2CurRank(it.clone())));
        st.check_invariants_expensive();
        st_parallel.par_record_all((0..10).into_par_iter().map(|start| {
            traces[start..(start + 10)]
                .iter()
                .map(|it| OgRank2CurRank(it.clone()))
        }));
        st_parallel.check_invariants_expensive();
        st_naive.record_all(traces.into_iter().map(OgRank2CurRank));
        for ((before_and_after, par_before_and_after), naive_before_and_after) in st
            .orderings()
            .before_and_afters
            .iter()
            .zip(st_parallel.orderings().before_and_afters.iter())
            .zip(st_naive.orderings().before_and_afters.iter())
        {
            for other in before_and_after.iter() {
                assert!(naive_before_and_after.contains(other));
            }
            for other in par_before_and_after.iter() {
                assert!(naive_before_and_after.contains(other));
            }
        }
    }
}
