use std::{io, process, time::Duration};

use async_trait::async_trait;
use bytes::Bytes;
use canal_kernel::{
    repl::{Repl, ReplMessage},
    ExecutionError,
};
use tokio::{sync::mpsc, time::sleep};

pub struct MockRepl {
    process: process::Child,
    message_receiver: mpsc::Receiver<ReplMessage>,
}

#[async_trait]
impl Repl for MockRepl {
    fn new(process: process::Child, message_receiver: mpsc::Receiver<ReplMessage>) -> Self {
        Self {
            message_receiver,
            process,
        }
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
                        Err(ExecutionError::Interrupted)
                    },
                };

                let _ = notif_sender.send(result);
            }
            ReplMessage::Kill { notif_sender } => {
                let result = self.process.kill();
                let _ = notif_sender.send(result);
            }
        }
    }

    async fn next_message(&mut self) -> Option<ReplMessage> {
        self.message_receiver.recv().await
    }

    async fn terminate(self) -> io::Result<()> {
        Ok(())
    }
}

impl MockRepl {
    pub async fn execute(
        &self,
        code: String,
        io_sender: mpsc::UnboundedSender<Bytes>,
    ) -> std::result::Result<(), ExecutionError> {
        // Demo of the output of code:
        // - Buggy code contains `buggy` and produces output `Syntax error`
        // - `expesive` uses sleep to simulate long operation without partial output `partial...`
        // - Working code prints anything in code
        let is_buggy_code = code.contains("buggy");
        let is_expensive_op = code.contains("expensive");

        if is_buggy_code {
            Self::simulate_buggy(io_sender).await
        } else if is_expensive_op {
            Self::simulate_expensive(io_sender).await
        } else {
            Self::simulate_print(code, io_sender).await
        }
    }

    async fn simulate_print(
        code: String,
        io_sender: mpsc::UnboundedSender<Bytes>,
    ) -> std::result::Result<(), ExecutionError> {
        let output = code;

        io_sender
            .send(output.into())
            .expect("IO channel for output is not open");

        Ok(())
    }

    async fn simulate_buggy(
        io_sender: mpsc::UnboundedSender<Bytes>,
    ) -> std::result::Result<(), ExecutionError> {
        let output = "error";

        io_sender
            .send(output.into())
            .expect("IO channel for output is not open");

        Err(ExecutionError::Failed)
    }

    async fn simulate_expensive(
        io_sender: mpsc::UnboundedSender<Bytes>,
    ) -> std::result::Result<(), ExecutionError> {
        let partial_output = "partial...";
        io_sender
            .send(partial_output.into())
            .expect("IO channel for output is not open");

        // This long running operation (with sleep) will not completely executed (dropped)
        // because the cancellation/sigint will take first to complete
        sleep(Duration::from_secs(5)).await;

        let rest_output = "...rest";
        io_sender
            .send(rest_output.into())
            .expect("IO channel for output is not open");

        Ok(())
    }
}
