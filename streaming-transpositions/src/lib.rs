use std::{collections::HashSet, fmt::Display, hash::Hash};

use rand::{distributions::Distribution, Rng};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OgRank(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CurRank(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NTraces(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CumSum(pub u32);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HookOgRank2CurRank(pub OgRank2CurRank);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutOgRank2CurRank(pub OgRank2CurRank);

pub struct Orderings<'a> {
    before_and_afters: &'a [HashSet<OgRank>],
    more_before_and_afters: Option<&'a [HashSet<OgRank>]>,
}
pub struct OrderingsIterator<'a, 'b>(&'b Orderings<'a>, usize);
impl<'a> Orderings<'a> {
    pub fn iter<'b>(&'b self) -> OrderingsIterator<'a, 'b> {
        OrderingsIterator(self, 0)
    }
}
impl<'a, 'b> Iterator for OrderingsIterator<'a, 'b> {
    type Item = HashSet<OgRank>;
    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.1;
        self.1 += 1;
        if idx == self.0.before_and_afters.len() {
            None
        } else {
            let mut ret = self.0.before_and_afters[idx].clone();
            if let Some(more_before_and_afters) = self.0.more_before_and_afters {
                for x in more_before_and_afters[idx].iter() {
                    ret.insert(*x);
                }
            }
            Some(ret)
        }
    }
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
// pub struct CumsumsIterator<'a>(&'a [(NTraces, CumSum)], &'a [()], usize);
#[derive(Debug, Clone)]
pub struct StreamingTranspositions {
    inner: StreamingTranspositionsDelta,
    all_ancestors: Option<Box<StreamingTranspositions>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingTranspositionsDelta {
    og_trace_length: usize,
    search_radius: i32,
    traces_recorded: NTraces,
    save_cumsum_when_cumsum_increases_by: f32,
    cumsum: CumSum,
    cumsums: Vec<(NTraces, CumSum)>,
    before_and_afters: Vec<HashSet<OgRank>>,
}
impl StreamingTranspositionsDelta {
    fn add_permutable(&mut self, ogrank: &OgRank, other_idx: &OgRank) {
        if !self.before_and_afters[ogrank.idx()].contains(other_idx) {
            self.before_and_afters[ogrank.idx()].insert(*other_idx);
            self.before_and_afters[other_idx.idx()].insert(*ogrank);
            self.cumsum.0 += 1;
        }
    }
    pub fn merge(&mut self, other: &Self) {
        for (idx, other_before_and_after) in other.before_and_afters.iter().enumerate() {
            for other in other_before_and_after.iter() {
                self.add_permutable(&OgRank(idx as u32), other);
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
impl From<StreamingTranspositionsDelta> for StreamingTranspositions {
    fn from(delta: StreamingTranspositionsDelta) -> Self {
        Self {
            inner: delta,
            all_ancestors: None,
        }
    }
}

impl StreamingTranspositions {
    pub fn new(
        og_trace_length: usize,
        search_radius: i32,
        save_cumsum_when_cumsum_increases_by: f32,
    ) -> Self {
        Self {
            inner: StreamingTranspositionsDelta {
                og_trace_length,
                search_radius,
                traces_recorded: NTraces(0),
                save_cumsum_when_cumsum_increases_by,
                cumsum: CumSum(0),
                cumsums: Vec::new(),
                before_and_afters: Self::empty_before_and_afters(og_trace_length),
            },
            all_ancestors: None,
        }
    }
    pub fn as_delta(&self) -> &'_ StreamingTranspositionsDelta {
        &self.inner
    }
    pub fn from_deltas<'a>(
        mut deltas: impl Iterator<Item = &'a StreamingTranspositionsDelta>,
    ) -> Self {
        let mut start: StreamingTranspositions = deltas.next().unwrap().clone().into();
        for delta in deltas {
            start.inner.merge(delta);
        }
        Self {
            inner: StreamingTranspositionsDelta {
                og_trace_length: start.inner.og_trace_length,
                search_radius: start.inner.search_radius,
                traces_recorded: start.inner.traces_recorded,
                save_cumsum_when_cumsum_increases_by: start
                    .inner
                    .save_cumsum_when_cumsum_increases_by,
                cumsum: start.inner.cumsum,
                cumsums: vec![],
                before_and_afters: Self::empty_before_and_afters(start.inner.og_trace_length),
            },
            all_ancestors: Some(Box::new(start)),
        }
    }
    pub fn update_ancestors(&mut self) {
        if self.all_ancestors.is_none() {
            self.all_ancestors = Some(Box::new(self.empty()));
        }
        self.all_ancestors
            .as_mut()
            .unwrap()
            .inner
            .merge(&self.inner);
        self.inner = StreamingTranspositionsDelta {
            og_trace_length: self.inner.og_trace_length,
            search_radius: self.inner.search_radius,
            traces_recorded: self.inner.traces_recorded,
            save_cumsum_when_cumsum_increases_by: self.inner.save_cumsum_when_cumsum_increases_by,
            cumsum: self.inner.cumsum,
            cumsums: vec![],
            before_and_afters: Self::empty_before_and_afters(self.inner.og_trace_length),
        };
    }
    fn empty_before_and_afters(size: usize) -> Vec<HashSet<OgRank>> {
        let mut before_and_afters = Vec::with_capacity(size);
        for _ in 0..size {
            before_and_afters.push(HashSet::new());
        }
        before_and_afters
    }
    pub fn record(&mut self, trace: OgRank2CurRank) {
        if trace.0.len() != self.inner.og_trace_length {
            panic!(
                "trace length {} does not match og_trace_length {}",
                trace.0.len(),
                self.inner.og_trace_length
            );
        }
        let mut ogrank_currank_pairs = trace.unpack();
        ogrank_currank_pairs.sort_by_key(|it| it.1);
        for idx in 0..self.inner.og_trace_length {
            let ogrank = ogrank_currank_pairs[idx].0;
            let left_bound = (idx as i32 - self.inner.search_radius).max(0) as usize;
            let og_trace_length = self.inner.og_trace_length as u32;
            let iterator = ogrank_currank_pairs[left_bound..idx]
                .iter()
                .filter(|(other_idx, _currank)| other_idx > &ogrank)
                .filter(|(_other_idx, currank)| currank.0 != og_trace_length);
            // the trace length is used as a placeholder when the hookinvocation of the ogrank is not observed
            if self.all_ancestors.is_some() {
                for (other_idx, _currank) in iterator {
                    self.add_permutable(&ogrank, other_idx);
                }
            } else {
                for (other_idx, _currank) in iterator {
                    self.inner.add_permutable(&ogrank, other_idx);
                }
            }
        }
        self.inner.traces_recorded.0 += 1;
        self.inner.update_cumsums_if_needed();
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
            |a, b| {
                let (mut target, source) = if a.inner.cumsum < b.inner.cumsum {
                    (b, a)
                } else {
                    (a, b)
                };
                if let Some(all_ancestors) = source.all_ancestors {
                    target.inner.merge(&all_ancestors.inner);
                }
                target.inner.merge(&source.inner);
                target
            },
        )
    }
    fn empty(&self) -> Self {
        Self::new(
            self.inner.og_trace_length,
            self.inner.search_radius,
            self.inner.save_cumsum_when_cumsum_increases_by,
        )
    }
    pub fn contains(&self, idx: OgRank, other_idx: OgRank) -> bool {
        self.inner.before_and_afters[idx.idx()].contains(&other_idx)
    }
    pub fn orderings(&self) -> Orderings {
        Orderings {
            before_and_afters: &self.inner.before_and_afters,
            more_before_and_afters: self
                .all_ancestors
                .as_ref()
                .map(|it| &it.inner.before_and_afters[..]),
        }
    }
    pub fn cumsums(&self) -> impl Iterator<Item = (NTraces, CumSum)> + '_ + Clone {
        self.all_ancestors
            .as_ref()
            .map(|it| it.inner.cumsums.iter())
            .unwrap_or_else(|| [].iter())
            .chain(self.inner.cumsums.iter())
            .map(|(n_traces, cumsum)| (*n_traces, *cumsum))
    }
    pub fn traces_recorded(&self) -> NTraces {
        self.inner.traces_recorded
    }
    /// Goes into an infinite loop if all pairs have been observed.
    pub fn random_unobserved_ordering(
        &self,
        r: f64,
        filter: impl Fn(OgRank, OgRank) -> bool,
    ) -> (OgRank, OgRank) {
        let mut rng = rand::thread_rng();
        let d = rand::distributions::Bernoulli::new(r).unwrap();
        loop {
            let i = rng.gen_range(0..self.inner.og_trace_length);
            for j in ((i + 1)..self.inner.og_trace_length)
                .filter(|&it| filter(OgRank(it as u32), OgRank(i as u32)))
            {
                if d.sample(&mut rng)
                    && !self.inner.before_and_afters[i].contains(&OgRank(j as u32))
                {
                    return (OgRank(j as u32), OgRank(i as u32));
                }
            }
        }
    }
    pub fn check_invariants_expensive(&self) {
        for (idx, before_and_after) in self.inner.before_and_afters.iter().enumerate() {
            for other in before_and_after.iter() {
                if !self.inner.before_and_afters[other.idx()].contains(&OgRank(idx as u32)) {
                    panic!(
                        "before_and_after[{}] contains {} but before_and_after[{}] does not contain {}",
                        idx, other.idx(), other.idx(), idx
                    );
                }
            }
        }
    }
    fn add_permutable(&mut self, idx: &OgRank, other: &OgRank) {
        if !self.inner.before_and_afters[idx.idx()].contains(other)
            && !self
                .all_ancestors
                .as_ref()
                .map(|it| it.inner.before_and_afters[idx.idx()].contains(other))
                .unwrap_or(false)
        {
            self.inner.cumsum.0 += 1;
            self.inner.before_and_afters[idx.idx()].insert(*other);
        }
    }
    pub fn cumsum(&self) -> CumSum {
        self.inner.cumsum
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BigSmallIterator {
    max_ogrank_strict: i32,
    power: u32, // loop variable
    diffmaxstrict: i32,
    diffmin: i32,
    start_minus_diff: i32, // loop variable
    diff: i32,             // loop variable
}

impl BigSmallIterator {
    pub fn from_strans(strans: &StreamingTranspositions) -> Self {
        Self::new(OgRank(strans.inner.og_trace_length as u32))
    }
    pub fn new(max_ogrank_strict: OgRank) -> Self {
        let diffmaxstrict = 2.min(max_ogrank_strict.0 as i32);
        Self {
            max_ogrank_strict: max_ogrank_strict.0 as i32,
            power: 0,
            diffmaxstrict,
            diffmin: 1,
            start_minus_diff: 1 - diffmaxstrict,
            diff: 1,
        }
    }
    pub fn power(&self) -> u32 {
        self.power
    }
    pub fn max_ogrank_strict(&self) -> OgRank {
        OgRank(self.max_ogrank_strict as u32)
    }
}

impl Iterator for BigSmallIterator {
    type Item = (OgRank, OgRank);
    /// Postcondition: The returned pair is in ascending order.
    fn next(&mut self) -> Option<(OgRank, OgRank)> {
        if self.diff < self.diffmaxstrict
            && self.diff <= (self.max_ogrank_strict - self.start_minus_diff) / 2
        {
            let start = self.start_minus_diff + self.diff;
            let other = start + self.diff;
            self.diff += 1;
            if other < self.max_ogrank_strict {
                Some((OgRank(start as u32), OgRank(other as u32)))
            } else {
                self.next()
            }
        } else if self.start_minus_diff < self.max_ogrank_strict - self.diffmin {
            self.start_minus_diff += 1;
            self.diff = self.diffmin.max(-self.start_minus_diff);
            self.diffmaxstrict = (1 << (self.power + 1)).min(self.max_ogrank_strict);
            self.next()
        } else if self.power <= self.max_ogrank_strict.ilog2() {
            self.power += 1;
            self.diffmin = 1 << self.power;
            self.diffmaxstrict = (1 << (self.power + 1)).min(self.max_ogrank_strict);
            self.start_minus_diff = 0 - self.diffmaxstrict + 1;
            self.diff = self.diffmin.max(-self.start_minus_diff);
            self.next()
        } else {
            None
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
                        2,
                    ),
                ),
                (
                    NTraces(
                        4,
                    ),
                    CumSum(
                        2,
                    ),
                ),
            ]
        "#]];
        let mut st = StreamingTranspositions::new(4, 1, 0.1);
        st.record_all(traces.into_iter().map(OgRank2CurRank));
        expected_before_and_after.assert_eq(&st.orderings().to_string());
        expected_cumsums.assert_debug_eq(&st.cumsums().collect::<Vec<_>>());
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
