use std::time::Duration;

use ordering_server::{FederateId, HookInvocation};

fn main() {
    let (client, _jh) = ordering_server::client::BlockingClient::start(
        ("127.0.0.1", 15045),
        FederateId(1),
        Duration::from_secs(5),
    );
    println!("B did startup");
    client.tracepoint_maybe_wait(HookInvocation::from_short(("B", 1, 0)));
    println!("      world.");
}
