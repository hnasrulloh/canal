use std::{collections::VecDeque, sync::Arc};

use bytes::Bytes;
use tokio::{
    sync::{mpsc, Mutex},
    task,
};
use tokio_util::sync::CancellationToken;

use crate::{message::Message, repl::ReplHandle};

pub struct Kernel {
    repl: ReplHandle,
    message_source: mpsc::Receiver<Message>,
    exec_queue: Arc<Mutex<ExecQueue>>,
}

impl Kernel {
    pub fn new(
        repl: ReplHandle,
        message_source: mpsc::Receiver<Message>,
        max_queue_capacity: usize,
    ) -> Self {
        Self {
            repl,
            message_source,
            exec_queue: Arc::new(Mutex::new(ExecQueue::new(max_queue_capacity))),
        }
    }
}

pub async fn run(kernel: Kernel) {
    let exec_queue = kernel.exec_queue.clone();
    task::spawn(async { handle_message(kernel.message_source, exec_queue).await });

    let exec_queue = kernel.exec_queue.clone();
    task::spawn(async { execute(exec_queue, kernel.repl).await });
}

async fn handle_message(mut source: mpsc::Receiver<Message>, exec_queue: Arc<Mutex<ExecQueue>>) {
    loop {
        let exec_queue = exec_queue.clone();
        let sigint = CancellationToken::new();

        match source.recv().await {
            None => (),
            Some(message) => match message {
                Message::Kill => {
                    break;
                }
                Message::Interupt => {
                    sigint.cancel();
                }
                Message::Execute { code, io_sender } => {
                    let sigint = sigint.clone();

                    let exec = Exec {
                        code,
                        io_sender,
                        sigint,
                    };

                    let mut exec_queue = exec_queue.lock().await;
                    let _ = exec_queue.send(exec).await;
                }
            },
        }
    }
}

async fn execute(exec_queue: Arc<Mutex<ExecQueue>>, repl: ReplHandle) {
    let mut exec_queue = exec_queue.lock().await;

    while let Some(exec) = exec_queue.recv().await {
        let _ = repl.execute(exec.code, exec.io_sender, exec.sigint).await;
    }
}

struct Exec {
    code: String,
    io_sender: mpsc::UnboundedSender<Bytes>,
    sigint: CancellationToken,
}

struct ExecQueue {
    max_capacity: usize,
    queue: VecDeque<Exec>,
}

impl ExecQueue {
    fn new(max_capacity: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(max_capacity),
            max_capacity,
        }
    }

    async fn send(&mut self, exec: Exec) {
        self.queue.push_back(exec);
    }

    async fn recv(&mut self) -> Option<Exec> {
        self.queue.pop_front()
    }
}
