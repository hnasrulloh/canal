use std::{io, process};

use async_trait::async_trait;
use bytes::Bytes;
use tokio::{
    sync::{mpsc, oneshot},
    task,
};
use tokio_util::sync::CancellationToken;

use crate::ExecutionError;

#[async_trait]
pub trait Repl {
    fn new(process: process::Child, message_receiver: mpsc::Receiver<ReplMessage>) -> Self;

    async fn handle_message(&mut self, message: ReplMessage);

    async fn next_message(&mut self) -> Option<ReplMessage>;

    async fn terminate(self) -> io::Result<()>;
}

pub enum ReplMessage {
    Execute {
        notif_sender: oneshot::Sender<Result<(), ExecutionError>>,
        io_sender: mpsc::UnboundedSender<Bytes>,
        sigint: CancellationToken,
        code: String,
    },
    Kill {
        notif_sender: oneshot::Sender<io::Result<()>>,
    },
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
    ) -> Result<(), ExecutionError> {
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

    pub async fn kill(self) -> io::Result<()> {
        let (notif_sender, notif_receiver) = oneshot::channel();
        let message = ReplMessage::Kill { notif_sender };
        let _ = self.message_sender.send(message).await;

        // We expect the `io::Result`, not the `Result` from chaneel
        notif_receiver.await.expect("Repl has been killed")
    }
}

pub fn launch<R>(repl_process: process::Child) -> ReplHandle
where
    R: Repl + Send + 'static,
{
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
