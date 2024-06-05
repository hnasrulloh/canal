mod mock_repl;
mod utils;

use canal_kernel::{message::Message, message_queue::MessageQueue, repl, Kernel};
use googletest::prelude::*;
use mock_repl::MockRepl;
use tokio::{sync::mpsc, task};
use utils::spawn_dummy_repl;

#[googletest::test]
#[tokio::test]
async fn kernel_processes_a_message_succesfully() {
    let (io_sender, mut io_receiver) = mpsc::unbounded_channel();
    let message = Message::Execute {
        code: "1".into(),
        io_sender,
    };

    let (message_sender, message_receiver) = mpsc::channel(8);

    let mut kernel = Kernel {
        repl: repl::using::<MockRepl>(spawn_dummy_repl()),
        message_queue: MessageQueue::with_capacity(10),
        message_source: message_receiver,
    };

    task::spawn(async move { kernel.run().await });

    message_sender.send(message).await.unwrap();

    let output = io_receiver.recv().await.expect("Output is empty");
    expect_that!(output, is_utf8_string(eq("1")));
}
