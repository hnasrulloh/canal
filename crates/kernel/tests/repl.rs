mod mock_repl;
mod utils;

use bytes::{BufMut, Bytes, BytesMut};
use canal_kernel::repl::{self, ReplError};
use googletest::prelude::*;
use mock_repl::MockRepl;
use std::time::Duration;
use tokio::{sync::mpsc, task, time::sleep};
use tokio_util::sync::CancellationToken;
use utils::spawn_dummy_repl;

// TODO: Test repl with actual repl process with stdin and stdout
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

#[googletest::test]
#[tokio::test]
async fn repl_can_be_killed_in_mockrepl() {
    let repl_process = spawn_dummy_repl();
    let handle = repl::using::<MockRepl>(repl_process);

    let result = handle.kill().await;

    expect_that!(result, pat!(Ok(())));
}

async fn take_all_output(mut source: mpsc::UnboundedReceiver<Bytes>) -> BytesMut {
    let mut buffer = BytesMut::new();
    while let Some(b) = source.recv().await {
        buffer.put(b);
    }

    buffer
}
