#[tokio::main]
async fn main() {
    // let (mut client, _jh) =
    //     ordering_server::client::ChannelClient::start(("127.0.0.1", 15045)).await;
    // client
    //     .write(ordering_server::Frame {
    //         precedence_id: 0,
    //         hook_id: [b'S'; 32],
    //         federate_id: 1,
    //         sequence_number: 0,
    //         run_id: 0,
    //     })
    //     .await;
    // println!("Sent frame");
    // // let mut hook_id = [0; 32];
    // // hook_id[0] = b'B';
    // // client
    // //     .write(ordering_server::Frame {
    // //         precedence_id: 0,
    // //         hook_id,
    // //         federate_id: 0,
    // //         sequence_number: 0,
    // //         run_id: 0,
    // //     })
    // //     .await;
    // client.frames.recv().await.unwrap();
    // println!("\n      world.");
}
