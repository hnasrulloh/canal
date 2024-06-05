#![allow(dead_code)]

use std::process::{self, Command};

use bytes::{BufMut, Bytes, BytesMut};
use tokio::sync::mpsc;

pub fn spawn_dummy_repl() -> process::Child {
    Command::new(env!("CARGO_BIN_EXE_dummy_repl"))
        .spawn()
        .unwrap()
}

pub async fn take_all_output(mut source: mpsc::UnboundedReceiver<Bytes>) -> BytesMut {
    let mut buffer = BytesMut::new();
    while let Some(b) = source.recv().await {
        buffer.put(b);
    }

    buffer
}
