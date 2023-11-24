cd ../../
cargo build -p c-ordering-client
cd c-ordering-client/examples
gcc smoketest.c
./a.out
rm ./a.out
