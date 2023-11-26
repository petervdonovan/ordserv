use std::collections::HashMap;

use log::{debug, info, warn};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::{
    connection::Connection, tcpconnectionprovider::forwarding, EnvironmentVariables, FederateId,
    Precedence, PrecedenceId,
};

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
    mut connection_receiver: mpsc::Receiver<(Connection, FederateId, u32)>,
    precid: PrecedenceId,
) {
    let mut jhs: Vec<JoinHandle<()>> = Vec::new();
    let mut outer_precedence = precedence_stream.recv().await.unwrap_or(None);
    let mut n_successful_connections = 0;
    let mut n_attempted_connections = 0;
    'outer: while let Some(precedence) = outer_precedence.take() {
        debug!("Received precedence");
        for jh in jhs.drain(..) {
            jh.abort();
        }
        acks.send(environment_variables_for_clients(&precedence, precid))
            .await
            .unwrap();
        info!("Expecting {} connections", precedence.n_connections);
        n_attempted_connections += precedence.n_connections;
        let mut writers = HashMap::new();
        let mut readers = HashMap::new();
        let mut n_connected = 0;
        while n_connected < precedence.n_connections {
            tokio::select! {
                new_connection = connection_receiver.recv() => {
                    let (connection, fedid, run_id) = new_connection.unwrap();
                    if run_id != precedence.run_id {
                        warn!("Received connection with run_id {} but precedence has run_id {}", run_id, precedence.run_id);
                        connection.close().await;
                        warn!("Forcibly closed connection; client should crash.");
                    } else {
                        debug!("Received connection from {:?}", fedid);
                        let (reader, writer) = connection.into_split();
                        writers.insert(fedid, writer);
                        readers.insert(fedid, reader);
                        n_connected += 1;
                        n_successful_connections += 1;
                    }
                }
                new_precedence = precedence_stream.recv() => {
                    warn!("Received new precedence while waiting for connections");
                    outer_precedence = new_precedence.unwrap_or(None);
                    continue 'outer;
                }
            }
        }
        info!(
            "All connections received (success rate = {} / {} = {})",
            n_successful_connections,
            n_attempted_connections,
            n_successful_connections as f64 / n_attempted_connections as f64
        );
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
                            assert!(precedence.run_id == frame.run_id);
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
                    .unwrap_or_else(|| {
                        panic!("Received frame {:?} with hid {:?} for which there are no waiters; the actual precedence is:\n    {:?}", frame, frame.hid(), precedence);
                    })
                {
                    debug!("Forwarding frame to {:?}", dest);
                    let writers_debug = writers.keys().cloned().collect::<Vec<_>>(); // FIXME: this is just for debugging
                    writers
                        .get_mut(&dest.hid.1)
                        .unwrap_or_else(|| {
                            panic!("Received frame {:?} with hid {:?} and dest id {:?} for which there are no writers for the dest; the actual precedence is:\n    {:?}, and the writers are:\n    {:?}", frame, frame.hid(), &dest.hid.1, precedence, writers_debug);
                        })
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

async fn run_server(
    port: u16,
    updates_acks: Vec<(
        mpsc::Receiver<Option<Precedence>>,
        mpsc::Sender<EnvironmentVariables>,
    )>,
) -> JoinHandle<()> {
    let mut handles = Vec::with_capacity(updates_acks.len() + 1);
    let connection_receivers = forwarding(port, updates_acks.len()).await;
    for (precid, ((update_receiver, ack_sender), connection_receiver)) in updates_acks
        .into_iter()
        .zip(connection_receivers.into_iter())
        .enumerate()
    {
        handles.push(tokio::spawn(process_precedence_stream(
            update_receiver,
            ack_sender,
            connection_receiver,
            PrecedenceId(precid as u32),
        )));
    }
    tokio::spawn(async move {
        for handle in handles {
            handle.await.unwrap();
        }
    })
}
