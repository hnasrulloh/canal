use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{message::Message, repl::ReplHandle};

pub struct Kernel {
    repl: ReplHandle,
    message_source: mpsc::Receiver<Message>,
    queue_in: mpsc::Sender<Message>,
    queue_out: mpsc::Receiver<Message>,
}

impl Kernel {
    pub fn new(
        repl: ReplHandle,
        message_source: mpsc::Receiver<Message>,
        message_capacity: usize,
    ) -> Self {
        let (queue_in, queue_out) = mpsc::channel(message_capacity);

        Self {
            repl,
            message_source,
            queue_in,
            queue_out,
        }
    }

    pub async fn run(&mut self) {
        loop {
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
}
