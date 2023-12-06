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
  concurrency: Option<usize>,

  #[arg(short, long)]
  once: bool,

  #[arg(short, long)]
  frequency_of_save_in_seconds: Option<u32>,
}

const DEFAULT_SCRATCH_DIR: &str = "scratch";
const DEFAULT_SAVE_INTERVAL_SECONDS: u32 = 60;

fn main() {
  simple_logger::init_with_level(log::Level::Info).unwrap();
  console_subscriber::init();
  let args = Cli::parse();
  let scratch_dir = args
    .scratch_dir
    .unwrap_or(PathBuf::from(DEFAULT_SCRATCH_DIR));
  CONCURRENCY_LIMIT
    .set(args.concurrency.unwrap_or(DEFAULT_CONCURRENCY_LIMIT))
    .expect("impossible for the limit to already be set");
  std::fs::create_dir_all(&scratch_dir).expect("failed to create scratch dir");
  let mut state = State::load(args.src_dir, scratch_dir);
  let save_interval = args
    .frequency_of_save_in_seconds
    .unwrap_or(DEFAULT_SAVE_INTERVAL_SECONDS);
  loop {
    let (new_state, done) = state.run(save_interval);
    state = new_state;
    state.save_to_scratch_dir();
    if args.once || done {
      break;
    }
  }
}
