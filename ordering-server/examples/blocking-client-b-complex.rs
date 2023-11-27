use std::time::Duration;

use ordering_server::{FederateId, HookInvocation};

fn main() {
    // simple_logger::SimpleLogger::new().init().unwrap();
    let (client, _jh) = ordering_server::client::BlockingClient::start(
        ("127.0.0.1", 15045),
        FederateId(1),
        Duration::from_secs(5),
    );
    client.tracepoint_maybe_do(HookInvocation::from_short(("B99", 1, 0)));
    client.tracepoint_maybe_do(HookInvocation::from_short(("B0", 1, 0)));
    println!("      words");
    client.tracepoint_maybe_do(HookInvocation::from_short(("B0", 1, 0)));
    client.tracepoint_maybe_do(HookInvocation::from_short(("B0", 1, 1)));
    client.tracepoint_maybe_do(HookInvocation::from_short(("B1", 1, 0)));
    println!("      this");
    client.tracepoint_maybe_do(HookInvocation::from_short(("B1", 1, 1)));
    client.tracepoint_maybe_do(HookInvocation::from_short(("B2", 1, 0)));
    println!("      are");
    client.tracepoint_maybe_do(HookInvocation::from_short(("B5", 1, 0)));
    println!("      server");
    client.tracepoint_maybe_do(HookInvocation::from_short(("B6", 1, 0)));
}
