use std::os::fd::{IntoRawFd, RawFd};

use log::{debug, info, warn};
use tokio::{net::TcpListener, sync::mpsc};

use crate::channel_vec;
use crate::connection::UNIX_CONNECTION_MANAGEMENT;
use crate::{connection::Connection, FederateId, RunId};

pub type ConnectionElt<R, W> = (Connection<R, W>, FederateId, RunId);
pub type TcpConnectionElt =
    ConnectionElt<tokio::net::tcp::OwnedReadHalf, tokio::net::tcp::OwnedWriteHalf>;
pub type UnixConnectionElt = (RawFd, FederateId, RunId);

pub async fn forwarding(
    port: u16,
    n_connection_streams: usize,
) -> Vec<mpsc::Receiver<TcpConnectionElt>> {
    let (senders, receivers) = channel_vec(n_connection_streams);
    let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
    let _drop_me = tokio::spawn(forward_tcp_connections(listener, senders, port));
    receivers
}

async fn forward_tcp_connections(
    listener: TcpListener,
    connection_senders: Vec<mpsc::Sender<TcpConnectionElt>>,
    port: u16,
) {
    loop {
        debug!("Listening for connections on port {}", port);
        let mut connection = Connection::new(listener.accept().await.unwrap().0.into_split());
        info!("Accepted connection");
        let frame = connection.read_frame().await;
        match frame {
            Some(frame) => {
                debug!("Received initial frame: {:?}", frame);
                if frame.hook_id[0] != b'S' {
                    eprintln!(
                        "Received frame with hook_id {:?} instead of 'S'",
                        frame.hook_id
                    );
                    connection.close().await;
                    eprintln!("Forcibly closed connection; client should crash.");
                    continue;
                }
                debug!("Sending connection to client-specific thread");
                if connection_senders[frame.precedence_id as usize]
                    .send((
                        connection,
                        FederateId(frame.federate_id),
                        RunId(frame.run_id),
                    ))
                    .await
                    .is_err()
                {
                    debug!("Connection receiver dropped; closing channel.");
                    break;
                }
            }
            None => {
                eprintln!("A client disconnected without sending a frame");
            }
        }
    }
}

static CREATING_CONNECTIONS_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn reusing(
    n_connection_streams: usize,
    max_n_simultaneous_connections: usize,
    connection_requests: Vec<mpsc::Receiver<usize>>,
    granted_connections: Vec<mpsc::Sender<Vec<RawFd>>>,
) -> (
    Vec<mpsc::Receiver<UnixConnectionElt>>,
    Vec<tokio::task::JoinHandle<()>>,
) {
    let _lock = CREATING_CONNECTIONS_MUTEX.lock().unwrap();
    let (senders, receivers) = channel_vec(n_connection_streams);
    let mut connection_table: Vec<Vec<(RawFd, RawFd)>> = Vec::with_capacity(n_connection_streams);
    for _ in 0..n_connection_streams {
        let mut connections = Vec::with_capacity(max_n_simultaneous_connections);
        for _ in 0..max_n_simultaneous_connections {
            let (server_connection, client_connection) = make_server_and_client_connection_pair();
            connections.push((server_connection, client_connection));
        }
        connection_table.push(connections);
    }
    let mut abort_handles = Vec::with_capacity(n_connection_streams);
    for (((connection_requestor, connection_list), connection_sender), granted_connection_sender) in
        connection_requests
            .into_iter()
            .zip(connection_table.into_iter())
            .zip(senders.into_iter())
            .zip(granted_connections.into_iter())
    {
        abort_handles.push(tokio::spawn(reuse_tcp_connections(
            connection_list,
            connection_sender,
            connection_requestor,
            granted_connection_sender,
        )));
    }
    (receivers, abort_handles)
}

