use ordering_server::HookInvocation;

fn main() {
    let (client, _jh) = ordering_server::client::BlockingClient::start(("127.0.0.1", 15045), 1);
    println!("B did startup");
    client.tracepoint_maybe_wait(HookInvocation::from_short(("B", 1, 0)));
    println!("      world.");
}
