cargo build --release
sudo perf record -o /home/peter/school/264/project264/viz264/perf.data --call-graph dwarf --event cycles --aio --sample-cpu /home/peter/school/264/project264/viz264/target/release/viz264
sudo perf script -F +pid > /tmp/test.perf
