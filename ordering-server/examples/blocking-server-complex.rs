use std::process::Command;

use ordering_server::{server, Precedence};

#[tokio::main]
async fn main() {
    // simple_logger::SimpleLogger::new().init().unwrap();
    let mut server_handle = server::run(15045, 1).await;
    let precedence = Precedence::from_list(
        3,
        // Athe A0 B0 Bwords B0 A1 C0 Cof C0 B1 Bthis A1 B1 C1 Csentence C1 B1 C1 A2 Bare B2 C1 Cordered C1 A3 Aby A4 C2 Cthe C2' A4 Aordering A4' B5 Bserver B6 C2.
        &[
            (("B99", 1, 0), &[("C99", 2, 0)]),
            (("C99", 2, 0), &[("A99", 0, 0)]),
            (("A0", 0, 0), &[("B0", 1, 0)]),               // words
            (("B0", 1, 1), &[("A1", 0, 0), ("C0", 2, 0)]), // of
            (("C0", 2, 1), &[("B1", 1, 0), ("A1", 0, 1)]), // this
            (("A1", 0, 1), &[("B1", 1, 1), ("C1", 2, 0)]), // sentence
            (("C1", 2, 1), &[("B1", 1, 2), ("C1", 2, 2)]), //
            (("A2", 0, 0), &[("B1", 1, 3)]),               // are
            (("B2", 1, 0), &[("C1", 2, 3)]),               // ordered
            (("C1", 2, 4), &[("A3", 0, 0)]),               // by
            (("A4", 0, 0), &[("C2", 2, 0)]),               // the
            (("C2", 2, 0), &[("A4", 0, 1)]),               // ordering
            (("A4", 0, 1), &[("B5", 1, 0)]),               // server
            (("B6", 1, 0), &[("C2", 2, 1)]),               // .
        ],
        "/tmp".into(),
        0,
    );
    server_handle.updates_acks[0]
        .0
        .send(Some(precedence))
        .await
        .unwrap();
    let evars = server_handle.updates_acks[0].1.recv().await.unwrap();
    let mut child_a = Command::new("cargo")
        .args(["run", "--example", "blocking-client-a-complex"])
        .envs(evars.0.clone())
        .spawn()
        .expect("failed to execute process");
    let mut child_b = Command::new("cargo")
        .args(["run", "--example", "blocking-client-b-complex"])
        .envs(evars.0.clone())
        .spawn()
        .expect("failed to execute process");
    let mut child_c = Command::new("cargo")
        .args(["run", "--example", "blocking-client-c-complex"])
        .envs(evars.0)
        .spawn()
        .expect("failed to execute process");
    child_a.wait().unwrap();
    child_b.wait().unwrap();
    child_c.wait().unwrap();
    server_handle.updates_acks[0].0.send(None).await.unwrap();
    server_handle.join_handle.await.unwrap();
    println!("Server finished");
}
