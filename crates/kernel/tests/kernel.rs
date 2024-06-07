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
async fn kernel_can_be_interupted() {
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

// #[googletest::test]
// #[tokio::test]
// async fn kernel_drops_all_exec_message_in_queue_when_interupted() {
//     // TODO: Fix this

//     let (kernel, message_sender) = create_kernel(10);

//     task::spawn(async move { run(kernel).await });

//     let (message1, io_receiver1) = create_message_execute("expensive");
//     message_sender.send(message1).await.unwrap();

//     let (message2, io_receiver2) = create_message_execute("2");
//     message_sender.send(message2).await.unwrap();

//     let (message3, io_receiver3) = create_message_execute("3");
//     message_sender.send(message3).await.unwrap();

//     // A slight waiting needed to avoid message processing race
//     sleep(Duration::from_micros(10)).await;
//     message_sender.send(Message::Interupt).await.unwrap();

//     expect_that!(
//         take_all_output(io_receiver1).await,
//         is_utf8_string(eq("partial..."))
//     );

//     // No output for cancelled execs
//     expect_that!(take_all_output(io_receiver2).await, is_utf8_string(eq("")));
//     expect_that!(take_all_output(io_receiver3).await, is_utf8_string(eq("")));
// }

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
