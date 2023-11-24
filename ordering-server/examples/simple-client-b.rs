#[tokio::main]
async fn main() {
    let (mut client, _jh) =
        ordering_server::client::ChannelClient::start(("127.0.0.1", 15045)).await;
    client
        .write(ordering_server::Frame {
            precedence_id: 0,
            hook_id: [b'B'; 32],
            federate_id: 0,
            sequence_number: 0,
        })
        .await;
    println!("Sent frame");
    client.frames.recv().await.unwrap();
    println!("\n      world.");
}
