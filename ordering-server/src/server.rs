use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::{connection::Connection, Precedence};

pub struct ServerHandle {
    pub updates_acks: Vec<(mpsc::Sender<Option<Precedence>>, mpsc::Receiver<()>)>,
    pub join_handle: JoinHandle<()>,
}

pub fn run(port: u16, capacity: usize) -> ServerHandle {
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
        join_handle: run_server(port, my_updates_acks),
    }
}

/// Contract: Conceptually this function interacts with one sequential process which repeatedly does
/// the following:
/// 1. Sends a precedence
/// 2. Waits for an ack
/// 3. Spawns the promised number of processes, which each send a frame, upon which connections to
///    them are forwarded here via the connection_receiver.
/// 4. Waits for all the processes to finish
///
/// This ends when None is received from the precedence stream.
async fn process_precedence_stream(
    mut precedence_stream: mpsc::Receiver<Option<Precedence>>,
    acks: mpsc::Sender<()>,
    mut connection_receiver: mpsc::Receiver<Connection>,
) {
    let mut jhs = Vec::new();
    while let Some(precedence) = precedence_stream.recv().await.unwrap() {
        println!("Received precedence");
        // Cancel all green threads from the last iteration of the loop as soon as a new precedence object is received
        jhs.clear();
        acks.send(()).await.unwrap();
        let mut writers = vec![];
        let mut readers = vec![];
        for _ in 0..precedence.n_connections {
            let (reader, writer) = connection_receiver.recv().await.unwrap().into_split();
            writers.push(writer);
            readers.push(reader);
        }
        let (send_frames, mut recv_frames) = mpsc::channel(1);
        for mut reader in readers {
            let send_frames = send_frames.clone();
            jhs.push(tokio::spawn(async move {
                loop {
                    let frame = reader.read_frame().await;
                    match frame {
                        Some(frame) => {
                            println!("Received frame: {:?}", frame);
                            send_frames.send(frame).await.unwrap();
                        }
                        None => {
                            println!("Connection closed");
                            break;
                        }
                    }
                }
            }));
        }
        jhs.push(tokio::spawn(async move {
            while let Some(frame) = recv_frames.recv().await {
                for writer in &mut writers {
                    writer.write_frame(frame).await;
                }
            }
        }));
    }
    println!("Received None from precedence stream");
    drop(jhs);
}

async fn forward_tcp_connections(connection_senders: Vec<mpsc::Sender<Connection>>, port: u16) {
    loop {
        let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
        let mut connection = Connection::new(listener.accept().await.unwrap().0);
        println!("Accepted connection");
        let frame = connection.read_frame().await;
        match frame {
            Some(frame) => {
                println!("Received frame: {:?}", frame);
                connection_senders[frame.precedence_id as usize]
                    .send(connection)
                    .await
                    .unwrap();
            }
            None => {
                eprintln!("A client disconnected without sending a frame");
            }
        }
    }
}

fn run_server(
    port: u16,
    updates_acks: Vec<(mpsc::Receiver<Option<Precedence>>, mpsc::Sender<()>)>,
) -> JoinHandle<()> {
    let mut connection_senders: Vec<mpsc::Sender<Connection>> = Vec::new();
    let mut handles = Vec::with_capacity(updates_acks.len() + 1);
    for (update_receiver, ack_sender) in updates_acks {
        let (connection_sender, connection_receiver) = mpsc::channel(1);
        connection_senders.push(connection_sender);
        handles.push(tokio::spawn(process_precedence_stream(
            update_receiver,
            ack_sender,
            connection_receiver,
        )));
    }
    let _drop_me = tokio::spawn(forward_tcp_connections(connection_senders, port));
    tokio::spawn(async move {
        for handle in handles {
            handle.await.unwrap();
        }
    })
}
