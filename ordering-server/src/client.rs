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
        unix, TcpStream, ToSocketAddrs,
    },
    sync::mpsc,
    task::JoinHandle,
};

use crate::{
    connection::{Connection, WriteConnection},
    server::{PRECEDENCE_FILE_NAME, PRECEDENCE_ID_NAME},
    tcpconnectionprovider::socket_from_raw_fd,
    FederateId, Frame, HookInvocation, Precedence, PrecedenceId,
};

pub struct Client<W>
where
    W: AsyncWriteExt + Unpin,
{
    connection: WriteConnection<W>,
}
pub struct ChannelClient<W>
where
    W: AsyncWriteExt + Unpin,
{
    client: Client<W>,
    pub frames: mpsc::Receiver<Frame>,
}

pub struct BlockingClient<W>
where
    W: AsyncWriteExt + Unpin,
{
    client: Arc<Mutex<Client<W>>>,
    requires_ok_to_proceed: HashSet<HookInvocation>,
    requires_notify: HashSet<HookInvocation>,
    ok_to_proceed: Arc<Mutex<HashSet<HookInvocation>>>,
    ok_cvar: Arc<Condvar>,
    rt: tokio::runtime::Runtime,
    precid: PrecedenceId,
    fedid: FederateId,
    run_id: u32,
    wait_timeout: Duration,
}

impl Client<OwnedWriteHalf> {
    pub async fn start<T: ToSocketAddrs + std::fmt::Debug>(
        addr: T,
        callback: Box<dyn FnMut(Frame) + Send>,
    ) -> (Client<OwnedWriteHalf>, JoinHandle<()>) {
        Self::start_from_socket(socket_from_addr(addr).await.into_split(), callback).await
    }
}

impl<W> Client<W>
where
    W: AsyncWriteExt + Unpin,
{
    async fn start_from_socket<R: AsyncReadExt + Unpin + Send + 'static>(
        socket: (R, W),
        mut callback: Box<dyn FnMut(Frame) + Send>,
    ) -> (Client<W>, JoinHandle<()>) {
        let (mut read, write) = Connection::new(socket).into_split();
        (
            Client { connection: write },
            tokio::spawn(async move {
                loop {
                    let frame = read.read_frame().await;
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
            }),
        )
    }
    pub async fn write(&mut self, frame: Frame) {
        self.connection.write_frame(frame).await;
    }
}

impl ChannelClient<tcp::OwnedWriteHalf> {
    pub async fn start<T: ToSocketAddrs + std::fmt::Debug>(
        addr: T,
    ) -> (ChannelClient<tcp::OwnedWriteHalf>, JoinHandle<()>) {
        ChannelClient::start_from_socket(socket_from_addr(addr).await.into_split()).await
    }
}

impl<W> ChannelClient<W>
where
    W: AsyncWriteExt + Unpin,
{
    pub async fn start_from_socket<R: AsyncReadExt + Unpin + Send + 'static>(
        socket: (R, W),
    ) -> (ChannelClient<W>, JoinHandle<()>) {
        let (frames_sender, frames_receiver) = mpsc::channel(1);
        let (client, join_handle) = Client::start_from_socket(
            socket,
            Box::new(move |frame| {
                debug!("Forwarding frame to parent: {:?}", frame);
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

impl BlockingClient<tcp::OwnedWriteHalf> {
    pub fn start<T: ToSocketAddrs + std::fmt::Debug>(
        addr: T,
        federate_id: FederateId,
        wait_timeout: Duration,
    ) -> (
        BlockingClient<tcp::OwnedWriteHalf>,
        std::thread::JoinHandle<()>,
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();
        let socket = rt.block_on(socket_from_addr(addr));
        Self::start_from_socket(rt, federate_id, wait_timeout, socket.into_split())
    }
}

impl BlockingClient<unix::OwnedWriteHalf> {
    pub fn start_reusing_connection(
        federate_id: FederateId,
        wait_timeout: Duration,
    ) -> (
        BlockingClient<unix::OwnedWriteHalf>,
        std::thread::JoinHandle<()>,
    ) {
        let raw_fd = crate::server::connection_raw_fd_for(federate_id);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();
        let socket = rt.block_on(async { unsafe { socket_from_raw_fd(raw_fd) } });
        Self::start_from_socket(rt, federate_id, wait_timeout, socket)
    }
}

impl<W> BlockingClient<W>
where
    W: AsyncWriteExt + Unpin,
{
    fn start_from_socket<R: AsyncReadExt + Unpin + Send + 'static>(
        rt: tokio::runtime::Runtime,
        federate_id: FederateId,
        wait_timeout: Duration,
        socket: (R, W),
    ) -> (BlockingClient<W>, std::thread::JoinHandle<()>) {
        let ok_to_proceed = Arc::new(Mutex::new(HashSet::new()));
        let ok_cvar = Arc::new(Condvar::new());
        let precedence = load_precedence();
        let run_id = precedence.run_id;
        let requires_ok_to_proceed = Self::get_requires_ok_to_proceed(&precedence);
        let requires_notify = Self::get_requires_notify(&precedence);
        let (client, jh) = Self::get_client(
            &rt,
            Arc::clone(&ok_to_proceed),
            Arc::clone(&ok_cvar),
            precedence,
            socket,
        );
        let join_handle = std::thread::spawn(move || {
            rt.block_on(jh).unwrap();
        });
        let client = BlockingClient {
            client: Arc::new(Mutex::new(client)),
            ok_to_proceed,
            ok_cvar,
            requires_ok_to_proceed,
            requires_notify,
            rt: tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .build()
                .unwrap(),
            precid: Self::load_precid(),
            fedid: federate_id,
            wait_timeout,
            run_id,
        };
        client.send_initial_frame();
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
            self.rt.block_on(self.client.lock().unwrap().write(Frame {
                precedence_id: self.precid.0,
                federate_id: self.fedid.0,
                hook_id,
                sequence_number: hook_invocation.seqnum.0,
                run_id: self.run_id,
            }));
        }
    }
    pub fn tracepoint_maybe_do(&self, hook_invocation: HookInvocation) {
        self.tracepoint_maybe_wait(hook_invocation.clone());
        self.tracepoint_maybe_notify(hook_invocation);
    }
    fn get_client<R: AsyncReadExt + Unpin + Send + 'static>(
        rt: &tokio::runtime::Runtime,
        ok_to_proceed: Arc<Mutex<HashSet<HookInvocation>>>,
        ok_cvar: Arc<Condvar>,
        precedence: Precedence,
        socket: (R, W),
    ) -> (Client<W>, tokio::task::JoinHandle<()>) {
        rt.block_on(Client::start_from_socket(
            socket,
            Box::new(move |frame| {
                debug!("Inside callback on frame: {:?}", frame);
                let mut ok_to_proceed = ok_to_proceed.lock().unwrap();
                for hook_invocation in precedence
                    .sender2waiters
                    .get(&frame.hook_invocation())
                    .unwrap()
                {
                    ok_to_proceed.insert(hook_invocation.clone());
                }
                ok_cvar.notify_all();
            }),
        ))
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
        self.rt.block_on(self.client.lock().unwrap().write(Frame {
            precedence_id: self.precid.0,
            federate_id: self.fedid.0,
            hook_id: [b'S'; 32],
            sequence_number: 0,
            run_id: self.run_id,
        }));
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
