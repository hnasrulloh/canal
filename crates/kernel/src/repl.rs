use std::{
    panic::{self, UnwindSafe},
    process,
};
use thiserror::Error;

#[derive(Debug)]
pub struct Repl {
    process: process::Child,
}

impl Repl {
    pub fn launch<F>(spawn_repl: F) -> Result<Self, ReplError>
    where
        F: Fn() -> process::Child + 'static + UnwindSafe,
    {
        let catch_unwind = panic::catch_unwind(spawn_repl);
        match catch_unwind {
            Ok(child) => Ok(Self { process: child }),
            Err(_) => Err(ReplError::SpawnFailed),
        }
    }
}

#[derive(Error, Debug)]
pub enum ReplError {
    #[error("")]
    SpawnFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;
}
