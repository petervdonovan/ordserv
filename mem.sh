cd viz
RUSTFLAGS=-g cargo build --release
cd ..
LD_PRELOAD=./libbytehound.so ./viz/target/release/viz
bytehound server memory-profiling_*.dat
