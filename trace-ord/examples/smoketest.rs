use std::path::Path;

use trace_ord::{tracerecords_to_string, Event};

const IGNORE_TESTS: [&str; 1] = [
    "DistributedNetworkOrder", // This test directly invokes send_timed_message, which is an implementation detail!  >:(
];

const MAX_TRACE_LENGTH: usize = 200;

const COMPUTED_PRECEDENCES_FILENAME: &str = "computed_precedences.mpk";
const DATASETS_PATH: &str = "trace-ord/datasets";

pub fn main() {
    let datasets_path = Path::new(DATASETS_PATH);
    let mut entries: Vec<_> = std::fs::read_dir(datasets_path)
        .unwrap()
        .filter(|entry| entry.as_ref().unwrap().metadata().unwrap().is_dir())
        .collect();
    entries.sort_by_key(|it| it.as_ref().unwrap().file_name());
    let mut ax2nuseses = vec![];
    let mut cp = trace_ord::serde::ComputedPrecedences::default();
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path().canonicalize().unwrap();
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        if IGNORE_TESTS.contains(&name.as_str()) {
            continue;
        }
        let ogtrace = lf_trace_reader::trace_by_physical_time(&path.join("rti.csv"));
        if ogtrace.len() > MAX_TRACE_LENGTH {
            println!("Skipping {} because it is too long", name);
            continue;
        }
        // if name != "SimpleFederated" {
        //     continue;
        // }
        let (trace, conninfo, preceding_permutables) =
            trace_ord::preceding_permutables_by_ogrank_from_dir(&path);
        let (ax2nuses, preceding_permutables) = preceding_permutables.unwrap_or_else(|err| {
            println!("Error: {}", err);
            panic!("Fatal error during processing of dataset from {:?}", path);
        });
        cp.add_test(
            name.clone(),
            ogtrace,
            trace,
            preceding_permutables.clone(),
            conninfo,
        );
        ax2nuseses.push(ax2nuses);
        // println!("{}", tracerecords_to_string(&trace[..], true, |_| false));
        // for (ogrank, permutables) in preceding_permutables.iter().enumerate() {
        //     let mut sample = permutables.iter().map(|it| it.0).collect::<Vec<_>>();
        //     sample.sort();
        //     println!("Permutable with {}:\n    {:?}", ogrank, sample);
        // }
        let n_permutables = cp.n_permutables(&name);
        let max_permutables = cp.max_n_permutables(&name);
        println!(
            "Total number of permutables in {}: {} / {}",
            name, n_permutables, max_permutables
        );
        if n_permutables == 0 {
            continue;
        }
    }
    println!("Total # constraints added by axiom (ignoring redundancy and consequences of transitivity):");
    for (ax, nuses) in ax2nuseses[1..]
        .iter()
        .fold(ax2nuseses[0].clone(), |mut acc, next| {
            for (ax, nuses) in next.iter() {
                *acc.get_mut(ax).unwrap() += *nuses;
            }
            acc
        })
    {
        println!("    {} uses of {}", nuses, ax);
    }
    println!("Geometric mean: {}", cp.geomean_n_permutables_normalized());
    std::fs::write(
        datasets_path.join(COMPUTED_PRECEDENCES_FILENAME),
        rmp_serde::to_vec(&cp).unwrap(),
    )
    .unwrap();
}
