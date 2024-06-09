use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use tokio::{
    sync::{mpsc, OwnedSemaphorePermit, Semaphore},
    task,
    time::sleep,
};
use tokio_util::sync::CancellationToken;

use crate::{
    repl::{ReplError, ReplHandle},
    Request, Response,
};

pub struct KernelTerminal {
    request_sender: mpsc::Sender<Request>,
    response_receiver: mpsc::Receiver<Response>,
}

impl KernelTerminal {
    pub async fn send(&self, message: Request) {
        self.request_sender
            .send(message)
            .await
            .expect("Kernel is killed")
    }

    pub async fn recv(&mut self) -> Option<Response> {
        self.response_receiver.recv().await
    }
}

pub struct Kernel {
    repl: ReplHandle,
}

impl Kernel {
    async fn handle_exec(
        &self,
        exec: Exec,
        response_sender: mpsc::Sender<Response>,
        exec_queue_cancellation: CancellationToken,
    ) {
        let result = self
            .repl
            .execute(exec.code, exec.io_sender, exec.sigint)
            .await;

        match result {
            Ok(_) => {
                let _ = response_sender
                    .send(Response::Success(exec.message_id))
                    .await;
            }
            Err(err) => {
                match err {
                    ReplError::Failed => {
                        let _ = response_sender
                            .send(Response::Failed(exec.message_id))
                            .await;
                    }
                    ReplError::Interrupted => {
                        let _ = response_sender
                            .send(Response::Cancelled(exec.message_id))
                            .await;
                    }
                }

                exec_queue_cancellation.cancel();
            }
        };
    }
}

pub fn launch(repl: ReplHandle, queue_capacity: usize) -> (KernelTerminal, Arc<Semaphore>) {
    let (request_sender, request_receiver) = mpsc::channel(queue_capacity);
    let (response_sender, response_receiver) = mpsc::channel(2 * queue_capacity);

    let (exec_sender, exec_receiver) = mpsc::channel(queue_capacity);

    let (queue_cancellation_info_sender, queue_cancellation_info_receiver) = mpsc::channel(1);
    let queue_cancellation_token = CancellationToken::new();

    let queue_semaphore = Arc::new(Semaphore::new(queue_capacity));

    task::spawn(process_request(
        request_receiver,
        exec_sender,
        queue_cancellation_token.clone(),
        queue_semaphore.clone(),
    ));

    let kernel = Kernel { repl };
    task::spawn(process_exec(
        kernel,
        exec_receiver,
        response_sender,
        queue_cancellation_info_receiver,
        queue_cancellation_token.clone(),
    ));

    task::spawn(watch_cancellation_and_send_queue_cancellation_info(
        queue_cancellation_info_sender.clone(),
        queue_cancellation_token.clone(),
        queue_semaphore.clone(),
    ));

    let terminal = KernelTerminal {
        request_sender,
        response_receiver,
    };

    (terminal, queue_semaphore)
}

async fn process_exec(
    kernel: Kernel,
    mut exec_receiver: mpsc::Receiver<Exec>,
    response_sender: mpsc::Sender<Response>,
    mut queue_cancellation_info_receiver: mpsc::Receiver<usize>,
    queue_cancellation_token: CancellationToken,
) {
    loop {
        let exec_result_sender = response_sender.clone();

        tokio::select! {
            biased;

            Some(number_of_dropped_exec) = queue_cancellation_info_receiver.recv() => {
                let mut execs = Vec::new();
                exec_receiver.recv_many(&mut execs, number_of_dropped_exec).await;

                for exec in execs.into_iter() {
                    let _ = exec_result_sender.send(Response::Cancelled(exec.message_id)).await;
                }
            }
            Some(exec) = exec_receiver.recv() => {
                kernel.handle_exec(exec, exec_result_sender, queue_cancellation_token.clone()).await;

                // This emulates latency of inter-process communication between kernel and REPL process.
                // The average of time needed to send data is around 4-10 microseconds.
                //
                // Without any latency between the kernel and a real REPL process, the cancellation request
                // (sent by a mpsc channel) will arrive slightly slower than the execution process by REPL.
                if cfg!(debug_assertions) {
                    sleep(Duration::from_micros(4)).await;
                }
            }
            else => {},
        }
    }
}

async fn process_request(
    mut request_receiver: mpsc::Receiver<Request>,
    exec_sender: mpsc::Sender<Exec>,
    queue_cancellation_token: CancellationToken,
    queue_semaphore: Arc<Semaphore>,
) {
    let sigint_control = CancellationToken::new();

    while let Some(msg) = request_receiver.recv().await {
        let semaphore = queue_semaphore.clone();

        match msg {
            Request::Execute {
                message_id,
                io_sender,
                code,
            } => {
                let queue_permit = semaphore
                    .acquire_owned()
                    .await
                    .expect("Queue semaphore could not acquire");

                let sigint = sigint_control.child_token();

                let exec = Exec {
                    message_id,
                    code,
                    io_sender,
                    sigint,
                    queue_permit,
                };

                let _ = exec_sender.send(exec).await;
            }
            Request::Interrupt => {
                queue_cancellation_token.cancel();
                sigint_control.cancel();
            }
        }
    }
}

async fn watch_cancellation_and_send_queue_cancellation_info(
    queue_cancellation_info_sender: mpsc::Sender<usize>,
    queue_cancellation_token: CancellationToken,
    queue_semaphore: Arc<Semaphore>,
) {
    loop {
        tokio::select! {
            _ = queue_cancellation_token.cancelled() => {
                let number_of_cancelled_exec = queue_semaphore.available_permits();
                let _ = queue_cancellation_info_sender.send(number_of_cancelled_exec).await;
            }
            else => {}
        }
    }
}

struct Exec {
    message_id: u32,
    code: String,
    io_sender: mpsc::UnboundedSender<Bytes>,
    sigint: CancellationToken,
    #[allow(dead_code)]
    queue_permit: OwnedSemaphorePermit,
}
