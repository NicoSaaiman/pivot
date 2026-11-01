use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AgentError>;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Bollard error: {0}")]
    BollardError(#[from] bollard::errors::Error),
    #[error("Docker not available")]
    DockerNotAvailable,
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Missing CPU field")]
    MissingCpuField,
    #[error("Missing load average field")]
    MissingLoadAverageField,
    #[error("Missing memory field")]
    MissingMemoryField,
    #[error("Missing uptime field")]
    MissingUptimeField,
    #[error("Invalid C string: {0}")]
    ParseFloat(#[from] std::num::ParseFloatError),
    #[error("Agent state lock was poisoned")]
    PoisonError,
    #[error("Http request failed")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Runtime error: {0}")]
    RuntimeError(#[from] runtime::errors::RuntimeError),
    #[error("Unknown execution state")]
    UnknownExecutionState,
    #[error("Unknown execution status")]
    UnknownExecutionStatus,
    #[error("Workload not found: {0}")]
    WorkloadNotFound(String),
}

impl IntoResponse for AgentError {
    fn into_response(self) -> Response {
        let message = self.to_string();

        (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
    }
}
