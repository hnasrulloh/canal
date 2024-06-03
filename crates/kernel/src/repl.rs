use std::process;

use async_trait::async_trait;
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
    GetStatus {
        resonds_to: oneshot::Sender<ReplStatus>,
    },
}

#[derive(Debug)]
pub enum ReplStatus {
    Idle,
}

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

    pub async fn get_status(&self) -> ReplStatus {
        let (tx, rx) = oneshot::channel();
        let message = ReplMessage::GetStatus { resonds_to: tx };

        let _ = self.sender.send(message).await;
        rx.await.expect("Repl has been killed")
    }
}
