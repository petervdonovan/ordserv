use std::{
    io,
    os::fd::{FromRawFd, IntoRawFd, RawFd},
};

use bytes::BufMut;
use log::{debug, info};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::{unix, UnixStream},
};

use crate::{FederateId, Frame, HookId, HookInvocation};

const FRAME_SIZE: usize = std::mem::size_of::<Frame>();
struct FrameBuffer([u8; FRAME_SIZE], usize);
impl Default for FrameBuffer {
    fn default() -> Self {
        Self([0; FRAME_SIZE], 0)
    }
}
pub struct Connection<R, W>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    read: ReadConnection<R>,
    write: WriteConnection<W>,
}
pub struct ReadConnection<R>
where
    R: AsyncReadExt + Unpin,
{
    pub stream: R, // FIXME: should be private
    buffer: FrameBuffer,
}
impl<R> ReadConnection<R>
where
    R: AsyncReadExt + Unpin,
{
    pub fn new(stream: R) -> Self {
        Self {
            stream,
            buffer: FrameBuffer::default(),
        }
    }
}

pub struct WriteConnection<W>
where
    W: AsyncWriteExt + Unpin,
{
    pub stream: BufWriter<W>, // FIXME: should be private
}

unsafe impl BufMut for FrameBuffer {
    // FIXME: not thread-safe
    fn remaining_mut(&self) -> usize {
        FRAME_SIZE - self.1
    }

    unsafe fn advance_mut(&mut self, cnt: usize) {
        if self.1 + cnt > FRAME_SIZE {
            panic!("Frame buffer overflow");
        }
        self.1 += cnt;
    }

    fn chunk_mut(&mut self) -> &mut bytes::buf::UninitSlice {
        if self.1 % FRAME_SIZE != 0 {
            panic!("Frame buffer is not aligned");
        }
        unsafe { std::mem::transmute(&mut self.0[self.1..]) }
    }
}

impl<R> ReadConnection<R>
where
    R: AsyncReadExt + Unpin,
{
    pub async fn read_frame(&mut self) -> Option<Frame> {
        debug!("Reading frame...");
        if self.stream.read_buf(&mut self.buffer).await.unwrap_or(0) == 0 {
            return None;
        }
        debug!("Got frame");
        if self.buffer.1 < std::mem::size_of::<Frame>() {
            panic!("Frame buffer is too small");
        }
        let frame = unsafe {
            std::mem::transmute::<[u8; std::mem::size_of::<Frame>()], Frame>(
                self.buffer.0[0..std::mem::size_of::<Frame>()]
                    .try_into()
                    .unwrap(),
            )
        };
        self.buffer
            .0
            .copy_within(std::mem::size_of::<Frame>()..FRAME_SIZE, 0);
        self.buffer.1 -= std::mem::size_of::<Frame>();
        Some(frame)
    }
}

impl<W> WriteConnection<W>
where
    W: AsyncWriteExt + Unpin,
{
    pub async fn write_frame(&mut self, frame: Frame) {
        debug!("Writing frame to the socket: {:?}", frame);
        self.stream
            .write_all(&unsafe { std::mem::transmute::<Frame, [u8; FRAME_SIZE]>(frame) })
            .await
            .unwrap();
        debug!("Flushing frame to the socket");
        let result = self.stream.flush().await;
        debug!("Flushed frame to the socket");
        if let Err(e) = result {
            debug!("Failed to flush frame: {:?}", e);
        }
    }
}

impl<R, W> Connection<R, W>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    pub fn new(stream: (R, W)) -> Self {
        let (read, write) = stream;
        Self {
            read: ReadConnection {
                stream: read,
                buffer: FrameBuffer::default(),
            },
            write: WriteConnection {
                stream: BufWriter::new(write),
            },
        }
    }
    pub async fn read_frame(&mut self) -> Option<Frame> {
        self.read.read_frame().await
    }
    #[allow(dead_code)]
    pub async fn write_frame(&mut self, frame: Frame) {
        self.write.write_frame(frame).await
    }
    pub async fn close(mut self) {
        info!("Closing connection");
        let _ = self.write.stream.shutdown().await; // if this fails, it was already closed anyway
    }
    pub fn into_split(self) -> (ReadConnection<R>, WriteConnection<W>) {
        (self.read, self.write)
    }
}

/// ### Safety
///
/// Creation and destruction functions for a connection assuming that the file handle is mutably
/// borrowed, not owned -- this is not expressed in the type system, although it should be, hence
/// the use of `unsafe`.
pub struct ConnectionManagement<R, W>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    pub borrow: unsafe fn(RawFd) -> io::Result<Connection<R, W>>,
    pub unborrow: unsafe fn((ReadConnection<R>, WriteConnection<W>)),
}

pub struct CreateConnectionError {
    pub message: String,
}

pub const UNIX_CONNECTION_MANAGEMENT: ConnectionManagement<
    unix::OwnedReadHalf,
    unix::OwnedWriteHalf,
> = ConnectionManagement {
    borrow: |fd| Ok(unsafe { Connection::new(socket_from_raw_fd(fd)?) }),
    unborrow: |(r, w)| {
        let reunited = r
            .stream
            .reunite(w.stream.into_inner())
            .expect("Failed to reunite connection");
        reunited.into_std().unwrap().into_raw_fd();
    },
};

unsafe fn socket_from_raw_fd(fd: RawFd) -> io::Result<(unix::OwnedReadHalf, unix::OwnedWriteHalf)> {
    let std = std::os::unix::net::UnixStream::from_raw_fd(fd);
    std.set_nonblocking(true)?;
    info!("recovering socket from std: {:?}", std);
    Ok(UnixStream::from_std(std)?.into_split())
}

impl Frame {
    pub fn hid(&self) -> HookId {
        let nul_range_end = self
            .hook_id
            .iter()
            .position(|&c| c == b'\0')
            .unwrap_or(self.hook_id.len()); // default to length if no `\0` present
        HookId(
            std::str::from_utf8(&self.hook_id[0..nul_range_end])
                .unwrap()
                .to_string(),
            FederateId(self.federate_id),
        )
    }
    pub fn hook_invocation(&self) -> HookInvocation {
        HookInvocation {
            hid: self.hid(),
            seqnum: crate::SequenceNumberByFileAndLine(self.sequence_number),
        }
    }
}
