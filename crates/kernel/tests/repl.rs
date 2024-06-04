use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use canal_kernel::repl::{self, Repl, ReplError, ReplMessage};
use googletest::prelude::*;
use std::{
    process::{self, Command},
    time::Duration,
};
use tokio::{sync::mpsc, task, time::sleep};
use tokio_util::sync::CancellationToken;

// TODO: Test repl with actual repl process
// see https://stackoverflow.com/questions/77120851/rust-mocking-stdprocesschild-for-test

#[googletest::test]
#[tokio::test]
async fn repl_executes_a_code_in_mockrepl() {
    let repl_process = spawn_dummy_repl();
    let handle = repl::using::<MockRepl>(repl_process);
    let (io_sender, io_receiver) = mpsc::unbounded_channel();
    let sigint = CancellationToken::new();
    let sigint_job = sigint.clone();

    let job = task::spawn(async move {
        handle
            .execute("print('hello')".to_string(), io_sender, sigint_job)
            .await
    });

    // Check the Repl output
    let mut output = take_all_output(io_receiver).await;
    expect_that!(output.split(), is_utf8_string(eq("hello")));

    // Check the completion status of the REPL job
    expect_that!(job.await.unwrap(), pat!(Ok(_)));
}

#[googletest::test]
#[tokio::test]
async fn repl_executes_a_buggy_code_in_mockrepl() {
    let repl_process = spawn_dummy_repl();
    let handle = repl::using::<MockRepl>(repl_process);
    let (io_sender, io_receiver) = mpsc::unbounded_channel();
    let sigint = CancellationToken::new();
    let sigint_job = sigint.clone();

    let job = task::spawn(async move {
        handle
            .execute("print(*buggy*".to_string(), io_sender, sigint_job)
            .await
    });

    // Check the Repl output
    let mut output = take_all_output(io_receiver).await;
    expect_that!(output.split(), is_utf8_string(eq("Syntax error")));

    // Check the completion status of the REPL job
    expect_that!(
        job.await.unwrap(),
        pat!(Err(pat!(ReplError::ExecutionFailed)))
    );
}

#[googletest::test]
#[tokio::test]
async fn repl_can_be_interupted_in_mockrepl() {
    let repl_process = spawn_dummy_repl();
    let handle = repl::using::<MockRepl>(repl_process);
    let (io_sender, io_receiver) = mpsc::unbounded_channel();
    let sigint = CancellationToken::new();
    let sigint_job = sigint.clone();

    let job = task::spawn(async move {
        handle
            .execute("expensive_op()".to_string(), io_sender, sigint_job)
            .await
    });

    sleep(Duration::from_micros(10)).await;
    sigint.cancel();

    // Check the Repl output
    let mut output = take_all_output(io_receiver).await;
    expect_that!(output.split(), is_utf8_string(eq("Partial output...")));

    // Check the completion status of the REPL job
    expect_that!(
        job.await.unwrap(),
        pat!(Err(pat!(ReplError::ExecutionInterupted)))
    );
}

fn spawn_dummy_repl() -> process::Child {
    Command::new(env!("CARGO_BIN_EXE_dummy_repl"))
        .spawn()
        .unwrap()
}

async fn take_all_output(mut source: mpsc::UnboundedReceiver<Bytes>) -> BytesMut {
    let mut buffer = BytesMut::new();
    while let Some(b) = source.recv().await {
        buffer.put(b);
    }

    buffer
}

struct MockRepl {
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
    async fn execute(
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
