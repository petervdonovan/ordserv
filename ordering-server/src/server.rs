use std::{collections::HashMap, env, ffi::c_int, os::fd::RawFd};

use log::{debug, error, info, warn};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::{
    channel_vec,
    connection::{ConnectionManagement, UNIX_CONNECTION_MANAGEMENT},
    tcpconnectionprovider::reusing,
    EnvironmentVariables, FederateId, Precedence, PrecedenceId, RunId,
};

pub(crate) const PRECEDENCE_FILE_NAME: &str = "ORDSERV_PRECEDENCE_FILE";
pub(crate) const PRECEDENCE_ID_NAME: &str = "ORDSERV_PRECEDENCE_ID";

pub(crate) fn connection_raw_fd_for(fedid: FederateId) -> RawFd {
    let fd = env::var_os(evar_name_for(fedid)).unwrap_or_else(|| {
        panic!(
            "Environment variable {} not set; did you forget to call run_server_reusing and use the resulting environment variables?",
            evar_name_for(fedid)
        )
    });
    fd.as_os_str().to_str().unwrap().parse::<c_int>().unwrap()
}

fn evar_name_for(fedid: FederateId) -> String {
    format!("ORDSERV_CONNECTION_{}", fedid.0)
}

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
// pub async fn run(port: u16, capacity: usize) -> ServerHandle {
//     let mut my_updates_acks = Vec::with_capacity(capacity);
//     let mut their_updates_acks = Vec::with_capacity(capacity);
//     for _ in 0..capacity {
//         let (update_sender, update_receiver) = mpsc::channel(1);
//         let (ack_sender, ack_receiver) = mpsc::channel(1);
//         their_updates_acks.push((update_sender, ack_receiver));
//         my_updates_acks.push((update_receiver, ack_sender));
//     }
//     ServerHandle {
//         updates_acks: their_updates_acks,
//         join_handle: run_server(port, my_updates_acks).await,
//     }
// }

/// This function spawns a process that assumes that each element of `updates_acks` is managed by a
/// single sequential process that repeatedly:
/// 1. Sends a precedence
/// 2. Waits for an ack
/// 3. Spawns the promised number of processes, which each send a frame, upon which connections to
///    them are forwarded here via the connection_receiver.
/// 4. Waits for all the processes to finish
///
/// This ends when None is received from the precedence stream.
pub async fn run_reusing_connections(
    capacity: usize,
    max_n_simultaneous_connections: usize,
) -> ServerHandle {
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
        join_handle: run_server_reusing_connections(
            my_updates_acks,
            max_n_simultaneous_connections,
        )
        .await,
    }
}

