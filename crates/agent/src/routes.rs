use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::handlers;
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/execute", post(handlers::execute_workload))
        .route("/workloads", get(handlers::get_workloads))
        .route("/workload/{id}", get(handlers::get_workload))
        .route("/workload/{id}", delete(handlers::delete_workload))
        .route("/info", get(handlers::get_node_info))
        .route("/metrics", get(handlers::get_node_metrics))
        .with_state(state)
}
