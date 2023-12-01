use std::time::Duration;

use ordering_server::{FederateId, HookInvocation};

fn main() {
    // let (client, _jh) = ordering_server::client::BlockingClient::start(
    //     ("127.0.0.1", 15045),
    //     FederateId(0),
    //     Duration::from_secs(5),
    // );
    // println!("Hello");
    // client.tracepoint_maybe_notify(HookInvocation::from_short(("A", 0, 0)));
}
