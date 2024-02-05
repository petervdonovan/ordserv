use std::collections::HashSet;

use lf_trace_reader::TraceRecord;
use serde::{Deserialize, Serialize};

use crate::{conninfo::ConnInfo, lflib::ConcEvent, lflib::OgRank};

/// A map from test name to a map from ogranks to the lists of preceding ogranks which can appear
/// later.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComputedPrecedences(pub Vec<(String, OgtraceOgtracePrecs)>);
pub type OgtraceOgtracePrecs = (Vec<TraceRecord>, Vec<Vec<OgRank>>, ConnInfo);

impl ComputedPrecedences {
    pub fn add_test(
        &mut self,
        test_name: String,
        ogtrace: Vec<TraceRecord>,
        etrace: Vec<ConcEvent>,
        permutables: Vec<HashSet<OgRank>>,
        conninfo: ConnInfo,
    ) {
        let filtered = permutables
            .into_iter()
            .zip(etrace.iter())
            // .filter(|(_, e)| matches!(e, crate::event::Event::Concrete { .. }))
            .map(|(ogrs, _)| {
                let mut collected = ogrs
                    .into_iter()
                    .filter_map(|ogr| match etrace[ogr.idx()] {
                        ConcEvent { ogrank, .. } => Some(ogrank),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                collected.sort();
                collected
            })
            .collect::<Vec<_>>();
        assert!(filtered.len() == ogtrace.len());
        assert!(filtered
            .iter()
            .enumerate()
            .all(|(idx, ogrs)| ogrs.iter().all(|ogr| ogr.idx() < idx)));
        self.0.push((test_name, (ogtrace, filtered, conninfo)));
    }
    pub fn get(&self, test_name: &str) -> &(Vec<TraceRecord>, Vec<Vec<OgRank>>, ConnInfo) {
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
