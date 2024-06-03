use std::process;

use async_trait::async_trait;
use bytes::Bytes;
use thiserror::Error;
use tokio::{
    sync::{mpsc, oneshot},
    task,
};

#[async_trait]
pub trait Repl {
    fn new(process: process::Child, receiver: mpsc::Receiver<ReplMessage>) -> Self;
    fn handle_message(&mut self, message: ReplMessage);
    async fn next_message(&mut self) -> Option<ReplMessage>;
}

pub enum ReplMessage {
    Execute {
        responds_to: oneshot::Sender<Result<(), ReplError>>,
        code: String,
        io_sender: mpsc::UnboundedSender<Bytes>,
    },
}

#[derive(Error, Debug)]
pub enum ReplError {}

async fn run_repl<R: Repl>(mut repl: R) {
    while let Some(message) = repl.next_message().await {
        repl.handle_message(message);
    }
}

pub struct ReplHandle {
    sender: mpsc::Sender<ReplMessage>,
}

impl ReplHandle {
    pub fn new<R>(process: process::Child) -> Self
    where
        R: Repl + Send + 'static,
    {
        let (tx, rx) = mpsc::channel(8);
        let repl = R::new(process, rx);

        task::spawn(run_repl(repl));

        Self { sender: tx }
    }

    pub async fn execute(
        &self,
        code: String,
        io_sender: mpsc::UnboundedSender<Bytes>,
    ) -> Result<(), ReplError> {
        let (notif_tx, notif_rx) = oneshot::channel();
        let message = ReplMessage::Execute {
            code,
            io_sender,
            responds_to: notif_tx,
        };

        let _ = self.sender.send(message).await;
        notif_rx.await.expect("Repl has been killed")
    }
}
