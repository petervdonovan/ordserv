use std::collections::HashMap;

use log::{debug, info};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::{connection::Connection, EnvironmentVariables, FederateId, Precedence, PrecedenceId};

pub(crate) const PRECEDENCE_FILE_NAME: &str = "ORDSERV_PRECEDENCE_FILE";
pub(crate) const PRECEDENCE_ID_NAME: &str = "ORDSERV_PRECEDENCE_ID";

pub struct ServerHandle {
    pub updates_acks: Vec<ServerSubHandle>,
    pub join_handle: JoinHandle<()>,
}
pub type ServerSubHandle = (
    mpsc::Sender<Option<Precedence>>,
    mpsc::Receiver<EnvironmentVariables>,
);

/// This function spawns a process that assumes that each element of `updates_acks` is managed by a
/// single sequential process that repeatedly:
/// 1. Sends a precedence
/// 2. Waits for an ack
/// 3. Spawns the promised number of processes, which each send a frame, upon which connections to
///    them are forwarded here via the connection_receiver.
/// 4. Waits for all the processes to finish
///
/// This ends when None is received from the precedence stream.
pub async fn run(port: u16, capacity: usize) -> ServerHandle {
    let mut my_updates_acks = Vec::with_capacity(capacity);
    let mut their_updates_acks = Vec::with_capacity(capacity);
    for _ in 0..capacity {
        let (update_sender, update_receiver) = mpsc::channel(1);
        let (ack_sender, ack_receiver) = mpsc::channel(1);
        their_updates_acks.push((update_sender, ack_receiver));
        my_updates_acks.push((update_receiver, ack_sender));
    }
    ServerHandle {
        updates_acks: their_updates_acks,
        join_handle: run_server(port, my_updates_acks).await,
    }
}

async fn process_precedence_stream(
    mut precedence_stream: mpsc::Receiver<Option<Precedence>>,
    acks: mpsc::Sender<EnvironmentVariables>,
    mut connection_receiver: mpsc::Receiver<(Connection, FederateId)>,
    precid: PrecedenceId,
) {
    let mut jhs: Vec<JoinHandle<()>> = Vec::new();
    let mut outer_precedence = precedence_stream.recv().await.unwrap_or(None);
    'outer: while let Some(precedence) = outer_precedence.take() {
        debug!("Received precedence");
        for jh in jhs.drain(..) {
            jh.abort();
        }
        acks.send(environment_variables_for_clients(&precedence, precid))
            .await
            .unwrap();
        info!("Expecting {} connections", precedence.n_connections);
        let mut writers = HashMap::new();
        let mut readers = HashMap::new();
        for _ in 0..precedence.n_connections {
            tokio::select! {
                new_connection = connection_receiver.recv() => {
                    let (connection, fedid) = new_connection.unwrap();
                    debug!("Received connection from {:?}", fedid);
                    let (reader, writer) = connection.into_split();
                    writers.insert(fedid, writer);
                    readers.insert(fedid, reader);
                }
                new_precedence = precedence_stream.recv() => {
                    outer_precedence = new_precedence.unwrap_or(None);
                    continue 'outer;
                }
            }
        }
        info!("All connections received");
        let (send_frames, mut recv_frames) = mpsc::channel(1);
        for (fedid, mut reader) in readers.into_iter() {
            let send_frames = send_frames.clone();
            jhs.push(tokio::spawn(async move {
                loop {
                    let frame = reader.read_frame().await;
                    match frame {
                        Some(frame) => {
                            debug!("Received frame: {:?} from {:?}", frame, fedid);
                            assert!(fedid.0 == frame.federate_id);
                            send_frames.send(frame).await.unwrap();
                        }
                        None => {
                            info!(target: "server", "Connection closed");
                            break;
                        }
                    }
                }
            }));
        }
        jhs.push(tokio::spawn(async move {
            while let Some(frame) = recv_frames.recv().await {
                for dest in precedence
                    .sender2waiters
                    .get(&frame.hook_invocation())
                    .unwrap()
                {
                    debug!("Forwarding frame to {:?}", dest);
                    writers
                        .get_mut(&dest.hid.1)
                        .unwrap()
                        .write_frame(frame)
                        .await;
                }
            }
        }));
        debug!("Awaiting a new precedence");
        outer_precedence = precedence_stream.recv().await.unwrap_or(None);
    }
    debug!("Received None from precedence stream");
}

fn environment_variables_for_clients(
    precedence: &Precedence,
    id: PrecedenceId,
) -> EnvironmentVariables {
    let f = precedence.scratch_dir.join("precedences.ord");
    std::fs::write(&f, rmp_serde::to_vec(&precedence).unwrap()).unwrap();
    EnvironmentVariables(vec![
        (PRECEDENCE_FILE_NAME.into(), f.as_os_str().into()),
        (PRECEDENCE_ID_NAME.into(), id.0.to_string().into()),
    ])
}

async fn forward_tcp_connections(
    listener: TcpListener,
    connection_senders: Vec<mpsc::Sender<(Connection, FederateId)>>,
    port: u16,
) {
    loop {
        debug!("Listening for connections on port {}", port);
        let mut connection = Connection::new(listener.accept().await.unwrap().0);
        info!("Accepted connection");
        let frame = connection.read_frame().await;
        match frame {
            Some(frame) => {
                debug!("Received initial frame: {:?}", frame);
                connection_senders[frame.precedence_id as usize]
                    .send((connection, FederateId(frame.federate_id)))
                    .await
                    .unwrap();
            }
            None => {
                eprintln!("A client disconnected without sending a frame");
            }
        }
    }
}

async fn run_server(
    port: u16,
    updates_acks: Vec<(
        mpsc::Receiver<Option<Precedence>>,
        mpsc::Sender<EnvironmentVariables>,
    )>,
) -> JoinHandle<()> {
    let mut connection_senders: Vec<mpsc::Sender<(Connection, FederateId)>> = Vec::new();
    let mut handles = Vec::with_capacity(updates_acks.len() + 1);
    for (precid, (update_receiver, ack_sender)) in updates_acks.into_iter().enumerate() {
        let (connection_sender, connection_receiver) = mpsc::channel(1);
        connection_senders.push(connection_sender);
        handles.push(tokio::spawn(process_precedence_stream(
            update_receiver,
            ack_sender,
            connection_receiver,
            PrecedenceId(precid as u32),
        )));
    }
    let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
    let _drop_me = tokio::spawn(forward_tcp_connections(listener, connection_senders, port));
    tokio::spawn(async move {
        for handle in handles {
            handle.await.unwrap();
        }
    })
}
