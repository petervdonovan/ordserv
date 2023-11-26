use std::process::Command;

use ordering_server::{server, Precedence};

#[tokio::main]
async fn main() {
    let mut server_handle = server::run(15045, 1).await;
    let precedence = Precedence::from_list(2, &[(("A", 0, 0), &[("B", 1, 0)])], "/tmp".into(), 0);
    server_handle.updates_acks[0]
        .0
        .send(Some(precedence))
        .await
        .unwrap();
    let evars = server_handle.updates_acks[0].1.recv().await.unwrap();
    println!("Received ack");
    let mut child_a = Command::new("cargo")
        .args(["run", "--example", "blocking-client-a"])
        .envs(evars.0.clone())
        .spawn()
        .expect("failed to execute process");
    let mut child_b = Command::new("cargo")
        .args(["run", "--example", "blocking-client-b"])
        .envs(evars.0)
        .spawn()
        .expect("failed to execute process");
    child_a.wait().unwrap();
    child_b.wait().unwrap();
    server_handle.updates_acks[0].0.send(None).await.unwrap();
    server_handle.join_handle.await.unwrap();
    println!("Server finished");
}
