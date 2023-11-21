sudo perf record -o /home/peter/school/264/project264/perf.data --call-graph dwarf --event cycles --aio --sample-cpu /home/peter/school/264/project264/protocol-test/target/release/protocol-test -c 20 -o -f 2 lf-264/test/C/src/federated/
sudo perf script -F +pid > /tmp/test.perf
