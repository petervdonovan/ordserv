use ordering_server::HookInvocation;

fn main() {
    let (client, _jh) = ordering_server::client::BlockingClient::start(("127.0.0.1", 15045), 2);
    client.tracepoint_maybe_do(HookInvocation::from_short(("C99", 2, 0)));
    client.tracepoint_maybe_do(HookInvocation::from_short(("C0", 2, 0)));
    println!("            of");
    client.tracepoint_maybe_do(HookInvocation::from_short(("C0", 2, 1)));
    client.tracepoint_maybe_do(HookInvocation::from_short(("C1", 2, 0)));
    println!("            sentence");
    client.tracepoint_maybe_do(HookInvocation::from_short(("C1", 2, 1)));
    client.tracepoint_maybe_do(HookInvocation::from_short(("C1", 2, 2)));
    client.tracepoint_maybe_do(HookInvocation::from_short(("C1", 2, 3)));
    println!("            ordered");
    client.tracepoint_maybe_do(HookInvocation::from_short(("C1", 2, 4)));
    client.tracepoint_maybe_wait(HookInvocation::from_short(("C2", 2, 0)));
    println!("            the");
    client.tracepoint_maybe_notify(HookInvocation::from_short(("C2", 2, 0)));
    client.tracepoint_maybe_do(HookInvocation::from_short(("C2", 2, 1)));
    println!("            .");
}