static FCNTL_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn make_server_and_client_connection_pair() -> (RawFd, RawFd) {
    let _lock = FCNTL_MUTEX.lock().unwrap();
    let (server_connection, client_connection) = std::os::unix::net::UnixStream::pair().unwrap();
    let client_connection = client_connection.into_raw_fd();
    let server_connection = server_connection.into_raw_fd();
    unsafe {
        // magic snippet I don't understand dug up from the depths of the web
        // https://stackoverflow.com/questions/55540577/how-to-communicate-a-rust-and-a-ruby-process-using-a-unix-socket-pair
        // where the link that was supposed to explain it is dead
        let flags = libc::fcntl(client_connection, libc::F_GETFD);
        libc::fcntl(client_connection, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
        let flags = libc::fcntl(server_connection, libc::F_GETFD);
        libc::fcntl(server_connection, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
    }
    (server_connection, client_connection)
}

async fn reuse_tcp_connections(
    mut connection_list: Vec<(RawFd, RawFd)>,
    connection_sender: mpsc::Sender<UnixConnectionElt>,
    mut n_connections_receiver: mpsc::Receiver<usize>,
    granted_connection_sender: mpsc::Sender<Vec<RawFd>>,
) {
    let mut n_connections_option = n_connections_receiver.recv().await;
    'outer_outer: while let Some(n_connections) = n_connections_option {
        let mut server_connections_borrowed = Vec::with_capacity(n_connections);
        for (server_connection, client_connection) in connection_list.iter_mut().take(n_connections)
        {
            let mut server_connection_borrowed =
                unsafe { (UNIX_CONNECTION_MANAGEMENT.borrow)(*server_connection) };
            while let Err(e) = server_connection_borrowed {
                warn!("Failed to borrow connection: {:?}", e);
                (*server_connection, *client_connection) = make_server_and_client_connection_pair();
                server_connection_borrowed =
                    unsafe { (UNIX_CONNECTION_MANAGEMENT.borrow)(*server_connection) };
            }
            server_connections_borrowed
                .push((*server_connection, server_connection_borrowed.unwrap()));
        }
        granted_connection_sender
            .send(
                connection_list
                    .iter()
                    .take(n_connections)
                    .map(|(_, client_connection)| client_connection)
                    .cloned()
                    .collect(),
            )
            .await
            .unwrap();
        'outer: for (server_connection, mut server_connection_borrowed) in
            server_connections_borrowed
        {
            // Connection::new(unsafe { socket_from_raw_fd(*server_connection) });
            loop {
                tokio::select! {
                    frame = server_connection_borrowed.read_frame() => {
                        match frame {
                            Some(frame) => {
                                debug!("Received initial frame: {:?}", frame);
                                if frame.hook_id[0] != b'S' {
                                    // eprintln!(
                                    //     "Received frame with hook_id {:?} instead of 'S'",
                                    //     frame.hook_id
                                    // );
                                    // server_connection_borrowed.close().await;
                                    // eprintln!("Forcibly closed connection; client should crash.");
                                    // continue;
                                    warn!(
                                    "Expected initial frame to have hook_id 'S', but got {:?}. This is not strictly an error condition because it is possible for frames from prior runs to be received by the server.",
                                    frame.hook_id
                                );
                                    continue;
                                }
                                unsafe {
                                    (UNIX_CONNECTION_MANAGEMENT.unborrow)(
                                        server_connection_borrowed.into_split(),
                                    );
                                }
                                // let second_frame = server_connection.read_frame().await; // DEBUG
                                // debug!("Second frame: {:?}", second_frame); // DEBUG
                                if connection_sender
                                    .send((
                                        server_connection,
                                        FederateId(frame.federate_id),
                                        RunId(frame.run_id),
                                    ))
                                    .await
                                    .is_err()
                                {
                                    debug!("Connection sender dropped; closing channel.");
                                    break 'outer;
                                }
                                break;
                            }
                            None => {
                                eprintln!("A client disconnected without sending a frame");
                                break;
                            }
                        }
                    }
                    n_connections_option_next = n_connections_receiver.recv() => {
                        n_connections_option = n_connections_option_next;
                        continue 'outer_outer;
                    }
                }
            }
        }
        n_connections_option = n_connections_receiver.recv().await;
    }
}
