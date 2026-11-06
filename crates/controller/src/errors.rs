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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn controller_error_into_response() {
        let error = ControllerError::NoAvailableNodes;

        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        assert_eq!(String::from_utf8(body.to_vec()).unwrap(), "No available nodes found");
    }
}