async fn process_precedence_stream<R, W>(
    mut precedence_stream: mpsc::Receiver<Option<Precedence>>,
    acks: mpsc::Sender<EnvironmentVariables>,
    mut connection_receiver: mpsc::Receiver<(RawFd, FederateId, RunId)>,
    precid: PrecedenceId,
    connection_requests: Option<mpsc::Sender<usize>>,
    connection_management: ConnectionManagement<R, W>,
) where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
    W: tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let mut outer_precedence = precedence_stream.recv().await.unwrap_or(None);
    let mut n_successful_connections = 0;
    let mut n_attempted_connections = 0;
    'outer: while let Some(precedence) = outer_precedence.take() {
        debug!("Received precedence");
        acks.send(environment_variables_for_clients(&precedence, precid).await)
            .await
            .unwrap();
        debug!("Expecting {} connections", precedence.n_connections);
        if let Some(connection_requests) = &connection_requests {
            connection_requests
                .send(precedence.n_connections)
                .await
                .unwrap();
        }
        n_attempted_connections += precedence.n_connections;
        let mut writers = HashMap::new();
        let mut readers = HashMap::new();
        let mut n_connected = 0;
        while n_connected < precedence.n_connections {
            tokio::select! {
                new_connection = connection_receiver.recv() => {
                    let (raw_connection, fedid, run_id) = new_connection.unwrap();
                    let connection = unsafe {(connection_management.borrow)(raw_connection)};
                    if run_id.0 != precedence.run_id {
                        error!("Received connection with run_id {} but precedence has run_id {}. This indicates a bug in the test framework, but I am not failing fast now due to lack of time.", run_id.0, precedence.run_id);
                    } else if let Ok(connection) = connection {
                        debug!("Received connection from {:?}", fedid);
                        let (reader, writer) = connection.into_split();
                        writers.insert(fedid, writer);
                        readers.insert(fedid, reader);
                        n_connected += 1;
                        n_successful_connections += 1;
                    } else {
                        error!("Failed to accept connection: {:?} even though it should have been vetted by the connection provider before it was sent here", raw_connection);
                    }
                }
                new_precedence = precedence_stream.recv() => {
                    warn!("Received new precedence while waiting for connections");
                    outer_precedence = new_precedence.unwrap_or(None);
                    continue 'outer;
                }
            }
        }
        debug!(
            "All connections received (success rate = {} / {} = {})",
            n_successful_connections,
            n_attempted_connections,
            n_successful_connections as f64 / n_attempted_connections as f64
        );
        let (send_frames, mut recv_frames) = mpsc::channel(1);
        let (halt_sender, mut halt_receiver) = tokio::sync::watch::channel(());
        let mut reader_handles = HashMap::new();
        for (fedid, mut reader) in readers.into_iter() {
            let mut halt_receiver = halt_sender.subscribe();
            let send_frames = send_frames.clone();
            reader_handles.insert(
                fedid,
                tokio::spawn(async move {
                    loop {
                        debug!("Waiting for frame from {:?}", fedid);
                        tokio::select! {
                            _ = halt_receiver.changed() => {
                                debug!("Reader received halt signal");
                                // halt_receiver.mark_changed();
                                break;
                            }
                            frame = reader.read_frame() => {
                                match frame {
                                    Some(frame) => {
                                        debug!("Received frame: {:?} from {:?}", frame, fedid);
                                        assert!(fedid.0 == frame.federate_id);
                                        assert!(precedence.run_id == frame.run_id);
                                        send_frames.send(frame).await.unwrap_or_else(|_| {
                                            warn!("Failed to send frame. This is not strictly an error condition because the two halt receivers (in the frame sender and receiver) are racing with each other, but it should be unusual because it should be uncommon for programs to finish while frames are in flight. Because of the timeout when waiting for in-flight frames, it can happen under 'normal' conditions, however.");
                                        });
                                    }
                                    None => {
                                        info!(target: "server", "Connection closed");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    reader
                }),
            );
        }
        let writer_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = halt_receiver.changed() => {
                        debug!("Writer received halt signal");
                        // halt_receiver.mark_changed();
                        break;
                    }
                    frame = recv_frames.recv() => {
                        match frame {
                            Some(frame) => {
                                for dest in precedence
                                    .sender2waiters
                                    .get(&frame.hook_invocation())
                                    .unwrap_or_else(|| {
                                        panic!("Received frame {:?} with hid {:?} for which there are no waiters; the actual precedence is:\n    {:?}", frame, frame.hid(), precedence);
                                    })
                                {
                                    debug!("Forwarding frame to {:?}", dest);
                                    let writers_debug = writers.keys().cloned().collect::<Vec<_>>(); // FIXME: this is just for debugging
                                    tokio::select!{
                                        _ = halt_receiver.changed() => {
                                            debug!("Writer received halt signal");
                                            // halt_receiver.mark_changed();
                                            break;
                                        }
                                        _ = writers
                                        .get_mut(&dest.hid.1)
                                        .unwrap_or_else(|| {
                                            panic!("Received frame {:?} with hid {:?} and dest id {:?} for which there are no writers for the dest; the actual precedence is:\n    {:?}, and the writers are:\n    {:?}", frame, frame.hid(), &dest.hid.1, precedence, writers_debug);
                                        })
                                        .write_frame(frame) => {
                                            debug!("Frame forwarded to {:?}", dest);
                                        }
                                    }
                                }
                            }
                            None => {
                                info!(target: "server", "Connection closed");
                                break;
                            }
                        }
                    }
                }
            }
            writers
        });
        debug!("Awaiting a new precedence");
        outer_precedence = precedence_stream.recv().await.unwrap_or(None);
        debug!("Received new precedence");
        halt_sender.send(()).unwrap();
        let mut writer_handle = writer_handle.await.unwrap();
        for fedid in reader_handles.keys().cloned().collect::<Vec<_>>() {
            let join_result = reader_handles.remove_entry(&fedid).unwrap().1.await;
            if let Err(e) = join_result {
                error!("Failed to join reader thread for {:?}. This is very bad because it means that we cannot recover the socket handle, but it is non-fatal because later when we find out that the socket handle is f**ed, we'll make a new one. Error:\n    {:?}", fedid, e);
            } else {
                unsafe {
                    (connection_management.unborrow)((
                        join_result.unwrap(),
                        writer_handle.remove_entry(&fedid).unwrap().1,
                    ))
                }
            }
        }
        debug!("unborrows done");
    }
    debug!("Received None from precedence stream");
}

