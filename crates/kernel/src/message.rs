use bytes::Bytes;
use tokio::sync::mpsc;

pub enum Message {
    Execute {
        code: String,
        io_sender: mpsc::UnboundedSender<Bytes>,
    },
    Interupt,
    Kill,
}
