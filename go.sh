cd lf-264
cd core/src/main/resources/lib/c/reactor-c/core/federated/RTI
./build.sh || 0
cd ../../../../../../../../../..
time ./gradlew assemble
mv build/install/lf-cli/bin/lfc build/install/lf-cli/bin/lfcpartest
cd ..
cd protocol-test/
time cargo build
cd ..
time ./protocol-test/target/debug/protocol-test lf-264/test/C/src/federated/
