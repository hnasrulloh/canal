pub mod kernel;
pub mod repl;

use bytes::Bytes;
use tokio::sync::mpsc;

pub type MessageId = u32;

#[derive(Debug)]
pub enum KernelRequest {
    Execute {
        message_id: MessageId,
        code: String,
        io_sender: mpsc::UnboundedSender<Bytes>,
    },
    Interrupt,
}

#[derive(Debug)]
pub enum KernelResponse {
    Success(MessageId),
    Failed(MessageId),
    Cancelled(MessageId),
}