async fn environment_variables_for_clients(
    precedence: &Precedence,
    id: PrecedenceId,
) -> EnvironmentVariables {
    let f = precedence.scratch_dir.join("precedences.ord");
    tokio::fs::write(&f, rmp_serde::to_vec(&precedence).unwrap())
        .await
        .unwrap();
    EnvironmentVariables(vec![
        (PRECEDENCE_FILE_NAME.into(), f.as_os_str().into()),
        (PRECEDENCE_ID_NAME.into(), id.0.to_string().into()),
    ])
}

// async fn run_server(
//     port: u16,
//     updates_acks: Vec<(
//         mpsc::Receiver<Option<Precedence>>,
//         mpsc::Sender<EnvironmentVariables>,
//     )>,
// ) -> JoinHandle<()> {
//     let mut handles = Vec::with_capacity(updates_acks.len() + 1);
//     let connection_receivers = forwarding(port, updates_acks.len()).await;
//     for (precid, ((update_receiver, ack_sender), connection_receiver)) in updates_acks
//         .into_iter()
//         .zip(connection_receivers.into_iter())
//         .enumerate()
//     {
//         handles.push(tokio::spawn(process_precedence_stream(
//             update_receiver,
//             ack_sender,
//             connection_receiver,
//             PrecedenceId(precid as u32),
//             None,
//             ConnectionManagement {
//                 borrow: (),
//                 unborrow: (),
//             },
//         )));
//     }
//     tokio::spawn(async move {
//         for handle in handles {
//             handle.await.unwrap();
//         }
//     })
// }

async fn run_server_reusing_connections(
    updates_acks: Vec<(
        mpsc::Receiver<Option<Precedence>>,
        mpsc::Sender<EnvironmentVariables>,
    )>,
    max_n_simultaneous_connections: usize,
) -> JoinHandle<()> {
    let mut handles = Vec::with_capacity(updates_acks.len() * 2 + 1);
    let (connection_requests_senders, connection_requests_receivers) =
        channel_vec(updates_acks.len());
    let (granted_connections_senders, granted_connections_receivers) =
        channel_vec(updates_acks.len());
    let (connection_receivers, abort_handles) = reusing(
        updates_acks.len(),
        max_n_simultaneous_connections,
        connection_requests_receivers,
        granted_connections_senders,
    );
    for (
        precid,
        (
            (((update_receiver, ack_sender), connection_receiver), connection_requests_sender),
            mut granted_connections_receiver,
        ),
    ) in updates_acks
        .into_iter()
        .zip(connection_receivers.into_iter())
        .zip(connection_requests_senders.into_iter())
        .zip(granted_connections_receivers.into_iter())
        .enumerate()
    {
        let (evars_sender, mut evars_receiver) = mpsc::channel::<EnvironmentVariables>(1);
        handles.push(tokio::spawn(process_precedence_stream(
            update_receiver,
            evars_sender,
            connection_receiver,
            PrecedenceId(precid as u32),
            Some(connection_requests_sender),
            UNIX_CONNECTION_MANAGEMENT,
        )));
        handles.push(tokio::spawn(async move {
            let mut evars_option = evars_receiver.recv().await;
            while let Some(mut evars) = evars_option.take() {
                tokio::select! {
                    granted_connections = granted_connections_receiver.recv() => {
                        for (fednum, granted) in granted_connections
                            .unwrap()
                            .iter()
                            .enumerate()
                        {
                            evars.0.push((
                                evar_name_for(FederateId(fednum as i32 - 1)).into(), // Start counting at -1 cuz RTI is -1. Assumes contiguousness
                                granted.to_string().into(),
                            ));
                        }
                        ack_sender.send(evars).await.unwrap();
                        evars_option = evars_receiver.recv().await;
                    }
                    next_evars = evars_receiver.recv() => {
                        if next_evars.is_some() {
                            error!("received next evars when it was not expected. Continuing...");
                        }
                        evars_option = next_evars;
                    }
                }
            }
        }));
    }
    tokio::spawn(async move {
        let id = rand::random::<u32>();
        for (k, handle) in handles.into_iter().enumerate() {
            info!("Waiting for handle {} ({})", k, id);
            handle.await.unwrap();
            info!("Handle {} done ({})", k, id);
        }
        for (k, handle) in abort_handles.into_iter().enumerate() {
            info!("Waiting for abort handle {} ({})", k, id);
            handle.abort();
            info!("Abort handle {} done ({})", k, id);
        }
    })
}
