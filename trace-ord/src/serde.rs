use std::collections::{HashMap, HashSet};

use lf_trace_reader::TraceRecord;
use serde::{Deserialize, Serialize};

use crate::{Event, OgRank};

/// A map from test name to a map from ogranks to the lists of preceding ogranks which can appear
/// later.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComputedPrecedences(pub Vec<(String, (Vec<TraceRecord>, Vec<Vec<OgRank>>))>);

impl ComputedPrecedences {
    pub fn add_test(
        &mut self,
        test_name: String,
        ogtrace: Vec<TraceRecord>,
        etrace: Vec<Event>,
        precedences: Vec<HashSet<OgRank>>,
    ) {
        let filtered = precedences
            .into_iter()
            .zip(etrace.iter())
            .filter(|(_, e)| matches!(e, Event::Concrete { .. }))
            .map(|(ogrs, _)| {
                let mut collected = ogrs
                    .into_iter()
                    .filter(|ogr| matches!(etrace[ogr.idx()], Event::Concrete { .. }))
                    .collect::<Vec<_>>();
                collected.sort();
                collected
            })
            .collect();
        self.0.push((test_name, (ogtrace, filtered)));
    }
    pub fn get(&self, test_name: &str) -> &(Vec<TraceRecord>, Vec<Vec<OgRank>>) {
        self.0
            .iter()
            .find(|(name, _)| name == test_name)
            .map(|(_, it)| it)
            .unwrap()
    }
    pub fn n_permutables(&self, test_name: &str) -> usize {
        self.get(test_name)
            .1
            .iter()
            .map(|it| it.len())
            .sum::<usize>()
    }
    pub fn max_n_permutables(&self, test_name: &str) -> usize {
        let len = self.get(test_name).1.len();
        len * (len - 1) / 2
    }
    pub fn geomean_n_permutables_normalized(&self) -> f64 {
        let mut prod: f64 = 1.0;
        let mut count = 0;
        for key in self.0.iter().map(|it| &it.0) {
            let n_permutables = self.n_permutables(key);
            let max_permutables = self.max_n_permutables(key);
            if n_permutables > 0 {
                prod *= n_permutables as f64 / max_permutables as f64;
                count += 1;
            }
        }
        prod.powf(1.0 / count as f64)
    }
}
