cd ..
cargo build -p viz264 --release
cd viz264
time ../target/release/viz264
xdg-open plots/cumsums.png
