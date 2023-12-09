const IGNORE_TESTS: [&str; 1] = [
    "DistributedNetworkOrder", // This test directly invokes send_timed_message, which is an implementation detail!  >:(
];

pub fn main() {
    let mut entries: Vec<_> = std::fs::read_dir("trace-ord/datasets").unwrap().collect();
    entries.sort_by_key(|it| it.as_ref().unwrap().file_name());
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path().canonicalize().unwrap();
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        if IGNORE_TESTS.contains(&name.as_str()) {
            continue;
        }
        // if name != "SpuriousDependency" {
        //     continue;
        // }
        let preceding_permutables = trace_ord::preceding_permutables_by_ogrank_from_dir(&path)
            .unwrap_or_else(|err| {
                println!("Error: {}", err);
                panic!("Fatal error during processing of dataset from {:?}", path);
            });
        for (ogrank, permutables) in preceding_permutables.iter().enumerate() {
            let mut sample = permutables.iter().map(|it| it.0).collect::<Vec<_>>();
            sample.sort();
            println!("Permutable with {}:\n    {:?}", ogrank, sample);
        }
        println!(
            "Total number of permutables in {}: {} / {}",
            name,
            preceding_permutables
                .iter()
                .map(|it| it.len())
                .sum::<usize>(),
            preceding_permutables.len() * (preceding_permutables.len() - 1) / 2
        );
    }
}
