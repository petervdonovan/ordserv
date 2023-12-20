cd ..
cargo build -p viz --release
cd viz
time ../target/release/viz
# xdg-open ../plots/cumsums.png
