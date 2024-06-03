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
    fn new(process: process::Child, message_receiver: mpsc::Receiver<ReplMessage>) -> Self;

    fn handle_message(&mut self, message: ReplMessage);

    async fn next_message(&mut self) -> Option<ReplMessage>;
}

pub enum ReplMessage {
    Execute {
        notif_sender: oneshot::Sender<Result<(), ReplError>>,
        io_sender: mpsc::UnboundedSender<Bytes>,
        code: String,
    },
}

#[derive(Error, Debug)]
pub enum ReplError {
    #[error("REPL could not execute the code properly")]
    ExecutionFailed,
}

async fn run_repl<R: Repl>(mut repl: R) {
    while let Some(message) = repl.next_message().await {
        repl.handle_message(message);
    }
}

pub struct ReplHandle {
    message_sender: mpsc::Sender<ReplMessage>,
}

impl ReplHandle {
    pub fn new<R>(process: process::Child) -> Self
    where
        R: Repl + Send + 'static,
    {
        let (message_sender, message_receiver) = mpsc::channel(1);
        let repl = R::new(process, message_receiver);

        task::spawn(run_repl(repl));

        Self { message_sender }
    }

    pub async fn execute(
        &self,
        code: String,
        io_sender: mpsc::UnboundedSender<Bytes>,
    ) -> Result<(), ReplError> {
        let (notif_sender, notif_receiver) = oneshot::channel();
        let message = ReplMessage::Execute {
            code,
            io_sender,
            notif_sender,
        };

        let _ = self.message_sender.send(message).await;
        notif_receiver.await.expect("Repl has been killed")
    }
}
