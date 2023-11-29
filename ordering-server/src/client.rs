use std::{
    collections::HashSet,
    env,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

use log::{debug, info};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{self, OwnedWriteHalf},
        unix, TcpStream, ToSocketAddrs, UnixStream,
    },
    select,
    sync::{mpsc, watch},
    task::JoinHandle,
};

use crate::{
    connection::{Connection, ConnectionManagement, WriteConnection, UNIX_CONNECTION_MANAGEMENT},
    server::{PRECEDENCE_FILE_NAME, PRECEDENCE_ID_NAME},
    FederateId, Frame, HookInvocation, Precedence, PrecedenceId,
};

pub struct Client<W>
where
    W: AsyncWriteExt + Unpin,
{
    pub connection: WriteConnection<W>, // FIXME: should be private
}
pub struct ChannelClient<W>
where
    W: AsyncWriteExt + Unpin,
{
    client: Client<W>,
    pub frames: mpsc::Receiver<Frame>,
}

pub struct BlockingClient {
    requires_ok_to_proceed: HashSet<HookInvocation>,
    requires_notify: HashSet<HookInvocation>,
    ok_to_proceed: Arc<Mutex<HashSet<HookInvocation>>>,
    ok_cvar: Arc<Condvar>,
    notification_sender: tokio::sync::mpsc::UnboundedSender<Frame>,
    precid: PrecedenceId,
    fedid: FederateId,
    run_id: u32,
    wait_timeout: Duration,
    pub halt: watch::Sender<()>, // FIXME: should be private
}

impl Client<OwnedWriteHalf> {
    pub async fn start<T: ToSocketAddrs + std::fmt::Debug>(
        addr: T,
        callback: Box<dyn FnMut(Frame) + Send>,
        halt: watch::Receiver<()>,
    ) -> (
        Client<OwnedWriteHalf>,
        JoinHandle<tokio::net::tcp::OwnedReadHalf>,
    ) {
        Self::start_from_socket(socket_from_addr(addr).await.into_split(), callback, halt).await
    }
}

impl<W> Client<W>
where
    W: AsyncWriteExt + Unpin,
{
    async fn start_from_socket<R: AsyncReadExt + Unpin + Send + 'static>(
        socket: (R, W),
        mut callback: Box<dyn FnMut(Frame) + Send>,
        mut halt: watch::Receiver<()>,
    ) -> (Client<W>, JoinHandle<R>) {
        let (mut read, write) = Connection::new(socket).into_split();
        (
            Client { connection: write },
            tokio::spawn(async move {
                loop {
                    select! {
                        frame = read.read_frame() => {
                            match frame {
                                Some(frame) => {
                                    debug!("Invoking callback on frame: {:?}", frame);
                                    callback(frame);
                                }
                                None => {
                                    info!(target: "client", "Connection closed");
                                    panic!("The client is supposed to be the one to close the connection, not the server. The server can close the connection if this client is a straggler from a previous run.");
                                }
                            }
                        }
                        _ = halt.changed() => {
                            info!(target: "client", "Halt received in start_from_socket");
                            break;
                        }
                    }
                }
                info!(target: "client", "Exiting start_from_socket");
                read.stream
            }),
        )
    }
    pub async fn write(&mut self, frame: Frame) {
        self.connection.write_frame(frame).await;
    }
}

// impl ChannelClient<tcp::OwnedWriteHalf> {
//     pub async fn start<T: ToSocketAddrs + std::fmt::Debug>(
//         addr: T,
//     ) -> (ChannelClient<tcp::OwnedWriteHalf>, JoinHandle<()>) {
//         ChannelClient::start_from_socket(socket_from_addr(addr).await.into_split()).await
//     }
// }

// impl<W> ChannelClient<W>
// where
//     W: AsyncWriteExt + Unpin,
// {
//     pub async fn start_from_socket<R: AsyncReadExt + Unpin + Send + 'static>(
//         socket: (R, W),
//     ) -> (ChannelClient<W>, JoinHandle<()>) {
//         let (frames_sender, frames_receiver) = mpsc::channel(1);
//         let (client, join_handle) = Client::start_from_socket(
//             socket,
//             Box::new(move |frame| {
//                 debug!("Forwarding frame to parent: {:?}", frame);
//                 frames_sender.clone().try_send(frame).unwrap();
//             }),
//         )
//         .await;
//         (
//             ChannelClient {
//                 client,
//                 frames: frames_receiver,
//             },
//             join_handle,
//         )
//     }
//     pub async fn write(&mut self, frame: Frame) {
//         self.client.write(frame).await;
//     }
// }

