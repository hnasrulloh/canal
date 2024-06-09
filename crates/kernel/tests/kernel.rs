mod mock_repl;
mod utils;

use std::time::Duration;

use bytes::Bytes;
use canal_kernel::{
    kernel::{self, KernelTerminal},
    repl, KernelRequest, KernelResponse,
};
use googletest::prelude::*;
use mock_repl::MockRepl;
use tokio::{sync::mpsc, time::sleep};
use utils::{spawn_dummy_repl, take_all_output};

#[googletest::test]
#[tokio::test]
async fn kernel_processes_a_message_succesfully() {
    let mut terminal = launch_terminal(10);
    let (request, io_receiver) = create_request_exec(1, "1");

    terminal.send(request).await;
    let response = terminal.recv().await.unwrap();

    expect_that!(take_all_output(io_receiver).await, is_utf8_string(eq("1")));
    expect_that!(response, pat!(KernelResponse::Success(pat!(1))));
}

#[googletest::test]
#[tokio::test]
async fn kernel_processes_multiple_messages_succesfully() {
    let mut terminal = launch_terminal(10);
    let (request1, io_receiver1) = create_request_exec(1, "1");
    let (request2, io_receiver2) = create_request_exec(2, "2");

    terminal.send(request1).await;
    terminal.send(request2).await;
    let response1 = terminal.recv().await.unwrap();
    let response2 = terminal.recv().await.unwrap();

    expect_that!(take_all_output(io_receiver1).await, is_utf8_string(eq("1")));
    expect_that!(take_all_output(io_receiver2).await, is_utf8_string(eq("2")));
    expect_that!(response1, pat!(KernelResponse::Success(pat!(1))));
    expect_that!(response2, pat!(KernelResponse::Success(pat!(2))));
}

#[googletest::test]
#[tokio::test]
async fn kernel_returns_an_error_when_interupted() {
    let mut terminal = launch_terminal(10);
    let (request, io_receiver) = create_request_exec(99, "expensive");

    terminal.send(request).await;

    // sleep is needed to wait the request being executed
    sleep(Duration::from_micros(10)).await;
    terminal.send(KernelRequest::Interrupt).await;

    let response = terminal.recv().await.unwrap();

    expect_that!(
        take_all_output(io_receiver).await,
        is_utf8_string(eq("partial..."))
    );
    expect_that!(response, pat!(KernelResponse::Cancelled(pat!(99))));
}

#[googletest::test]
#[tokio::test]
async fn kernel_drops_all_exec_message_in_queue_when_interupted() {
    let mut terminal = launch_terminal(10);
    let (request1, io_receiver1) = create_request_exec(99, "expensive");
    let (request2, io_receiver2) = create_request_exec(2, "2");
    let (request3, io_receiver3) = create_request_exec(3, "3");

    terminal.send(request1).await;
    terminal.send(request2).await;
    terminal.send(request3).await;

    // sleep is needed to wait the request being executed
    sleep(Duration::from_micros(50)).await;
    terminal.send(KernelRequest::Interrupt).await;

    let response1 = terminal.recv().await.unwrap();
    let response2 = terminal.recv().await.unwrap();
    let response3 = terminal.recv().await.unwrap();

    expect_that!(response1, pat!(KernelResponse::Cancelled(pat!(99))));
    expect_that!(response2, pat!(KernelResponse::Cancelled(pat!(2))));
    expect_that!(response3, pat!(KernelResponse::Cancelled(pat!(3))));

    expect_that!(
        take_all_output(io_receiver1).await,
        is_utf8_string(eq("partial..."))
    );
    expect_that!(
        take_all_output(io_receiver2).await,
        is_utf8_string(eq("")) // no byte sent
    );
    expect_that!(
        take_all_output(io_receiver3).await,
        is_utf8_string(eq("")) // no byte sent
    );
}

#[googletest::test]
#[tokio::test]
async fn kernel_returns_an_error_when_the_code_is_buggy() {
    let mut terminal = launch_terminal(10);
    let (request, io_receiver) = create_request_exec(99, "buggy");

    terminal.send(request).await;
    let response = terminal.recv().await.unwrap();

    expect_that!(response, pat!(KernelResponse::Failed(pat!(99))));
    expect_that!(
        take_all_output(io_receiver).await,
        is_utf8_string(eq("error"))
    );
}

#[googletest::test]
#[tokio::test]
async fn kernel_drops_all_exec_message_in_queue_when_the_code_is_buggy() {
    let mut terminal = launch_terminal(10);
    let (request1, io_receiver1) = create_request_exec(99, "buggy");
    let (request2, io_receiver2) = create_request_exec(2, "2");
    let (request3, io_receiver3) = create_request_exec(3, "3");

    terminal.send(request1).await;
    terminal.send(request2).await;
    terminal.send(request3).await;

    let response1 = terminal.recv().await.unwrap();
    let response2 = terminal.recv().await.unwrap();
    let response3 = terminal.recv().await.unwrap();

    expect_that!(response1, pat!(KernelResponse::Failed(pat!(99))));
    expect_that!(response2, pat!(KernelResponse::Cancelled(pat!(2))));
    expect_that!(response3, pat!(KernelResponse::Cancelled(pat!(3))));

    expect_that!(
        take_all_output(io_receiver1).await,
        is_utf8_string(eq("error"))
    );
    expect_that!(
        take_all_output(io_receiver2).await,
        is_utf8_string(eq("")) // no byte sent
    );
    expect_that!(
        take_all_output(io_receiver3).await,
        is_utf8_string(eq("")) // no byte sent
    );
}

fn launch_terminal(capacity: usize) -> KernelTerminal {
    let (terminal, _queue_semaphore) =
        kernel::launch(repl::launch::<MockRepl>(spawn_dummy_repl()), capacity);

    terminal
}

fn create_request_exec(
    message_id: u32,
    code: &str,
) -> (KernelRequest, mpsc::UnboundedReceiver<Bytes>) {
    let (io_sender, io_receiver) = mpsc::unbounded_channel();
    let message = KernelRequest::Execute {
        message_id,
        code: code.to_string(),
        io_sender,
    };

    (message, io_receiver)
}
