use std::process::{self, Command};

pub fn spawn_dummy_repl() -> process::Child {
    Command::new(env!("CARGO_BIN_EXE_dummy_repl"))
        .spawn()
        .unwrap()
}