// impl BlockingClient {
//     pub fn start<T: ToSocketAddrs + std::fmt::Debug>(
//         addr: T,
//         federate_id: FederateId,
//         wait_timeout: Duration,
//     ) -> (
//         BlockingClient,
//         std::thread::JoinHandle<Client<tokio::io::BufWriter<tokio::net::unix::OwnedWriteHalf>>>,
//     ) {
//         let rt = tokio::runtime::Builder::new_multi_thread()
//             .enable_io()
//             .build()
//             .unwrap();
//         let socket = rt.block_on(socket_from_addr(addr));
//         Self::start_from_socket(rt, federate_id, wait_timeout, socket.into_split())
//     }
// }

pub type BlockingClientJoinHandle =
    std::thread::JoinHandle<(Client<unix::OwnedWriteHalf>, unix::OwnedReadHalf)>;

impl BlockingClient {
    pub fn start_reusing_connection(
        federate_id: FederateId,
        wait_timeout: Duration,
    ) -> (BlockingClient, BlockingClientJoinHandle) {
        let raw_fd = crate::server::connection_raw_fd_for(federate_id);
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .build()
            .unwrap();
        let socket = rt.block_on(async { unsafe { (UNIX_CONNECTION_MANAGEMENT.borrow)(raw_fd) } });
        let (r, w) = socket.into_split();
        Self::start_from_socket::<unix::OwnedReadHalf, unix::OwnedWriteHalf>(
            rt,
            federate_id,
            wait_timeout,
            (r.stream, w.stream.into_inner()),
        )
    }
}

