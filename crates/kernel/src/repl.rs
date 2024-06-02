use std::panic::{self, UnwindSafe};
use thiserror::Error;

#[derive(Debug)]
pub struct Repl {
    pid: u32,
}

impl Repl {
    pub async fn launch<F>(spawn_repl: F) -> Result<Self, ReplError>
    where
        F: Fn() -> u32 + 'static + UnwindSafe,
    {
        let catch_unwind = panic::catch_unwind(spawn_repl);
        match catch_unwind {
            Ok(pid) => Ok(Self { pid }),
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

    #[googletest::test]
    #[tokio::test]
    async fn starting_a_repl_returns_ok() {
        let result = Repl::launch(try_spawn_repl_then_success).await;
        expect_that!(result, pat!(Ok(_)));
    }

    fn try_spawn_repl_then_success() -> u32 {
        let pid = 123;
        pid
    }

    #[googletest::test]
    #[tokio::test]
    async fn starting_a_repl_returns_error() {
        let result = Repl::launch(try_spawn_repl_then_fail).await;
        expect_that!(result, pat!(Err(pat!(ReplError::SpawnFailed))));
    }

    fn try_spawn_repl_then_fail() -> u32 {
        panic!("Failed to run a new REPL process");
    }
}
