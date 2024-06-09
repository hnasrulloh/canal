mod mock_repl;
mod utils;

use std::time::Duration;

use bytes::Bytes;
use canal_kernel::{
    kernel::{self, KernelHandle},
    repl, Request, Response,
};
use googletest::prelude::*;
use mock_repl::MockRepl;
use tokio::{sync::mpsc, time::sleep};
use utils::{spawn_dummy_repl, take_all_output};

#[googletest::test]
#[tokio::test]
async fn kernel_processes_a_message_succesfully() {
    let mut handle = create_kernel(10);
    let (request, io_receiver) = create_request_exec(1, "1");

    handle.send(request).await;
    let response = handle.recv().await.unwrap();

    expect_that!(take_all_output(io_receiver).await, is_utf8_string(eq("1")));
    expect_that!(response, pat!(Response::Success(pat!(1))));
}

#[googletest::test]
#[tokio::test]
async fn kernel_processes_multiple_messages_succesfully() {
    let mut handle = create_kernel(10);
    let (request1, io_receiver1) = create_request_exec(1, "1");
    let (request2, io_receiver2) = create_request_exec(2, "2");

    handle.send(request1).await;
    handle.send(request2).await;
    let response1 = handle.recv().await.unwrap();
    let response2 = handle.recv().await.unwrap();

    expect_that!(take_all_output(io_receiver1).await, is_utf8_string(eq("1")));
    expect_that!(take_all_output(io_receiver2).await, is_utf8_string(eq("2")));
    expect_that!(response1, pat!(Response::Success(pat!(1))));
    expect_that!(response2, pat!(Response::Success(pat!(2))));
}

#[googletest::test]
#[tokio::test]
async fn kernel_returns_an_error_when_interupted() {
    let mut handle = create_kernel(10);
    let (request, io_receiver) = create_request_exec(99, "expensive");

    handle.send(request).await;

    // sleep is needed to wait the request being executed
    sleep(Duration::from_micros(10)).await;
    handle.send(Request::Interrupt).await;

    let response = handle.recv().await.unwrap();

    expect_that!(
        take_all_output(io_receiver).await,
        is_utf8_string(eq("partial..."))
    );
    expect_that!(response, pat!(Response::Cancelled(pat!(99))));
}

#[googletest::test]
#[tokio::test]
async fn kernel_drops_all_exec_message_in_queue_when_interupted() {
    let mut handle = create_kernel(10);
    let (request1, io_receiver1) = create_request_exec(99, "expensive");
    let (request2, io_receiver2) = create_request_exec(2, "2");
    let (request3, io_receiver3) = create_request_exec(3, "3");

    handle.send(request1).await;
    handle.send(request2).await;
    handle.send(request3).await;

    // sleep is needed to wait the request being executed
    sleep(Duration::from_micros(50)).await;
    handle.send(Request::Interrupt).await;

    let response1 = handle.recv().await.unwrap();
    let response2 = handle.recv().await.unwrap();
    let response3 = handle.recv().await.unwrap();

    expect_that!(response1, pat!(Response::Cancelled(pat!(99))));
    expect_that!(response2, pat!(Response::Cancelled(pat!(2))));
    expect_that!(response3, pat!(Response::Cancelled(pat!(3))));

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
    let mut handle = create_kernel(10);
    let (request, io_receiver) = create_request_exec(99, "buggy");

    handle.send(request).await;
    let response = handle.recv().await.unwrap();

    expect_that!(response, pat!(Response::Failed(pat!(99))));
    expect_that!(
        take_all_output(io_receiver).await,
        is_utf8_string(eq("error"))
    );
}

#[googletest::test]
#[tokio::test]
async fn kernel_drops_all_exec_message_in_queue_when_the_code_is_buggy() {
    let mut handle = create_kernel(10);
    let (request1, io_receiver1) = create_request_exec(99, "buggy");
    let (request2, io_receiver2) = create_request_exec(2, "2");
    let (request3, io_receiver3) = create_request_exec(3, "3");

    handle.send(request1).await;
    handle.send(request2).await;
    handle.send(request3).await;

    let response1 = handle.recv().await.unwrap();
    let response2 = handle.recv().await.unwrap();
    let response3 = handle.recv().await.unwrap();

    expect_that!(response1, pat!(Response::Failed(pat!(99))));
    expect_that!(response2, pat!(Response::Cancelled(pat!(2))));
    expect_that!(response3, pat!(Response::Cancelled(pat!(3))));

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

fn create_kernel(capacity: usize) -> KernelHandle {
    let (handle, _queue_semaphore) =
        kernel::launch(repl::launch::<MockRepl>(spawn_dummy_repl()), capacity);

    handle
}

fn create_request_exec(message_id: u32, code: &str) -> (Request, mpsc::UnboundedReceiver<Bytes>) {
    let (io_sender, io_receiver) = mpsc::unbounded_channel();
    let message = Request::Execute {
        message_id,
        code: code.to_string(),
        io_sender,
    };

    (message, io_receiver)
}
