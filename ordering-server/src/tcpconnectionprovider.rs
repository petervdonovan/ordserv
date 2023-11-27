use std::os::fd::{AsRawFd, RawFd};

use std::os::unix::io::FromRawFd;

use log::{debug, info};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};

use crate::channel_vec;
use crate::{connection::Connection, FederateId, RunId};

pub type ConnectionElt = (Connection, FederateId, RunId);

pub async fn forwarding(
    port: u16,
    n_connection_streams: usize,
) -> Vec<mpsc::Receiver<ConnectionElt>> {
    let (senders, receivers) = channel_vec(n_connection_streams);
    let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
    let _drop_me = tokio::spawn(forward_tcp_connections(listener, senders, port));
    receivers
}

async fn forward_tcp_connections(
    listener: TcpListener,
    connection_senders: Vec<mpsc::Sender<ConnectionElt>>,
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

pub async fn reusing(
    port: u16,
    n_connection_streams: usize,
    max_n_simultaneous_connections: usize,
    connection_requests: Vec<mpsc::Receiver<usize>>,
    granted_connections: Vec<mpsc::Sender<Vec<RawFd>>>,
) -> Vec<mpsc::Receiver<ConnectionElt>> {
    let (senders, receivers) = channel_vec(n_connection_streams);
    let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
    let mut connection_table: Vec<Vec<(RawFd, RawFd)>> = Vec::with_capacity(n_connection_streams);
    for _ in 0..n_connection_streams {
        let mut connections = Vec::with_capacity(max_n_simultaneous_connections);
        for _ in 0..max_n_simultaneous_connections {
            let jh = tokio::task::spawn(async move {
                TcpStream::connect(("127.0.0.1", port))
                    .await
                    .unwrap()
                    .as_raw_fd()
            });
            let server_connection = listener.accept().await.unwrap().0.as_raw_fd();
            let client_connection = jh.await.unwrap();
            connections.push((server_connection, client_connection));
        }
        connection_table.push(connections);
    }
    for (((connection_requestor, connection_list), connection_sender), granted_connection_sender) in
        connection_requests
            .into_iter()
            .zip(connection_table.into_iter())
            .zip(senders.into_iter())
            .zip(granted_connections.into_iter())
    {
        let _drop_me = tokio::spawn(reuse_tcp_connections(
            connection_list,
            connection_sender,
            connection_requestor,
            granted_connection_sender,
        ));
    }
    receivers
}

pub(crate) unsafe fn socket_from_raw_fd(fd: RawFd) -> TcpStream {
    TcpStream::from_std(std::net::TcpStream::from_raw_fd(fd)).unwrap()
}

async fn reuse_tcp_connections(
    connection_list: Vec<(RawFd, RawFd)>,
    connection_sender: mpsc::Sender<ConnectionElt>,
    mut n_connections: mpsc::Receiver<usize>,
    granted_connection_sender: mpsc::Sender<Vec<RawFd>>,
) {
    while let Some(n_connections) = n_connections.recv().await {
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
        for (server_connection, _client_connection) in connection_list.iter().take(n_connections) {
            let mut server_connection = Connection::new(unsafe {
                TcpStream::from_std(std::net::TcpStream::from_raw_fd(*server_connection)).unwrap()
            });
            let frame = server_connection.read_frame().await;
            match frame {
                Some(frame) => {
                    debug!("Received initial frame: {:?}", frame);
                    if frame.hook_id[0] != b'S' {
                        // eprintln!(
                        //     "Received frame with hook_id {:?} instead of 'S'",
                        //     frame.hook_id
                        // );
                        // server_connection.close().await;
                        // eprintln!("Forcibly closed connection; client should crash.");
                        // continue;
                        panic!(
                            "Expected initial frame to have hook_id 'S', but got {:?}",
                            frame.hook_id
                        );
                    }
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
                        break;
                    }
                }
                None => {
                    eprintln!("A client disconnected without sending a frame");
                }
            }
        }
    }
}
