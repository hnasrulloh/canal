mod mock_repl;
mod utils;

use canal_kernel::repl::{self, ReplError};
use googletest::prelude::*;
use mock_repl::MockRepl;
use std::time::Duration;
use tokio::{sync::mpsc, task, time::sleep};
use tokio_util::sync::CancellationToken;
use utils::{spawn_dummy_repl, take_all_output};

// TODO: Test repl with mock repl process with stdin and stdout
// see https://stackoverflow.com/questions/77120851/rust-mocking-stdprocesschild-for-test

#[googletest::test]
#[tokio::test]
async fn repl_executes_a_code_in_mockrepl() {
    let repl_process = spawn_dummy_repl();
    let handle = repl::launch::<MockRepl>(repl_process);
    let (io_sender, io_receiver) = mpsc::unbounded_channel();
    let sigint = CancellationToken::new();
    let sigint_job = sigint.clone();

    let job =
        task::spawn(async move { handle.execute("1".to_string(), io_sender, sigint_job).await });

    // Check the Repl output
    expect_that!(take_all_output(io_receiver).await, is_utf8_string(eq("1")));

    // Check the completion status of the REPL job
    expect_that!(job.await.unwrap(), pat!(Ok(_)));
}

#[googletest::test]
#[tokio::test]
async fn repl_executes_a_buggy_code_in_mockrepl() {
    let repl_process = spawn_dummy_repl();
    let handle = repl::launch::<MockRepl>(repl_process);
    let (io_sender, io_receiver) = mpsc::unbounded_channel();
    let sigint = CancellationToken::new();
    let sigint_job = sigint.clone();

    let job = task::spawn(async move {
        handle
            .execute("buggy".to_string(), io_sender, sigint_job)
            .await
    });

    // Check the Repl output
    expect_that!(
        take_all_output(io_receiver).await,
        is_utf8_string(eq("error"))
    );

    // Check the completion status of the REPL job
    expect_that!(job.await.unwrap(), pat!(Err(pat!(ReplError::Failed))));
}

#[googletest::test]
#[tokio::test]
async fn repl_can_be_interupted_in_mockrepl() {
    let repl_process = spawn_dummy_repl();
    let handle = repl::launch::<MockRepl>(repl_process);
    let (io_sender, io_receiver) = mpsc::unbounded_channel();
    let sigint = CancellationToken::new();
    let sigint_job = sigint.clone();

    let job = task::spawn(async move {
        handle
            .execute("expensive".to_string(), io_sender, sigint_job)
            .await
    });

    sleep(Duration::from_micros(10)).await;
    sigint.cancel();

    // Check the Repl output
    expect_that!(
        take_all_output(io_receiver).await,
        is_utf8_string(eq("partial..."))
    );

    // Check the completion status of the REPL job
    expect_that!(job.await.unwrap(), pat!(Err(pat!(ReplError::Interrupted))));
}

#[googletest::test]
#[tokio::test]
async fn repl_can_be_killed_in_mockrepl() {
    let repl_process = spawn_dummy_repl();
    let handle = repl::launch::<MockRepl>(repl_process);

    let result = handle.kill().await;

    expect_that!(result, pat!(Ok(())));
}
