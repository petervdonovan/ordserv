use std::{collections::HashSet, fmt::Display};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OgRank(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CurRank(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NTraces(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CumSum(pub u32);
pub struct OgRank2CurRank<'a>(pub &'a [CurRank]);
impl<'a> OgRank2CurRank<'a> {
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
    pub afters: &'a [HashSet<OgRank>],
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
        for (idx, ((befores, afters), before_and_afters)) in self
            .befores
            .iter()
            .zip(self.afters.iter())
            .zip(self.before_and_afters.iter())
            .enumerate()
        {
            write!(f, "befores[{}]: ", idx)?;
            fmt_ogrank_set(f, befores)?;
            write!(f, "\nafters[{}]: ", idx)?;
            fmt_ogrank_set(f, afters)?;
            write!(f, "\nbefore_and_afters[{}]: ", idx)?;
            fmt_ogrank_set(f, before_and_afters)?;
            writeln!(f)?;
        }
        Ok(())
    }
}
type RelatedOgranksGiver<'a> = dyn Fn(&'a Orderings) -> &'a [HashSet<OgRank>];
impl<'a> Orderings<'a> {
    pub fn projections<'b>() -> Vec<(&'static str, Box<RelatedOgranksGiver<'b>>)> {
        vec![
            ("Before", Box::new(|it: &'_ Orderings<'_>| it.befores)),
            ("After", Box::new(|it| it.afters)),
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
    afters: Vec<HashSet<OgRank>>,
    before_and_afters: Vec<HashSet<OgRank>>,
}

impl StreamingTranspositions {
    pub fn new(
        og_trace_length: usize,
        search_radius: i32,
        save_cumsum_when_cumsum_increases_by: f32,
    ) -> Self {
        let mut befores = Vec::with_capacity(og_trace_length);
        let mut afters = Vec::with_capacity(og_trace_length);
        let mut before_and_afters = Vec::with_capacity(og_trace_length);

        for _ in 0..og_trace_length {
            befores.push(HashSet::new());
            afters.push(HashSet::new());
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
            afters,
            before_and_afters,
        }
    }
    fn grow_before_and_afters_set(&mut self, trace: &OgRank2CurRank<'_>) {
        fn do_set(
            semiset: &mut [HashSet<OgRank>],
            semiset_condition: impl Fn(CurRank, CurRank) -> bool,
            transpose_semiset: &mut [HashSet<OgRank>],
            before_and_afters: &mut [HashSet<OgRank>],
            cumsum: &mut CumSum,
            cur_ogrank: usize,
            trace: &OgRank2CurRank<'_>,
        ) {
            let mut remove_set = Vec::new();
            let mut transpose_remove_set = Vec::new();
            for semi_ogrank in semiset[cur_ogrank].iter() {
                if semiset_condition(trace.0[cur_ogrank], trace.0[semi_ogrank.idx()]) {
                    before_and_afters[cur_ogrank].insert(*semi_ogrank);
                    before_and_afters[semi_ogrank.idx()].insert(OgRank(cur_ogrank as u32));
                    remove_set.push(*semi_ogrank);
                    transpose_remove_set.push(OgRank(cur_ogrank as u32));
                    cumsum.0 += 1;
                }
            }
            for semi_ogrank in remove_set {
                semiset[cur_ogrank].remove(&semi_ogrank);
                transpose_semiset[cur_ogrank].remove(&semi_ogrank);
            }
            for semi_ogrank in transpose_remove_set {
                semiset[semi_ogrank.idx()].remove(&OgRank(cur_ogrank as u32));
                transpose_semiset[semi_ogrank.idx()].remove(&OgRank(cur_ogrank as u32));
            }
        }
        for idx in 0..self.og_trace_length {
            do_set(
                &mut self.befores,
                |a, b| a < b,
                &mut self.afters,
                &mut self.before_and_afters,
                &mut self.cumsum,
                idx,
                &trace,
            );
            do_set(
                &mut self.afters,
                |a, b| a > b,
                &mut self.befores,
                &mut self.before_and_afters,
                &mut self.cumsum,
                idx,
                &trace,
            );
            // let mut befores_remove_set = Vec::new();
            // for before_ogrank in self.befores[idx].iter() {
            //     if trace.0[before_ogrank.idx()] > trace.0[idx] {
            //         self.before_and_afters[idx].insert(*before_ogrank);
            //         self.before_and_afters[before_ogrank.idx()].insert(OgRank(idx as u32));
            //         befores_remove_set.push(*before_ogrank);
            //         self.cumsum.0 += 1;
            //     }
            // }
            // for before_ogrank in befores_remove_set {
            //     self.befores[idx].remove(&before_ogrank);
            // }
            // let mut afters_remove_set = Vec::new();
            // for after_ogrank in self.afters[idx].iter() {
            //     if trace.0[after_ogrank.idx()] < trace.0[idx] {
            //         self.before_and_afters[idx].insert(*after_ogrank);
            //         self.before_and_afters[after_ogrank.idx()].insert(OgRank(idx as u32));
            //         afters_remove_set.push(*after_ogrank);
            //         self.cumsum.0 += 1;
            //     }
            // }
            // for after_ogrank in afters_remove_set {
            //     self.afters[idx].remove(&after_ogrank);
            // }
        }
    }
    pub fn record(&mut self, trace: OgRank2CurRank<'_>) {
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
                println!(
                    "inserting {:?} into befores[{:?}] (n traces seen = {}, before_and_afters[{:?}] = {:?})",
                    other_idx,
                    ogrank.idx(),
                    self.traces_recorded.0,
                    ogrank.idx(),
                    self.before_and_afters[ogrank.idx()]
                );
                self.befores[ogrank.idx()].insert(*other_idx);
            }
        }
        self.grow_before_and_afters_set(&trace);
        for idx in 0..(self.og_trace_length.max(1) - 1) {
            let ogrank = ogrank_currank_pairs[idx].0;
            let right_bound =
                (idx + 1 + (self.search_radius as usize)).min(self.og_trace_length - 1);
            for (other_idx, _currank) in
                ogrank_currank_pairs[idx + 1..right_bound]
                    .iter()
                    .filter(|(other_idx, _currank)| {
                        !self.before_and_afters[ogrank.idx()].contains(other_idx)
                    })
            {
                self.afters[ogrank.idx()].insert(*other_idx);
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
    pub fn record_all<'a>(&mut self, traces: impl Iterator<Item = OgRank2CurRank<'a>>) {
        for trace in traces {
            self.record(trace);
        }
    }
    pub fn orderings(&self) -> Orderings {
        Orderings {
            befores: &self.befores,
            afters: &self.afters,
            before_and_afters: &self.before_and_afters,
        }
    }
    pub fn cumsums(&self) -> &[(NTraces, CumSum)] {
        &self.cumsums
    }
}

#[cfg(test)]
pub mod tests {
    use expect_test::expect;

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
            afters[0]: {OgRank(1), OgRank(2), OgRank(3)}
            before_and_afters[0]: {}
            befores[1]: {OgRank(0)}
            afters[1]: {}
            before_and_afters[1]: {OgRank(2), OgRank(3)}
            befores[2]: {OgRank(0)}
            afters[2]: {}
            before_and_afters[2]: {OgRank(1), OgRank(3)}
            befores[3]: {OgRank(0)}
            afters[3]: {}
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
                        2,
                    ),
                ),
                (
                    NTraces(
                        3,
                    ),
                    CumSum(
                        4,
                    ),
                ),
                (
                    NTraces(
                        4,
                    ),
                    CumSum(
                        6,
                    ),
                ),
            ]
        "#]];
        let mut st = StreamingTranspositions::new(4, 1, 0.1);
        st.record_all(traces.iter().map(|it| OgRank2CurRank(it)));
        expected_before_and_after.assert_eq(&st.orderings().to_string());
        expected_cumsums.assert_debug_eq(&st.cumsums());
    }
}
