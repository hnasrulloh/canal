mod mock_repl;
mod utils;

use std::time::Duration;

use bytes::Bytes;
use canal_kernel::{
    kernel::{self, KernelHandle},
    repl, Message, MessageError,
};
use googletest::prelude::*;
use mock_repl::MockRepl;
use tokio::{sync::mpsc, time::sleep};
use utils::{spawn_dummy_repl, take_all_output};

#[googletest::test]
#[tokio::test]
async fn kernel_processes_a_message_succesfully() {
    let mut handle = create_kernel(10);
    let (message, io_receiver) = create_message(1, "1");

    let queue_result = handle.send(message).await;
    let exec_result = handle.recv().await;

    expect_that!(take_all_output(io_receiver).await, is_utf8_string(eq("1")));
    expect_that!(queue_result, pat!(Ok(_)));
    expect_that!(exec_result, pat!(Some(pat!(Ok(_)))));
}

#[googletest::test]
#[tokio::test]
async fn kernel_processes_multiple_messages_succesfully() {
    let mut handle = create_kernel(10);
    let (message1, io_receiver1) = create_message(1, "1");
    let (message2, io_receiver2) = create_message(2, "2");

    let queue_result1 = handle.send(message1).await;
    let queue_result2 = handle.send(message2).await;
    let exec_result1 = handle.recv().await;
    let exec_result2 = handle.recv().await;

    expect_that!(take_all_output(io_receiver1).await, is_utf8_string(eq("1")));
    expect_that!(take_all_output(io_receiver2).await, is_utf8_string(eq("2")));
    expect_that!(queue_result1, pat!(Ok(_)));
    expect_that!(queue_result2, pat!(Ok(_)));
    expect_that!(exec_result1, pat!(Some(pat!(Ok(_)))));
    expect_that!(exec_result2, pat!(Some(pat!(Ok(_)))));
}

#[googletest::test]
#[tokio::test]
async fn kernel_returns_an_error_when_interupted() {
    let mut handle = create_kernel(10);
    let (msg_exec, io_receiver) = create_message(99, "expensive");

    let queue_result = handle.send(msg_exec).await;
    sleep(Duration::from_micros(10)).await; // sleep is needed to avoid polling race
    handle.send(Message::Interrupt).await.unwrap();
    let exec_result = handle.recv().await;

    expect_that!(queue_result, pat!(Ok(_)));
    expect_that!(
        exec_result,
        pat!(Some(pat!(Err(pat!(MessageError::Cancelled(pat!(99)))))))
    );
    expect_that!(
        take_all_output(io_receiver).await,
        is_utf8_string(eq("partial..."))
    );
}

// TODO: Fix message processing race (working message executed too early before interrupted/failed message sent to channel)

#[googletest::test]
#[tokio::test]
async fn kernel_drops_all_exec_message_in_queue_when_interupted() {
    let mut handle = create_kernel(10);
    let (msg_exec1, io_receiver1) = create_message(99, "expensive");
    let (msg_exec2, io_receiver2) = create_message(2, "2");
    let (msg_exec3, io_receiver3) = create_message(3, "3");

    let queue_result1 = handle.send(msg_exec1).await;
    let queue_result2 = handle.send(msg_exec2).await;
    let queue_result3 = handle.send(msg_exec3).await;

    sleep(Duration::from_micros(50)).await; // needed to emulates IPC latency
    handle.send(Message::Interrupt).await.unwrap();

    let exec_result1 = handle.recv().await;
    let exec_result2 = handle.recv().await;
    let exec_result3 = handle.recv().await;

    expect_that!(queue_result1, pat!(Ok(_)));
    expect_that!(queue_result2, pat!(Ok(_)));
    expect_that!(queue_result3, pat!(Ok(_)));

    expect_that!(
        exec_result1,
        pat!(Some(pat!(Err(pat!(MessageError::Cancelled(pat!(99)))))))
    );
    expect_that!(
        exec_result2,
        pat!(Some(pat!(Err(pat!(MessageError::Cancelled(pat!(2)))))))
    );
    expect_that!(
        exec_result3,
        pat!(Some(pat!(Err(pat!(MessageError::Cancelled(pat!(3)))))))
    );

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
    let (msg_exec, io_receiver) = create_message(99, "buggy");

    let queue_result = handle.send(msg_exec).await;
    let exec_result = handle.recv().await;

    expect_that!(queue_result, pat!(Ok(_)));
    expect_that!(
        exec_result,
        pat!(Some(pat!(Err(pat!(MessageError::Failed(pat!(99)))))))
    );
    expect_that!(
        take_all_output(io_receiver).await,
        is_utf8_string(eq("error"))
    );
}

#[googletest::test]
#[tokio::test]
async fn kernel_drops_all_exec_message_in_queue_when_the_code_is_buggy() {
    let mut handle = create_kernel(10);
    let (msg_exec1, io_receiver1) = create_message(99, "buggy");
    let (msg_exec2, io_receiver2) = create_message(2, "2");
    let (msg_exec3, io_receiver3) = create_message(3, "3");

    let queue_result1 = handle.send(msg_exec1).await;
    let queue_result2 = handle.send(msg_exec2).await;
    let queue_result3 = handle.send(msg_exec3).await;

    let exec_result1 = handle.recv().await;
    let exec_result2 = handle.recv().await;
    let exec_result3 = handle.recv().await;

    expect_that!(queue_result1, pat!(Ok(_)));
    expect_that!(queue_result2, pat!(Ok(_)));
    expect_that!(queue_result3, pat!(Ok(_)));

    expect_that!(
        exec_result1,
        pat!(Some(pat!(Err(pat!(MessageError::Failed(pat!(99)))))))
    );
    expect_that!(
        exec_result2,
        pat!(Some(pat!(Err(pat!(MessageError::Cancelled(pat!(2)))))))
    );
    expect_that!(
        exec_result3,
        pat!(Some(pat!(Err(pat!(MessageError::Cancelled(pat!(3)))))))
    );

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
    kernel::launch(repl::launch::<MockRepl>(spawn_dummy_repl()), capacity)
}

fn create_message(message_id: u32, code: &str) -> (Message, mpsc::UnboundedReceiver<Bytes>) {
    let (io_sender, io_receiver) = mpsc::unbounded_channel();
    let message = Message::Execute {
        message_id,
        code: code.to_string(),
        io_sender,
    };

    (message, io_receiver)
}
