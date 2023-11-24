use std::{
    collections::HashSet,
    env,
    sync::{Arc, Condvar, Mutex},
};

use log::{debug, info};
use tokio::{
    net::{TcpStream, ToSocketAddrs},
    sync::mpsc,
    task::JoinHandle,
};

use crate::{
    connection::{Connection, WriteConnection},
    server::{PRECEDENCE_FILE_NAME, PRECEDENCE_ID_NAME},
    FederateId, Frame, HookInvocation, Precedence, PrecedenceId,
};

pub struct Client {
    connection: WriteConnection,
}
pub struct ChannelClient {
    client: Client,
    pub frames: mpsc::Receiver<Frame>,
}

pub struct BlockingClient {
    client: Arc<Mutex<Client>>,
    requires_ok_to_proceed: HashSet<HookInvocation>,
    requires_notify: HashSet<HookInvocation>,
    ok_to_proceed: Arc<Mutex<HashSet<HookInvocation>>>,
    ok_cvar: Arc<Condvar>,
    rt: tokio::runtime::Runtime,
    precid: PrecedenceId,
    fedid: FederateId,
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
                            debug!("Invoking callback on frame: {:?}", frame);
                            callback(frame);
                        }
                        None => {
                            info!(target: "client", "Connection closed");
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

impl BlockingClient {
    pub fn start<T: ToSocketAddrs>(
        addr: T,
        federate_id: u32,
    ) -> (BlockingClient, std::thread::JoinHandle<()>) {
        let ok_to_proceed = Arc::new(Mutex::new(HashSet::new()));
        let ok_cvar = Arc::new(Condvar::new());
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();
        let precedence = load_precedence();
        let requires_ok_to_proceed = Self::get_requires_ok_to_proceed(&precedence);
        let requires_notify = Self::get_requires_notify(&precedence);
        let (client, jh) = Self::get_client(
            &rt,
            addr,
            Arc::clone(&ok_to_proceed),
            Arc::clone(&ok_cvar),
            precedence,
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
            fedid: FederateId(federate_id),
        };
        client.send_initial_frame();
        (client, join_handle)
    }
    pub fn tracepoint_maybe_wait(&self, hook_invocation: HookInvocation) {
        if self.requires_ok_to_proceed.contains(&hook_invocation) {
            let mut ok_to_proceed = self.ok_to_proceed.lock().unwrap();
            while !ok_to_proceed.contains(&hook_invocation) {
                ok_to_proceed = self.ok_cvar.wait(ok_to_proceed).unwrap();
            }
        }
    }
    pub fn tracepoint_maybe_notify(&self, hook_invocation: HookInvocation) {
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
            }));
        }
    }
    pub fn tracepoint_maybe_do(&self, hook_invocation: HookInvocation) {
        self.tracepoint_maybe_wait(hook_invocation.clone());
        self.tracepoint_maybe_notify(hook_invocation);
    }
    fn get_client<T: ToSocketAddrs>(
        rt: &tokio::runtime::Runtime,
        addr: T,
        ok_to_proceed: Arc<Mutex<HashSet<HookInvocation>>>,
        ok_cvar: Arc<Condvar>,
        precedence: Precedence,
    ) -> (Client, tokio::task::JoinHandle<()>) {
        rt.block_on(Client::start(
            addr,
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
        }));
    }
}

fn load_precedence() -> Precedence {
    let f = env::var(PRECEDENCE_FILE_NAME).unwrap();
    let f = std::fs::File::open(f).unwrap();
    rmp_serde::from_read(f).unwrap()
}
