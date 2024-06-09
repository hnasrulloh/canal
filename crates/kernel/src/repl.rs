use std::{process, sync::Arc};

use async_trait::async_trait;
use bytes::Bytes;
use thiserror::Error;
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    task,
};
use tokio_util::sync::CancellationToken;

#[async_trait]
pub trait Repl {
    fn new(
        process: Arc<Mutex<process::Child>>,
        message_receiver: mpsc::Receiver<ReplMessage>,
    ) -> Self;

    async fn handle_message(&mut self, message: ReplMessage);

    async fn next_message(&mut self) -> Option<ReplMessage>;
}

pub enum ReplMessage {
    Execute {
        notif_sender: oneshot::Sender<Result<(), ReplError>>,
        io_sender: mpsc::UnboundedSender<Bytes>,
        sigint: CancellationToken,
        code: String,
    },
}

#[derive(Error, Debug)]
pub enum ReplError {
    #[error("Execution failed")]
    Failed,
    #[error("Execution was interrupted")]
    Interrupted,
}

pub struct ReplHandle {
    message_sender: mpsc::Sender<ReplMessage>,
}

impl ReplHandle {
    pub async fn execute(
        &self,
        code: String,
        io_sender: mpsc::UnboundedSender<Bytes>,
        sigint: CancellationToken,
    ) -> Result<(), ReplError> {
        let (notif_sender, notif_receiver) = oneshot::channel();
        let message = ReplMessage::Execute {
            code,
            sigint,
            io_sender,
            notif_sender,
        };

        let _ = self.message_sender.send(message).await;
        notif_receiver.await.expect("Repl has been killed")
    }
}

pub fn launch<R>(repl_process: Arc<Mutex<process::Child>>) -> ReplHandle
where
    R: Repl + Send + 'static,
{
    // Minimize message loss by using blocking message with limited number of buffer in channel
    let (message_sender, message_receiver) = mpsc::channel(1);
    let repl = R::new(repl_process, message_receiver);

    task::spawn(run_repl(repl));

    ReplHandle { message_sender }
}

async fn run_repl<R: Repl>(mut repl: R) {
    while let Some(message) = repl.next_message().await {
        repl.handle_message(message).await;
    }
}
