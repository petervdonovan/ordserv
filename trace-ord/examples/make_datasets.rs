use std::{collections::HashSet, ffi::OsStr, path::Path, process::Command};

fn test_name(test: &OsStr) -> String {
    let test = test.to_str().unwrap();
    let test = test.split('-').next().unwrap();
    test.to_string()
}

fn main() {
    let mut already_run = HashSet::new();
    for entry in std::fs::read_dir("testc/bin").unwrap() {
        let name = test_name(&entry.as_ref().unwrap().file_name());
        if already_run.contains(&name) {
            continue;
        }
        let entry = entry.unwrap();
        already_run.insert(name.clone());
        let cwd = Path::new("trace-ord/datasets").join(name);
        std::fs::create_dir_all(&cwd).unwrap();
        Command::new(
            Path::new(&entry.path())
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap(),
        )
        .env("LF_CONNECTION_INFO_FILE", &format!("conninfo.txt"))
        .current_dir(&cwd)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
        for entry in std::fs::read_dir(&cwd).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path().canonicalize().unwrap();
            if path.extension().unwrap() == "lft" {
                println!("Converting {:?} to CSV", path);
                let ret = Command::new("trace_to_csv")
                    .current_dir(&cwd)
                    .arg(path.file_name().unwrap())
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();
                assert!(ret.success());
            }
        }
    }
}
