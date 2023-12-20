ulimit -n 200000 || 0
# ulimit -u 200000 || 0
# ulimit -s 4096
# echo 200000 > /proc/sys/kernel/threads-max
time cargo build --release -p protocol-test
time cargo build --release -p c-ordering-client
time ./target/release/protocol-test -c 50 -f 60 lf-ordserv/test/C/src/federated/
