use std::sync::Arc;

use bytes::Bytes;
use tokio::{
    sync::{
        mpsc::{self, error::SendError},
        OwnedSemaphorePermit, Semaphore,
    },
    task,
};
use tokio_util::sync::CancellationToken;

use crate::{
    repl::{ReplError, ReplHandle},
    Message, MessageError, MessageId,
};

pub struct KernelHandle {
    message_sender: mpsc::Sender<Message>,
    exec_result_receiver: mpsc::Receiver<Result<MessageId, MessageError>>,
}

impl KernelHandle {
    pub async fn send(&self, message: Message) -> Result<(), SendError<Message>> {
        self.message_sender.send(message).await
    }

    pub async fn recv(&mut self) -> Option<Result<MessageId, MessageError>> {
        self.exec_result_receiver.recv().await
    }
}

pub struct Kernel {
    repl: ReplHandle,
}

impl Kernel {
    async fn handle_exec(
        &self,
        exec: Exec,
        exec_result_sender: mpsc::Sender<Result<MessageId, MessageError>>,
    ) {
        let result = self
            .repl
            .execute(exec.code, exec.io_sender, exec.sigint)
            .await;

        match result {
            Ok(_) => {
                let _ = exec_result_sender.send(Ok(exec.message_id)).await;
            }
            Err(e) => match e {
                ReplError::Failed => {
                    let msg_err = Err(MessageError::Failed(exec.message_id));
                    let _ = exec_result_sender.send(msg_err).await;
                }
                ReplError::Interrupted => {
                    let msg_err = Err(MessageError::Cancelled(exec.message_id));
                    let _ = exec_result_sender.send(msg_err).await;
                }
            },
        };
    }
}

pub fn launch(repl: ReplHandle, queue_capacity: usize) -> KernelHandle {
    let (message_sender, kernel_message_receiver) = mpsc::channel(queue_capacity);

    let (exec_sender, exec_receiver) = mpsc::channel(queue_capacity);
    let (exec_cancellation_request_sender, exec_cancellation_request_receiver) = mpsc::channel(1);
    let (exec_result_sender, exec_result_receiver) = mpsc::channel(2 * queue_capacity);

    let semaphore = Arc::new(Semaphore::new(queue_capacity));

    task::spawn(process_message(
        kernel_message_receiver,
        exec_sender,
        exec_cancellation_request_sender,
        semaphore,
    ));

    let kernel = Kernel { repl };
    task::spawn(run_kernel(
        kernel,
        exec_receiver,
        exec_cancellation_request_receiver,
        exec_result_sender,
    ));

    KernelHandle {
        message_sender,
        exec_result_receiver,
    }
}

async fn run_kernel(
    kernel: Kernel,
    mut exec_receiver: mpsc::Receiver<Exec>,
    mut exec_cancellation_request_receiver: mpsc::Receiver<usize>,
    exec_result_sender: mpsc::Sender<Result<MessageId, MessageError>>,
) {
    loop {
        let exec_result_sender = exec_result_sender.clone();

        tokio::select! {
            biased;

            Some(number_of_dropped_exec) = exec_cancellation_request_receiver.recv() => {
                let mut execs = Vec::new();
                exec_receiver.recv_many(&mut execs, number_of_dropped_exec).await;

                for exec in execs.into_iter() {
                    let err = Err(MessageError::Cancelled(exec.message_id));
                    let _ = exec_result_sender.send(err).await;
                }
            }
            Some(exec) = exec_receiver.recv() => {
                kernel.handle_exec(exec, exec_result_sender).await;
            }
            else => {},
        }
    }
}

async fn process_message(
    mut message_receiver: mpsc::Receiver<Message>,
    exec_sender: mpsc::Sender<Exec>,
    exec_cancellation_request_sender: mpsc::Sender<usize>,
    semaphore: Arc<Semaphore>,
) {
    let sigint_control = CancellationToken::new();

    while let Some(msg) = message_receiver.recv().await {
        let semaphore = semaphore.clone();

        match msg {
            Message::Execute {
                message_id,
                io_sender,
                code,
            } => {
                let permit = semaphore
                    .acquire_owned()
                    .await
                    .expect("Semaphore acquire error");

                let sigint = sigint_control.child_token();

                let exec = Exec {
                    message_id,
                    code,
                    io_sender,
                    sigint,
                    permit,
                };

                let _ = exec_sender.send(exec).await;
            }
            Message::Interrupt => {
                let number_of_cancelled_exec = semaphore.available_permits();
                let _ = exec_cancellation_request_sender
                    .send(number_of_cancelled_exec)
                    .await;

                sigint_control.cancel();
            }
        }
    }
}

struct Exec {
    message_id: u32,
    code: String,
    io_sender: mpsc::UnboundedSender<Bytes>,
    sigint: CancellationToken,
    #[allow(dead_code)]
    permit: OwnedSemaphorePermit,
}
