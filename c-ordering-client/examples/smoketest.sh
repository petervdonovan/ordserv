cd ../../
cargo build -p c-ordering-client
cd c-ordering-client/examples
gcc smoketest.c ../../target/debug/libc_ordering_client.a
./a.out
rm ./a.out
