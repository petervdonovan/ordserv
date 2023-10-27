cd lf-264
./gradlew assemble
mv build/install/lf-cli/bin/lfc build/install/lf-cli/bin/lfcpartest
cd ../protocol-test/
cargo build
cd ..
./protocol-test/target/debug/protocol-test lf-264/test/C/src/federated/
