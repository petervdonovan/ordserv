use std::{collections::HashSet, path::PathBuf};

use streaming_transpositions::{OgRank2CurRank, StreamingTranspositions};
use trace_ord::conninfo;
use viz264::{
    compare_permutable_sets, describe_permutable_sets, error_rate, get_atses, get_latest_ats,
    get_trace_ords, runs_over_time_chart,
};

fn do_compare_permutable_sets() {
    let ats_a = get_latest_ats(&PathBuf::from("scratcha"));
    let ats_b = get_latest_ats(&PathBuf::from("scratchb"));
    compare_permutable_sets(&ats_a, &ats_b);
}

fn do_describe_permutable_sets() {
    let ats = get_latest_ats(&PathBuf::from("scratch"));
    describe_permutable_sets(&ats);
}

fn do_throughput_and_error_rate() {
    let scratch = &PathBuf::from("scratch");
    let atses = get_atses(scratch);
    let latest = get_latest_ats(scratch);
    runs_over_time_chart(&atses, "plots/runs_over_time.png");
    error_rate(&latest, "plots/error_rate.png");
}

fn all_permutables_from_preceding_permutables(
    preceding_permutables: &Vec<Vec<trace_ord::OgRank>>,
    ogtrace: &[lf_trace_reader::TraceRecord],
) -> Vec<HashSet<trace_ord::OgRank>> {
    let mut all_permutables = Vec::with_capacity(ogtrace.len());
    for _ in 0..ogtrace.len() {
        all_permutables.push(HashSet::new());
    }
    for (og, permutables) in preceding_permutables.iter().enumerate() {
        for ogr in permutables.iter() {
            all_permutables[ogr.idx()].insert(trace_ord::OgRank(og as u32));
            all_permutables[og].insert(*ogr);
        }
    }
    all_permutables
}

fn empirical_all_permutables_translated(
    strans_out: &StreamingTranspositions,
    ogog2og: &OgRank2CurRank,
    len: usize,
) -> Vec<HashSet<trace_ord::OgRank>> {
    let mut empirical_all_permutables = Vec::with_capacity(len);
    let mut hit_ogs = HashSet::new();
    for _ in 0..len {
        empirical_all_permutables.push(HashSet::new());
    }
    for (og, ogr) in strans_out.orderings().iter().enumerate() {
        let current_idx = ogog2og.0[og].0 as usize;
        if current_idx >= len {
            continue;
        }
        hit_ogs.insert(current_idx);
        let current: &mut HashSet<trace_ord::OgRank> = &mut empirical_all_permutables[current_idx];
        for permutable in ogr.iter().filter(|permutable| permutable.0 < len as u32) {
            current.insert(trace_ord::OgRank(ogog2og.0[permutable.idx()].0));
        }
    }
    // for (k, overapproximation) in empirical_all_permutables.iter_mut().enumerate() {
    //     if !hit_ogs.contains(&k) {
    //         *overapproximation = (0..k)
    //             .chain(k + 1..len)
    //             .map(|it| trace_ord::OgRank(it as u32))
    //             .collect();
    //     }
    // }
    empirical_all_permutables
}

