use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ControllerError>;

#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("No available nodes found")]
    NoAvailableNodes,
    #[error("Node not found")]
    NodeNotFound,
    #[error("Controller state lock was poisoned")]
    PoisonError,
    #[error("Http request failed")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Workload not found")]
    WorkloadNotFound,
}

impl IntoResponse for ControllerError {
    fn into_response(self) -> Response {
        let message = self.to_string();

        (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
    }
}
