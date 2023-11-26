use log::{debug, info};
use tokio::{net::TcpListener, sync::mpsc};

use crate::{connection::Connection, FederateId};

pub async fn forwarding(
    port: u16,
    n_connections: usize,
) -> Vec<mpsc::Receiver<(Connection, FederateId, u32)>> {
    let mut senders = Vec::with_capacity(n_connections);
    let mut receivers = Vec::with_capacity(n_connections);
    for _ in 0..n_connections {
        let (sender, receiver) = mpsc::channel(1);
        senders.push(sender);
        receivers.push(receiver);
    }
    let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
    let _drop_me = tokio::spawn(forward_tcp_connections(listener, senders, port));
    receivers
}

async fn forward_tcp_connections(
    listener: TcpListener,
    connection_senders: Vec<mpsc::Sender<(Connection, FederateId, u32)>>,
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
                if connection_senders[frame.precedence_id as usize]
                    .send((connection, FederateId(frame.federate_id), frame.run_id))
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
