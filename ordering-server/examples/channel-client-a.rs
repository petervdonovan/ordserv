#[tokio::main]
async fn main() {
    let (mut client, _jh) =
        ordering_server::client::ChannelClient::start(("127.0.0.1", 15045)).await;
    client
        .write(ordering_server::Frame {
            precedence_id: 0,
            hook_id: [b'S'; 32],
            federate_id: 0,
            sequence_number: 0,
            run_id: 0,
        })
        .await;
    println!("Hello");
    let mut hook_id = [0; 32];
    hook_id[0] = b'A';
    client
        .write(ordering_server::Frame {
            precedence_id: 0,
            hook_id,
            federate_id: 0,
            sequence_number: 0,
            run_id: 0,
        })
        .await;
}
