use tokio::{
    net::{TcpStream, ToSocketAddrs},
    sync::mpsc,
    task::JoinHandle,
};

use crate::{
    connection::{Connection, WriteConnection},
    Frame,
};

pub struct Client {
    connection: WriteConnection,
}
pub struct ChannelClient {
    client: Client,
    pub frames: mpsc::Receiver<Frame>,
}

impl Client {
    pub async fn start<T: ToSocketAddrs>(
        addr: T,
        mut callback: Box<dyn FnMut(Frame) + Send>,
    ) -> (Client, JoinHandle<()>) {
        let socket = TcpStream::connect(addr).await.unwrap();
        let (mut read, write) = Connection::new(socket).into_split();
        (
            Client { connection: write },
            tokio::spawn(async move {
                loop {
                    let frame = read.read_frame().await;
                    match frame {
                        Some(frame) => {
                            println!("Received frame: {:?}", frame);
                            callback(frame);
                        }
                        None => {
                            println!("Connection closed");
                            break;
                        }
                    }
                }
            }),
        )
    }
    pub async fn write(&mut self, frame: Frame) {
        self.connection.write_frame(frame).await;
    }
}

impl ChannelClient {
    pub async fn start<T: ToSocketAddrs>(addr: T) -> (ChannelClient, JoinHandle<()>) {
        let (frames_sender, frames_receiver) = mpsc::channel(1);
        let (client, join_handle) = Client::start(
            addr,
            Box::new(move |frame| {
                println!("Forwarding frame to parent: {:?}", frame);
                frames_sender.clone().try_send(frame).unwrap();
            }),
        )
        .await;
        (
            ChannelClient {
                client,
                frames: frames_receiver,
            },
            join_handle,
        )
    }
    pub async fn write(&mut self, frame: Frame) {
        self.client.write(frame).await;
    }
}
