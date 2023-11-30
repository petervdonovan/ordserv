ulimit -n 200000 || 0
time cargo build --release -p protocol-test
time cargo build --release -p c-ordering-client
./target/release/protocol-test -c 20 -f 10 -o lf-264/test/C/src/federated/
