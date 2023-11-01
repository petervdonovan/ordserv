use std::path::PathBuf;

use clap::Parser;

use protocol_test::state::State;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
  src_dir: PathBuf,

  #[arg(short, long)]
  scratch_dir: Option<PathBuf>,

  #[arg(short, long)]
  max_wallclock_overhead: Option<u16>,
}

const DEFAULT_SCRATCH_DIR: &str = "scratch";
const MIN_SAVE_INTERVAL_SECONDS: u32 = 60;
const MAX_SAVE_INTERVAL_SECONDS: u32 = 3600;
const SAVE_INTERVAL_INCREASE_PER_ITERATION: u32 = 60;

fn main() {
  let args = Cli::parse();
  let scratch_dir = args
    .scratch_dir
    .unwrap_or(PathBuf::from(DEFAULT_SCRATCH_DIR));
  std::fs::create_dir_all(&scratch_dir).expect("failed to create scratch dir");
  let delay_params = protocol_test::DelayParams {
    max_expected_wallclock_overhead: args
      .max_wallclock_overhead
      .map_or(10e9 as u64, |max_wallclock_overhead| {
        max_wallclock_overhead as u64 * (1e9 as u64)
      }),
  };
  let mut state = State::load(args.src_dir, scratch_dir, delay_params);
  let mut save_interval = MIN_SAVE_INTERVAL_SECONDS;
  loop {
    state = state.run(save_interval);
    state.save_to_scratch_dir();
    save_interval =
      (save_interval + SAVE_INTERVAL_INCREASE_PER_ITERATION).min(MAX_SAVE_INTERVAL_SECONDS);
  }
}
