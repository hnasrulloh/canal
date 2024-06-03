// TODO: Test repl with actual repl process
// see https://stackoverflow.com/questions/77120851/rust-mocking-stdprocesschild-for-test

use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use canal_kernel::repl::{Repl, ReplHandle, ReplMessage};
use googletest::prelude::*;
use std::process::{self, Command};
use tokio::{sync::mpsc, task};

#[googletest::test]
#[tokio::test]
async fn repl_executes_a_message_with_mockrepl() {
    let child = Command::new(env!("CARGO_BIN_EXE_dummy_repl"))
        .spawn()
        .unwrap();

    let handle = ReplHandle::new::<MockRepl>(child);

    let (io_tx, mut io_rx) = mpsc::unbounded_channel();

    let result =
        task::spawn(async move { handle.execute("print('hello')".to_string(), io_tx).await });

    let mut buffer = BytesMut::new();
    while let Some(b) = io_rx.recv().await {
        buffer.put(b);
    }

    expect_that!(buffer.split(), is_utf8_string(eq("hello")));

    expect_that!(result.await, pat!(Ok(_)));
}

struct MockRepl {
    receiver: mpsc::Receiver<ReplMessage>,
}

#[async_trait]
impl Repl for MockRepl {
    fn new(_process: process::Child, receiver: mpsc::Receiver<ReplMessage>) -> Self {
        Self { receiver }
    }

    fn handle_message(&mut self, message: ReplMessage) {
        match message {
            ReplMessage::Execute {
                responds_to,
                code,
                io_sender,
            } => {
                // Take `hello` from `print('hello')`
                let output = code
                    .split_terminator('\'')
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .get(1)
                    .unwrap()
                    .clone();

                let output = Bytes::from(output);

                io_sender
                    .send(output)
                    .expect("IO sender for output is not open");

                let _ = responds_to.send(Ok(()));
            }
        }
    }

    async fn next_message(&mut self) -> Option<ReplMessage> {
        self.receiver.recv().await
    }
}
