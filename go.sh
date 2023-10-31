ulimit -n 8192
cd protocol-test/
time cargo build
cd ..
time ./protocol-test/target/debug/protocol-test lf-264/test/C/src/federated/
