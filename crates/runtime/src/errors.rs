use thiserror::Error;

pub type Result<T> = std::result::Result<T, RuntimeError>;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Failed to create process: fork failed")]
    ForkFailed,
    #[error("Invalid C string: {0}")]
    NulError(#[from] std::ffi::NulError),
    #[error("Failed to retrieve process state")]
    ProcessStateFailed,
    #[error("Failed to terminate process")]
    TerminationFailed,
    #[error("Wait operation failed")]
    WaitFailed,
}
