use std::path::PathBuf;

use clap::Parser;

use protocol_test::{state::State, CONCURRENCY_LIMIT};

const DEFAULT_CONCURRENCY_LIMIT: usize = 400;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
  src_dir: PathBuf,

  #[arg(short, long)]
  scratch_dir: Option<PathBuf>,

  #[arg(short, long)]
  max_wallclock_overhead: Option<i16>,

  #[arg(short, long)]
  concurrency: Option<usize>,

  #[arg(short, long)]
  once: bool,
}

const DEFAULT_SCRATCH_DIR: &str = "scratch";
const MIN_SAVE_INTERVAL_SECONDS: u32 = 90;
const MAX_SAVE_INTERVAL_SECONDS: u32 = 360;
const SAVE_INTERVAL_INCREASE_PER_ITERATION: u32 = 60;

fn main() {
  let args = Cli::parse();
  let scratch_dir = args
    .scratch_dir
    .unwrap_or(PathBuf::from(DEFAULT_SCRATCH_DIR));
  CONCURRENCY_LIMIT
    .set(args.concurrency.unwrap_or(DEFAULT_CONCURRENCY_LIMIT))
    .expect("impossible for the limit to already be set");
  std::fs::create_dir_all(&scratch_dir).expect("failed to create scratch dir");
  let delay_params = protocol_test::DelayParams {
    max_expected_wallclock_overhead: args
      .max_wallclock_overhead
      .map_or(5000, |max_wallclock_overhead| max_wallclock_overhead * 1000),
  };
  let mut state = State::load(args.src_dir, scratch_dir, delay_params);
  let mut save_interval = MIN_SAVE_INTERVAL_SECONDS;
  loop {
    state = state.run(save_interval);
    state.save_to_scratch_dir();
    if args.once {
      break;
    }
    save_interval =
      (save_interval + SAVE_INTERVAL_INCREASE_PER_ITERATION).min(MAX_SAVE_INTERVAL_SECONDS);
  }
}
