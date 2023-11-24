use bytes::BufMut;
use log::debug;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;

use crate::{FederateId, Frame, HookId, HookInvocation};

const FRAME_SIZE: usize = std::mem::size_of::<Frame>();
struct FrameBuffer([u8; FRAME_SIZE], usize);
impl Default for FrameBuffer {
    fn default() -> Self {
        Self([0; FRAME_SIZE], 0)
    }
}
pub struct Connection {
    read: ReadConnection,
    write: WriteConnection,
}
pub struct ReadConnection {
    stream: OwnedReadHalf,
    buffer: FrameBuffer,
}
pub struct WriteConnection {
    stream: BufWriter<OwnedWriteHalf>,
}

unsafe impl BufMut for FrameBuffer {
    fn remaining_mut(&self) -> usize {
        FRAME_SIZE - self.1
    }

    unsafe fn advance_mut(&mut self, cnt: usize) {
        self.1 += cnt;
    }

    fn chunk_mut(&mut self) -> &mut bytes::buf::UninitSlice {
        unsafe { std::mem::transmute(&mut self.0[self.1..]) }
    }
}

impl ReadConnection {
    pub async fn read_frame(&mut self) -> Option<Frame> {
        debug!("Reading frame...");
        if self.stream.read_buf(&mut self.buffer).await.unwrap() == 0 {
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

impl WriteConnection {
    pub async fn write_frame(&mut self, frame: Frame) {
        self.stream
            .write_all(&unsafe { std::mem::transmute::<Frame, [u8; FRAME_SIZE]>(frame) })
            .await
            .unwrap();
        self.stream.flush().await.unwrap();
    }
}
impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        let (read, write) = stream.into_split();
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
    pub fn into_split(self) -> (ReadConnection, WriteConnection) {
        (self.read, self.write)
    }
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