fn do_computed_precedences() {
    let cp_path = PathBuf::from("trace-ord/datasets/computed_precedences.mpk");
    let scratch = &PathBuf::from("scratch");
    let cp = get_trace_ords(&cp_path);
    let ats = get_latest_ats(scratch);
    for (tid, runs) in ats.runs {
        let name = &ats.kcs.executables().get(&tid).unwrap().name();
        println!("Processing {}", name);
        if !cp.0.iter().any(|(cpname, _)| cpname == name) {
            println!("Skipping {}", name);
            continue;
        }
        let strans_out = &runs.read().unwrap().strans_out;
        let (ogtrace, preceding_permutables, _conninfo) = cp.get(name);
        let (ogog2og, _, _) = ats
            .kcs
            .metadata(&tid)
            .out_ovkey
            .vectorfy(ogtrace.iter().cloned());

        let all_permutables =
            all_permutables_from_preceding_permutables(preceding_permutables, ogtrace);
        let empirical_all_permutables =
            empirical_all_permutables_translated(strans_out, &ogog2og, all_permutables.len());
        let mut sum = 0.0;
        let mut count = 0;
        let mut nontrivials = 0;
        let mut nontrivial_oks = 0;
        for (upper_bound, lower_bound) in
            all_permutables.iter().zip(empirical_all_permutables.iter())
        {
            let ok = upper_bound.is_superset(lower_bound);
            let ratio = lower_bound.len() as f64 / upper_bound.len() as f64;
            // print!(
            //     "{} = {} / {} (ok={})  ",
            //     ratio,
            //     lower_bound.len(),
            //     upper_bound.len(),
            //     ok
            // );
            if ok {
                sum += ratio;
                count += 1;
            }
            if !lower_bound.is_empty() {
                nontrivials += 1;
                if ok {
                    nontrivial_oks += 1;
                }
            }
            // assert!(upper_bound.is_superset(lower_bound));
        }
        println!(
            "\nmean = {}, proportion nontrivials ok = {}",
            sum / count as f64,
            nontrivial_oks as f64 / nontrivials as f64
        );
    }
}

fn do_check_axioms() {
    let cp_path = PathBuf::from("trace-ord/datasets/computed_precedences.mpk");
    let scratch = &PathBuf::from("scratch");
    let ats = get_latest_ats(scratch);
    let axioms = trace_ord::axioms::axioms();
    let cp = get_trace_ords(&cp_path);
    let mut bad = HashSet::new();
    for (tid, runs) in ats.runs {
        let name = &ats.kcs.executables().get(&tid).unwrap().name();
        if !cp.0.iter().any(|(cpname, _)| cpname == name) {
            println!("Skipping {}", name);
            continue;
        }
        println!("Processing {}", name);
        let traces = &runs.read().unwrap().raw_traces;
        let conninfo = &cp.get(name).2;
        for ax in axioms.iter() {
            let mut ok = true;
            let mut failures = 0;
            let mut count = 0;
            for trace in traces.iter().filter_map(|it| match it {
                (_, Ok((ov, _, _))) => Some(ov),
                _ => None,
            }) {
                let trace_records = ats.kcs.metadata(&tid).out_ovkey.records.clone();
                let ogrank2currank = trace.unpack(&ats.ovr);
                let mut trace_records = trace_records
                    .iter()
                    .cloned()
                    .enumerate()
                    .filter(|(ogr, _)| ogrank2currank[*ogr] < trace.sentinel())
                    .collect::<Vec<_>>();
                trace_records.sort_by_key(|(ogr, _)| ogrank2currank[*ogr].0);
                let trace_records = trace_records
                    .into_iter()
                    .map(|(_, tr)| tr)
                    .collect::<Vec<_>>();
                let elaborated =
                    trace_ord::elaborated_from_trace_records(trace_records, &axioms, conninfo);
                if let Err(e) = ax.check(&elaborated, conninfo) {
                    ok = false;
                    failures += 1;
                    println!("{}", e);
                }
                count += 1;
            }
            if ok {
                println!("  Axiom ok: {}", ax);
            } else {
                println!(
                    "\n  Axiom not ok: {} ({} / {} = {} bad)\n",
                    ax,
                    failures,
                    count,
                    failures as f64 / count as f64
                );
                bad.insert(ax);
            }
        }
    }
    for ax in axioms.iter().filter(|it| !bad.contains(it)) {
        println!("Good: {}", ax);
    }
    for ax in bad.iter() {
        println!("Bad: {}", ax);
    }
}

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();
    std::env::set_current_dir("..").unwrap();
    // do_throughput_and_error_rate();
    // do_describe_permutable_sets();
    // do_compare_permutable_sets();
    // do_computed_precedences();
    do_check_axioms();
}
