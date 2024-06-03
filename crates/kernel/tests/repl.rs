use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use canal_kernel::repl::{self, Repl, ReplError, ReplMessage};
use googletest::prelude::*;
use std::process::{self, Command};
use tokio::{sync::mpsc, task};

// TODO: Test repl with actual repl process
// see https://stackoverflow.com/questions/77120851/rust-mocking-stdprocesschild-for-test

#[googletest::test]
#[tokio::test]
async fn repl_executes_a_code_with_mockrepl() {
    let repl_process = spawn_dummy_repl();
    let handle = repl::using::<MockRepl>(repl_process);
    let (io_sender, io_receiver) = mpsc::unbounded_channel();

    let job = task::spawn(async move {
        handle
            .execute("print('hello')".to_string(), io_sender)
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
async fn repl_executes_a_buggy_code_with_mockrepl() {
    let repl_process = spawn_dummy_repl();
    let handle = repl::using::<MockRepl>(repl_process);
    let (io_sender, io_receiver) = mpsc::unbounded_channel();

    let job =
        task::spawn(async move { handle.execute("print(*buggy*".to_string(), io_sender).await });

    // Check the Repl output
    let mut output = take_all_output(io_receiver).await;
    expect_that!(output.split(), is_utf8_string(eq("Syntax error")));

    // Check the completion status of the REPL job
    expect_that!(
        job.await.unwrap(),
        pat!(Err(pat!(ReplError::ExecutionFailed)))
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

    fn handle_message(&mut self, message: ReplMessage) {
        match message {
            ReplMessage::Execute {
                notif_sender,
                io_sender,
                code,
            } => {
                // Demo of the output of code:
                // - Working code produces `hello` from `print('hello')`
                // - Buggy code contains `buggy` and produces output `Syntax error`
                let is_buggy_code = code.contains("buggy");
                let output = if is_buggy_code {
                    "Syntax error".to_string()
                } else {
                    code.split_terminator('\'')
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>()
                        .get(1)
                        .unwrap()
                        .clone()
                };

                let output = Bytes::from(output);

                io_sender
                    .send(output)
                    .expect("IO channel for output is not open");

                // Demo of job completion notification with working and buggy code
                let _ = if is_buggy_code {
                    notif_sender.send(Err(ReplError::ExecutionFailed))
                } else {
                    notif_sender.send(Ok(()))
                };
            }
        }
    }

    async fn next_message(&mut self) -> Option<ReplMessage> {
        self.message_receiver.recv().await
    }
}
