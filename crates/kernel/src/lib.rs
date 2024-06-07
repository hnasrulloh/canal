mod kernel;
pub mod message;
pub mod repl;

pub use kernel::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Execution failed")]
    Failed,
    #[error("Execution was interrupted")]
    Interrupted,
}

#[derive(Error, Debug)]
pub enum QueueError {
    #[error("Maximum message capacity of the queue was exceeded")]
    MessageOverload,
}
