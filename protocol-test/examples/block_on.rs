use std::time::Duration;

fn main() {
  let rt = tokio::runtime::Runtime::new().unwrap();
  rt.block_on(async {
    tokio::task::spawn(async {
      std::thread::sleep(Duration::from_secs(20));
      println!("Hello from thread");
    });
    println!("Hello world");
  });
  // rt.shutdown_timeout(Duration::from_secs(2));
}
