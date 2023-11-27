use std::time::Duration;

use ordering_server::{FederateId, HookInvocation};

fn main() {
    // simple_logger::SimpleLogger::new().init().unwrap();
    let (client, _jh) = ordering_server::client::BlockingClient::start(
        ("127.0.0.1", 15045),
        FederateId(0),
        Duration::from_secs(5),
    );
    client.tracepoint_maybe_do(HookInvocation::from_short(("A99", 0, 0)));
    println!("the");
    client.tracepoint_maybe_do(HookInvocation::from_short(("A0", 0, 0)));
    client.tracepoint_maybe_do(HookInvocation::from_short(("A1", 0, 0)));
    client.tracepoint_maybe_do(HookInvocation::from_short(("A1", 0, 1)));
    client.tracepoint_maybe_do(HookInvocation::from_short(("A2", 0, 0)));
    client.tracepoint_maybe_do(HookInvocation::from_short(("A3", 0, 0)));
    println!("by");
    client.tracepoint_maybe_do(HookInvocation::from_short(("A4", 0, 0)));
    client.tracepoint_maybe_wait(HookInvocation::from_short(("A4", 0, 1)));
    println!("ordering");
    client.tracepoint_maybe_notify(HookInvocation::from_short(("A4", 0, 1)));
}
