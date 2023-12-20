cd ./viz
cargo build
cd ..
./viz/target/debug/viz
xdg-open plots/throughput.png
