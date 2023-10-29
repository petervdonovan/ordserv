cd lf-264
cd core/src/main/resources/lib/c/reactor-c/core/federated/RTI
./build.sh
cd ../../../../../../../../../..
time ./gradlew assemble
COMMIT=$(git rev-parse HEAD | cut -c1-32)
rm -f build/install/lf-cli/bin/lfcpartest-*
mv build/install/lf-cli/bin/lfc build/install/lf-cli/bin/lfcpartest-$COMMIT
cd ..
