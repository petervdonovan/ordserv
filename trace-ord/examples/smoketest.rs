use std::{collections::HashSet, path::Path};

use trace_ord::elaborated_from_lf_trace_records;

pub fn main() {
    let trace_path = Path::new("trace-ord/examples/smoketest.csv");
    let traces = lf_trace_reader::trace_by_physical_time(trace_path);
    let (traces, _map2og) = elaborated_from_lf_trace_records(traces);
    for (k, trace) in traces.iter().enumerate() {
        println!("{k} {}", trace);
    }
    let axioms = trace_ord::axioms::axioms();
    let always_occurring: HashSet<_> = (0..traces.len())
        .map(|ogrank| trace_ord::OgRank(ogrank as u32))
        .collect();
    let preceding_permutables =
        trace_ord::preceding_permutables_by_ogrank(&traces, &axioms, always_occurring);
    for (ogrank, permutables) in preceding_permutables.iter().enumerate() {
        let mut sample = permutables.iter().map(|it| it.0).collect::<Vec<_>>();
        sample.sort();
        println!("\nPermutable with {}:\n    {:?}", ogrank, sample);
    }
    println!(
        "Total number of permutables: {}",
        preceding_permutables
            .iter()
            .map(|it| it.len())
            .sum::<usize>()
    );
}
