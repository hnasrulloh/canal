mod mock_repl;
mod utils;

use std::time::Duration;

use bytes::Bytes;
use canal_kernel::{kernel, repl, Message};
use googletest::prelude::*;
use mock_repl::MockRepl;
use tokio::{sync::mpsc, task, time::sleep};
use utils::{spawn_dummy_repl, take_all_output};

#[googletest::test]
#[tokio::test]
async fn kernel_processes_a_message_succesfully() {
    let mut handle = kernel::launch(repl::launch::<MockRepl>(spawn_dummy_repl()), 10);

    let (io_sender, io_receiver) = mpsc::unbounded_channel();

    let message = Message::Execute {
        message_id: 1,
        code: "1".to_string(),
        io_sender,
    };

    let queue_result = handle.send(message).await;
    let exec_result = handle.recv().await;

    expect_that!(take_all_output(io_receiver).await, is_utf8_string(eq("1")));
    expect_that!(queue_result, pat!(Ok(_)));
    expect_that!(exec_result, pat!(Some(pat!(Ok(_)))));
}

// #[googletest::test]
// #[tokio::test]
// async fn kernel_processes_multiple_messages_succesfully() {
//     let (kernel, message_sender) = create_kernel(10);
//     let (message1, io_receiver1) = create_message_execute("1");
//     let (message2, io_receiver2) = create_message_execute("2");

//     task::spawn(async move { run(kernel).await });
//     message_sender.send(message1).await.unwrap();
//     message_sender.send(message2).await.unwrap();

//     expect_that!(take_all_output(io_receiver1).await, is_utf8_string(eq("1")));
//     expect_that!(take_all_output(io_receiver2).await, is_utf8_string(eq("2")));
// }

// #[googletest::test]
// #[tokio::test]
// async fn kernel_can_be_interupted() {
//     let (kernel, message_sender) = create_kernel(10);
//     let (exec_message, io_receiver) = create_message_execute("expensive");

//     task::spawn(async move { run(kernel).await });
//     message_sender.send(exec_message).await.unwrap();

//     // A slight waiting needed to avoid message processing race
//     sleep(Duration::from_micros(10)).await;
//     message_sender.send(Message::Interupt).await.unwrap();

//     expect_that!(
//         take_all_output(io_receiver).await,
//         is_utf8_string(eq("partial..."))
//     );
// }

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
