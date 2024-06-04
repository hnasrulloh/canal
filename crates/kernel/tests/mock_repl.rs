use std::{process, time::Duration};

use async_trait::async_trait;
use bytes::Bytes;
use canal_kernel::repl::{Repl, ReplError, ReplMessage};
use tokio::{sync::mpsc, time::sleep};

pub struct MockRepl {
    message_receiver: mpsc::Receiver<ReplMessage>,
}

#[async_trait]
impl Repl for MockRepl {
    fn new(_process: process::Child, message_receiver: mpsc::Receiver<ReplMessage>) -> Self {
        Self { message_receiver }
    }

    async fn handle_message(&mut self, message: ReplMessage) {
        match message {
            ReplMessage::Execute {
                notif_sender,
                io_sender,
                sigint,
                code,
            } => {
                let result = tokio::select! {
                    execution_result = self.execute(code, io_sender) => {
                        execution_result
                    },
                    _ = sigint.cancelled() => {
                        Err(ReplError::ExecutionInterupted)
                    },
                };

                let _ = notif_sender.send(result);
            }
        }
    }

    async fn next_message(&mut self) -> Option<ReplMessage> {
        self.message_receiver.recv().await
    }
}

impl MockRepl {
    pub async fn execute(
        &self,
        code: String,
        io_sender: mpsc::UnboundedSender<Bytes>,
    ) -> std::result::Result<(), ReplError> {
        // Demo of the output of code:
        // - Working code produces `hello` from `print('hello')`
        // - Buggy code contains `buggy` and produces output `Syntax error`
        // - `expesive_op` uses sleep to simulate long operation
        let is_buggy_code = code.contains("buggy");
        let is_expensive_op = code.contains("expensive_op");

        if is_buggy_code {
            Self::simulate_buggy(io_sender).await
        } else if is_expensive_op {
            Self::simulate_expensive(io_sender).await
        } else {
            Self::simulate_working(io_sender).await
        }
    }

    async fn simulate_working(
        io_sender: mpsc::UnboundedSender<Bytes>,
    ) -> std::result::Result<(), ReplError> {
        let output = "hello";

        io_sender
            .send(output.into())
            .expect("IO channel for output is not open");

        Ok(())
    }

    async fn simulate_buggy(
        io_sender: mpsc::UnboundedSender<Bytes>,
    ) -> std::result::Result<(), ReplError> {
        let output = "Syntax error";

        io_sender
            .send(output.into())
            .expect("IO channel for output is not open");

        Err(ReplError::ExecutionFailed)
    }

    async fn simulate_expensive(
        io_sender: mpsc::UnboundedSender<Bytes>,
    ) -> std::result::Result<(), ReplError> {
        let partial_output = "Partial output...";
        io_sender
            .send(partial_output.into())
            .expect("IO channel for output is not open");

        // This long running operation (with sleep) will not completely executed (dropped)
        // because the cancellation/sigint will take first to complete
        sleep(Duration::from_secs(10)).await;

        let rest_output = "rest of output";
        io_sender
            .send(rest_output.into())
            .expect("IO channel for output is not open");

        Ok(())
    }
}
