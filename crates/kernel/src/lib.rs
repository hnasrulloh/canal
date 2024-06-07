pub mod kernel;
pub mod repl;

use bytes::Bytes;
use thiserror::Error;
use tokio::sync::mpsc;

pub type MessageId = u32;

pub enum Message {
    Execute {
        message_id: MessageId,
        code: String,
        io_sender: mpsc::UnboundedSender<Bytes>,
    },
    Interrupt,
}

#[derive(Error, Debug)]
pub enum MessageError {
    #[error("")]
    Failed(MessageId),
    #[error("")]
    Cancelled(MessageId),
}
