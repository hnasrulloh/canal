use tokio::sync::mpsc;
use thiserror::Error;
use tokio::task;

struct Repl {
    config: ReplConfig,
}

impl Repl {
    fn with(config: ReplConfig) -> Self {
        Self { config }
    }

    async fn launch(&self) -> Result<(), ReplError> {
        let (tx, mut rx) = mpsc::channel(1);

        task::spawn_blocking(|| {
            self.config.spawner(tx);
        });

        rx.blocking_recv().unwrap_or_else(|| Err(ReplError::SpawningFailed))
    }
}

struct ReplConfig {
    spawner: &'static spawn_repl,
}

#[derive(Error, Debug)]
pub enum ReplError {
    #[error("")]
    SpawningFailed,
}

pub type spawn_repl = dyn FnOnce(mpsc::Sender<Result<(), ReplError>>);

#[cfg(test)]
mod tests {
    use googletest::prelude::*;
    use crate::repl::{Repl, ReplConfig};

    #[googletest::test]
    #[tokio::test]
    async fn spawning_interpreter_returns_a_result() {
        let repl = Repl::with(ReplConfig {
            spawner: || { Ok(()) },
            ..default_repl_config()
        });
    }

    fn default_repl_config() -> ReplConfig {
        ReplConfig {
            spawner: || { Ok(()) },
        }
    }
}