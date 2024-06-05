mod mock_repl;
mod utils;

use bytes::Bytes;
use canal_kernel::{message::Message, repl, Kernel};
use googletest::prelude::*;
use mock_repl::MockRepl;
use tokio::{sync::mpsc, task};
use utils::spawn_dummy_repl;

#[googletest::test]
#[tokio::test]
async fn kernel_processes_a_message_succesfully() {
    let (mut kernel, message_sender) = create_kernel(10);
    let (message, mut io_receiver) = create_message_execute("1");

    task::spawn(async move { kernel.run().await });
    message_sender.send(message).await.unwrap();

    let output = io_receiver.recv().await.expect("Output is empty");
    expect_that!(output, is_utf8_string(eq("1")));
}

#[googletest::test]
#[tokio::test]
async fn kernel_processes_multiple_messages_succesfully() {
    let (mut kernel, message_sender) = create_kernel(10);
    let (message1, mut io_receiver1) = create_message_execute("1");
    let (message2, mut io_receiver2) = create_message_execute("2");

    task::spawn(async move { kernel.run().await });
    message_sender.send(message1).await.unwrap();
    message_sender.send(message2).await.unwrap();

    expect_that!(
        io_receiver1.recv().await.expect("Output is empty"),
        is_utf8_string(eq("1"))
    );

    expect_that!(
        io_receiver2.recv().await.expect("Output is empty"),
        is_utf8_string(eq("2"))
    );
}

fn create_kernel(maximum_message_capacity: usize) -> (Kernel, mpsc::Sender<Message>) {
    let (message_sender, message_receiver) = mpsc::channel(8);
    let repl = repl::using::<MockRepl>(spawn_dummy_repl());
    let kernel = Kernel::new(repl, message_receiver, maximum_message_capacity);

    (kernel, message_sender)
}

fn create_message_execute(code: &str) -> (Message, mpsc::UnboundedReceiver<Bytes>) {
    let (io_sender, io_receiver) = mpsc::unbounded_channel();

    let message = Message::Execute {
        code: code.to_string(),
        io_sender,
    };

    (message, io_receiver)
}
