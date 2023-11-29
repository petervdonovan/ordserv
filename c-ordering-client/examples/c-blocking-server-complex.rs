use std::process::Command;

use ordering_server::{
    server, EnvironmentVariables, Precedence, ORDSERV_PORT_ENV_VAR,
    ORDSERV_WAIT_TIMEOUT_MILLISECONDS_ENV_VAR,
};

const PORT: u16 = 15045;

fn compile(name: String) {
    let mut child = Command::new("gcc")
        .args([
            "-g",
            "-Wall",
            "-o",
            &format!("./c-ordering-client/examples/{}.run", name),
            "-I",
            "./c-ordering-client/examples/",
            "./c-ordering-client/examples/c-ordering-client.c",
            &format!("./c-ordering-client/examples/{}.c", name),
        ])
        .spawn()
        .unwrap();
    if !child.wait().unwrap().success() {
        panic!("failed to compile");
    }
}

fn run_federate(name: String, runtime_evars: &EnvironmentVariables) -> std::thread::JoinHandle<()> {
    let evars = runtime_evars.0.clone();
    std::thread::spawn(move || {
        let mut child = Command::new(format!("./c-ordering-client/examples/{}.run", name))
            .envs(evars)
            .env(ORDSERV_PORT_ENV_VAR, PORT.to_string())
            .env(ORDSERV_WAIT_TIMEOUT_MILLISECONDS_ENV_VAR, "50000000000")
            .env(
                "C_ORDERING_CLIENT_LIBRARY_PATH",
                "./target/debug/libc_ordering_client.so",
            )
            .spawn()
            .expect("failed to execute process");
        let result = child.wait().expect("failed to wait for child");
        if !result.success() {
            panic!("failure {}", name);
        }
    })
}

#[tokio::main]
async fn main() {
    // simple_logger::SimpleLogger::new().init().unwrap();
    let mut server_handle = server::run_reusing_connections(1, 16).await;
    let precedence = Precedence::from_list(
        3,
        // Athe A0 B0 Bwords B0 A1 C0 Cof C0 B1 Bthis A1 B1 C1 Csentence C1 B1 C1 A2 Bare B2 C1 Cordered C1 A3 Aby A4 C2 Cthe C2' A4 Aordering A4' B5 Bserver B6 C2.
        &[
            (("B99", 1, 0), &[("C99", -1, 0)]),
            (("C99", -1, 0), &[("A99", 0, 0)]),
            (("A0", 0, 0), &[("B0", 1, 0)]),                 // words
            (("B0", 1, 1), &[("A1", 0, 0), ("C0", -1, 0)]),  // of
            (("C0", -1, 1), &[("B1", 1, 0), ("A1", 0, 1)]),  // this
            (("A1", 0, 1), &[("B1", 1, 1), ("C1", -1, 0)]),  // sentence
            (("C1", -1, 1), &[("B1", 1, 2), ("C1", -1, 2)]), //
            (("A2", 0, 0), &[("B1", 1, 3)]),                 // are
            (("B2", 1, 0), &[("C1", -1, 3)]),                // ordered
            (("C1", -1, 4), &[("A3", 0, 0)]),                // by
            (("A4", 0, 0), &[("C2", -1, 0)]),                // the
            (("C2", -1, 0), &[("A4", 0, 1)]),                // ordering
            (("A4", 0, 1), &[("B5", 1, 0)]),                 // server
            (("B6", 1, 0), &[("C2", -1, 1)]),                // .
        ],
        "/tmp".into(),
        0,
    );
    if !Command::new("cargo")
        .args(["build", "-p", "c-ordering-client"])
        .spawn()
        .unwrap()
        .wait()
        .unwrap()
        .success()
    {
        panic!("failed to compile");
    }
    server_handle.updates_acks[0]
        .0
        .send(Some(precedence.clone()))
        .await
        .unwrap();
    let evars = server_handle.updates_acks[0].1.recv().await.unwrap();
    compile("blocking-client-a".into());
    compile("blocking-client-b".into());
    compile("blocking-client-c".into());
    let child_a = run_federate("blocking-client-a".into(), &evars);
    let child_b = run_federate("blocking-client-b".into(), &evars);
    let child_c = run_federate("blocking-client-c".into(), &evars);
    child_a.join().unwrap();
    child_b.join().unwrap();
    child_c.join().unwrap();
    server_handle.updates_acks[0]
        .0
        .send(Some(precedence))
        .await
        .unwrap();
    let evars = server_handle.updates_acks[0].1.recv().await.unwrap();
    compile("blocking-client-a".into());
    compile("blocking-client-b".into());
    compile("blocking-client-c".into());
    let child_a = run_federate("blocking-client-a".into(), &evars);
    let child_b = run_federate("blocking-client-b".into(), &evars);
    let child_c = run_federate("blocking-client-c".into(), &evars);
    child_a.join().unwrap();
    child_b.join().unwrap();
    child_c.join().unwrap();
    server_handle.updates_acks[0].0.send(None).await.unwrap();
    server_handle.join_handle.await.unwrap();
    println!("Server finished");
}
