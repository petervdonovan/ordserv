use trace_ord::EventPerTraceUniqueId;

const IGNORE_TESTS: [&str; 1] = [
    "DistributedNetworkOrder", // This test directly invokes send_timed_message, which is an implementation detail!  >:(
];

const MAX_TRACE_LENGTH: usize = 200;

pub fn main() {
    let mut entries: Vec<_> = std::fs::read_dir("trace-ord/datasets").unwrap().collect();
    entries.sort_by_key(|it| it.as_ref().unwrap().file_name());
    let mut geomean = 1.0;
    let mut count = 0;
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path().canonicalize().unwrap();
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        if IGNORE_TESTS.contains(&name.as_str()) {
            continue;
        }
        if lf_trace_reader::trace_by_physical_time(&path.join("rti.csv")).len() > MAX_TRACE_LENGTH {
            println!("Skipping {} because it is too long", name);
            continue;
        }
        // if name != "SpuriousDependency" {
        //     continue;
        // }
        let (trace, preceding_permutables) =
            trace_ord::preceding_permutables_by_ogrank_from_dir(&path);
        let preceding_permutables = preceding_permutables.unwrap_or_else(|err| {
            println!("Error: {}", err);
            panic!("Fatal error during processing of dataset from {:?}", path);
        });
        // for (ogrank, permutables) in preceding_permutables.iter().enumerate() {
        //     let mut sample = permutables.iter().map(|it| it.0).collect::<Vec<_>>();
        //     sample.sort();
        //     println!("Permutable with {}:\n    {:?}", ogrank, sample);
        // }
        let (len, n_permutables) = preceding_permutables
            .iter()
            .enumerate()
            .filter_map(|(ogrank, permutables)| {
                if let EventPerTraceUniqueId::First(_) = trace[ogrank].unique_id {
                    None
                } else {
                    Some(permutables)
                }
            })
            .map(|it| {
                it.iter()
                    .filter(|ogr| {
                        !matches!(trace[ogr.idx()].unique_id, EventPerTraceUniqueId::First(_))
                    })
                    .count()
            })
            .fold((0, 0), |(len, n_permutables_sum), n_permutables| {
                (len + 1, n_permutables_sum + n_permutables)
            });
        let max_permutables = len * (len - 1) / 2;
        geomean *= n_permutables as f64 / max_permutables as f64;
        count += 1;
        println!(
            "Total number of permutables in {}: {} / {}",
            name, n_permutables, max_permutables
        );
    }
    geomean = geomean.powf(1.0 / count as f64);
    println!("Geometric mean: {}", geomean);
}
