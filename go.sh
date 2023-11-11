ulimit -n 8192
cd protocol-test/
time cargo build --release
cd ..
time ./protocol-test/target/release/protocol-test lf-264/test/C/src/federated/
