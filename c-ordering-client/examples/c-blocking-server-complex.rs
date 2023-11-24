use std::process::{Child, Command};

use ordering_server::{server, EnvironmentVariables, Precedence, ORDSERV_PORT_ENV_VAR};

// use simple_logger::SimpleLogger;

const PORT: u16 = 8080;

fn compile_and_run(name: String, runtime_evars: &EnvironmentVariables) -> Child {
    println!("DEBUG: cwd: {:?}", std::env::current_dir().unwrap());
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
    println!("DEBUG: running with envs: {:?}", runtime_evars.0);
    Command::new(format!("./c-ordering-client/examples/{}.run", name))
        .envs(runtime_evars.0.clone())
        .env(ORDSERV_PORT_ENV_VAR, PORT.to_string())
        .env(
            "C_ORDERING_CLIENT_LIBRARY_PATH",
            "./target/debug/libc_ordering_client.so",
        )
        .spawn()
        .expect("failed to execute process")
}

#[tokio::main]
async fn main() {
    // SimpleLogger::new().init().unwrap();
    let mut server_handle = server::run(PORT, 1);
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
    );
    server_handle.updates_acks[0]
        .0
        .send(Some(precedence))
        .await
        .unwrap();
    let evars = server_handle.updates_acks[0].1.recv().await.unwrap();
    let mut child_a = compile_and_run("blocking-client-a".into(), &evars);
    let mut child_b = compile_and_run("blocking-client-b".into(), &evars);
    let mut child_c = compile_and_run("blocking-client-c".into(), &evars);
    child_a.wait().unwrap();
    child_b.wait().unwrap();
    child_c.wait().unwrap();
    server_handle.updates_acks[0].0.send(None).await.unwrap();
    server_handle.join_handle.await.unwrap();
    println!("Server finished");
}