impl BlockingClient {
    fn start_from_socket<
        R: AsyncReadExt + Unpin + Send + 'static,
        W: AsyncWriteExt + Unpin + Send + 'static,
    >(
        rt: tokio::runtime::Runtime,
        federate_id: FederateId,
        wait_timeout: Duration,
        socket: (R, W),
    ) -> (BlockingClient, std::thread::JoinHandle<(Client<W>, R)>) {
        let ok_to_proceed = Arc::new(Mutex::new(HashSet::new()));
        let ok_cvar = Arc::new(Condvar::new());
        let precedence = load_precedence();
        let run_id = precedence.run_id;
        let requires_ok_to_proceed = Self::get_requires_ok_to_proceed(&precedence);
        let requires_notify = Self::get_requires_notify(&precedence);
        let (notification_sender2async, notification_receiver2async) =
            tokio::sync::mpsc::unbounded_channel();
        let ok_to_proceed_clone = Arc::clone(&ok_to_proceed);
        let ok_cvar_clone = Arc::clone(&ok_cvar);
        let (halt_send, halt_recv) = watch::channel(());
        let join_handle = std::thread::spawn(move || {
            let client = rt.block_on(Self::run_client(
                ok_to_proceed_clone,
                ok_cvar_clone,
                precedence,
                socket,
                notification_receiver2async,
                halt_recv,
            ));
            client
        });
        let client = BlockingClient {
            ok_to_proceed,
            ok_cvar,
            requires_ok_to_proceed,
            requires_notify,
            notification_sender: notification_sender2async,
            precid: Self::load_precid(),
            fedid: federate_id,
            wait_timeout,
            run_id,
            halt: halt_send,
        };
        info!("BlockingClient sending initial frame");
        client.send_initial_frame();
        info!("BlockingClient sent initial frame");
        (client, join_handle)
    }
    pub fn tracepoint_maybe_wait(&self, hook_invocation: HookInvocation) {
        assert!(hook_invocation.hid.1 == self.fedid);
        if self.requires_ok_to_proceed.contains(&hook_invocation) {
            debug!("{:?} requires wait", hook_invocation);
            let mut ok_to_proceed = self.ok_to_proceed.lock().unwrap();
            while !ok_to_proceed.contains(&hook_invocation) {
                let result = self
                    .ok_cvar
                    .wait_timeout(ok_to_proceed, self.wait_timeout)
                    .unwrap();
                debug!("Got notification on cvar: {:?}", result);
                ok_to_proceed = result.0;
                if result.1.timed_out() {
                    eprintln!("Timed out waiting for {:?}", hook_invocation);
                    break;
                }
            }
        }
    }
    pub fn tracepoint_maybe_notify(&self, hook_invocation: HookInvocation) {
        debug!("Checking if {:?} requires notify", hook_invocation);
        if self.requires_notify.contains(&hook_invocation) {
            let mut hook_id = [0; 32];
            for (idx, byte) in hook_invocation.hid.0.as_bytes().iter().enumerate() {
                hook_id[idx] = *byte;
            }
            debug!("Notifying {:?}", hook_invocation);
            self.notification_sender
                .send(Frame {
                    precedence_id: self.precid.0,
                    federate_id: self.fedid.0,
                    hook_id,
                    sequence_number: hook_invocation.seqnum.0,
                    run_id: self.run_id,
                })
                .unwrap();
            debug!("Notified {:?}", hook_invocation);
        }
    }
    pub fn tracepoint_maybe_do(&self, hook_invocation: HookInvocation) {
        self.tracepoint_maybe_wait(hook_invocation.clone());
        self.tracepoint_maybe_notify(hook_invocation);
    }
    async fn run_client<
        R: AsyncReadExt + Unpin + Send + 'static,
        W: AsyncWriteExt + Unpin + Send + 'static,
    >(
        ok_to_proceed: Arc<Mutex<HashSet<HookInvocation>>>,
        ok_cvar: Arc<Condvar>,
        precedence: Precedence,
        socket: (R, W),
        mut notification_receiver: tokio::sync::mpsc::UnboundedReceiver<Frame>,
        halt: watch::Receiver<()>,
    ) -> (Client<W>, R) {
        let (mut client, jh) = Client::start_from_socket(
            socket,
            Box::new(move |frame| {
                debug!("Inside callback on frame: {:?}", frame);
                let mut ok_to_proceed = ok_to_proceed.lock().unwrap();
                debug!("Got lock on ok_to_proceed");
                for hook_invocation in precedence
                    .sender2waiters
                    .get(&frame.hook_invocation())
                    .unwrap()
                {
                    ok_to_proceed.insert(hook_invocation.clone());
                }
                ok_cvar.notify_all();
            }),
            halt,
        )
        .await;
        while let Some(frame) = notification_receiver.recv().await {
            debug!("Received notification: {:?}", frame);
            client.write(frame).await;
            debug!("Wrote notification: {:?}", frame);
        }
        debug!("Client exiting");
        (client, jh.await.unwrap())
    }
    fn get_requires_ok_to_proceed(precedence: &Precedence) -> HashSet<HookInvocation> {
        let mut ret = HashSet::new();
        for waiter in precedence.sender2waiters.values().flatten() {
            ret.insert(waiter.clone());
        }
        ret
    }
    fn get_requires_notify(precedence: &Precedence) -> HashSet<HookInvocation> {
        let mut ret = HashSet::new();
        for notifier in precedence.sender2waiters.keys() {
            ret.insert(notifier.clone());
        }
        ret
    }
    fn load_precid() -> PrecedenceId {
        let id = env::var(PRECEDENCE_ID_NAME).unwrap();
        PrecedenceId(id.parse().unwrap())
    }
    fn send_initial_frame(&self) {
        self.notification_sender
            .send(Frame {
                precedence_id: self.precid.0,
                federate_id: self.fedid.0,
                hook_id: [b'S'; 32],
                sequence_number: 0,
                run_id: self.run_id,
            })
            .unwrap();
    }
}

async fn socket_from_addr<T: ToSocketAddrs + std::fmt::Debug>(addr: T) -> TcpStream {
    info!(target: "client", "Connecting to {:?}...", addr);
    let socket = TcpStream::connect(&addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to connect to {:?}: {}", addr, e));
    info!(target: "client", "Connected to {:?}", addr);
    socket
}

fn load_precedence() -> Precedence {
    let f = env::var(PRECEDENCE_FILE_NAME).unwrap();
    let f = std::fs::File::open(f).unwrap();
    rmp_serde::from_read(f).unwrap()
}
