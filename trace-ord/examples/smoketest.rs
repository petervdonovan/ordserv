pub fn main() {
    let mut entries: Vec<_> = std::fs::read_dir("trace-ord/datasets").unwrap().collect();
    entries.sort_by_key(|it| it.as_ref().unwrap().file_name());
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path().canonicalize().unwrap();
        let preceding_permutables = trace_ord::preceding_permutables_by_ogrank_from_dir(&path)
            .unwrap_or_else(|err| {
                println!("Error: {}", err);
                panic!("Fatal error during processing of dataset from {:?}", path);
            });
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
}
