cd viz264
RUSTFLAGS=-g cargo build --release
cd ..
LD_PRELOAD=./libbytehound.so ./viz264/target/release/viz264
bytehound server memory-profiling_*.dat
