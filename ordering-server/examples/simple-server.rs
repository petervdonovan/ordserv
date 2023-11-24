use ordering_server::{server, Precedence};

#[tokio::main]
async fn main() {
    let mut server_handle = server::run(15045, 1);
    let precedence = Precedence::from_list(2, &[(("A", 0, 0), &[("B", 1, 0)])], "/tmp".into());
    server_handle.updates_acks[0]
        .0
        .send(Some(precedence))
        .await
        .unwrap();
    server_handle.updates_acks[0].1.recv().await.unwrap();
    println!("Received ack");
    server_handle.updates_acks[0].0.send(None).await.unwrap();
    server_handle.join_handle.await.unwrap();
    println!("Server finished");
}
