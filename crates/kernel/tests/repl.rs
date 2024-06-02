// TODO: Test repl with actual repl process
// see https://stackoverflow.com/questions/77120851/rust-mocking-stdprocesschild-for-test

use async_trait::async_trait;
use canal_kernel::repl::{Repl, ReplHandle, ReplMessage, ReplStatus};
use googletest::prelude::*;
use std::process::{self, Command};
use tokio::sync::mpsc;

#[googletest::test]
#[tokio::test]
async fn get_repl_status_with_mock() {
    let child = Command::new(env!("CARGO_BIN_EXE_dummy_repl"))
        .spawn()
        .unwrap();

    let handle = ReplHandle::new::<MockRepl>(child);

    let status = handle.get_status().await;
    expect_that!(status, pat!(ReplStatus::Idle));
}

struct MockRepl {
    receiver: mpsc::Receiver<ReplMessage>,
}

#[async_trait]
impl Repl for MockRepl {
    fn new(_process: process::Child, receiver: mpsc::Receiver<ReplMessage>) -> Self {
        MockRepl { receiver }
    }

    fn handle_message(&mut self, message: ReplMessage) {
        match message {
            ReplMessage::GetStatus { resonds_to } => {
                let _ = resonds_to.send(ReplStatus::Idle);
            }
        }
    }

    async fn next_message(&mut self) -> Option<ReplMessage> {
        self.receiver.recv().await
    }
}
