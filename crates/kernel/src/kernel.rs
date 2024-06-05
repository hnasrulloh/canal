use bytes::Bytes;
use tokio::sync::mpsc;
use tokio_stream::Stream;
use tokio_util::sync::CancellationToken;

use crate::{
    message::Message,
    message_queue::MessageQueue,
    repl::{self, Repl, ReplHandle},
};

pub struct Kernel {
    pub repl: ReplHandle,
    pub message_queue: MessageQueue<Message>,
    pub message_source: mpsc::Receiver<Message>,
}

impl Kernel {
    pub async fn run(&mut self) {
        match self.message_source.recv().await {
            None => (),
            Some(message) => match message {
                Message::Kill => (),
                Message::Interupt => (),
                Message::Execute { code, io_sender } => {
                    let sigint = CancellationToken::new();
                    let sigint_job = sigint.clone();
                    let _ = self.repl.execute(code, io_sender, sigint_job).await;
                }
            },
        }
    }
}
