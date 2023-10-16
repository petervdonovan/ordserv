use std::{collections::HashMap, path::PathBuf, process::Command};

use regex::Regex;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct HookId(String);

fn get_counts(executable: PathBuf) -> HashMap<HookId, u32> {
  let output = Command::new(executable.as_os_str())
    .env("LF_LOGTRACE", "YES")
    .output()
    .expect("failed to execute subprocess");
  if !output.status.success() {
    println!("Failed to get correct initial counts for {executable:?}. Re-running.");
    return get_counts(executable);
  }
  let regex = Regex::new(r"<<< (?<HookId>.*) >>>").unwrap();
  let mut ret = HashMap::new();
  for line in std::str::from_utf8(&output.stdout)
    .expect("expected output to be UTF-8")
    .lines()
  {
    if let Some(caps) = regex.captures(line) {
      let hid = HookId(caps["HookId"].to_string());
      let next = ret.get(&hid).unwrap_or(&0) + 1;
      ret.insert(hid, next);
    }
  }
  ret
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;

  fn tests_relpath() -> PathBuf {
    PathBuf::from("../../../../lingua-franca/test/C/bin")
  }

  #[test]
  fn test_get_counts() {
    for entry in tests_relpath()
      .read_dir()
      .expect("read_dir call failed")
      .flatten()
    {
      let counts = get_counts(entry.path());
      println!("{counts:?}");
    }
  }
}
