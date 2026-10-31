use axum::{
    Router,
    routing::{delete, get, post},
};

use std::sync::{Arc, RwLock};

use crate::{handlers, state::ControllerState};

pub fn create_router(state: Arc<RwLock<ControllerState>>) -> Router {
    Router::new()
        .route("/execute", post(handlers::execute_workload))
        .route("/workload/{id}", get(handlers::get_workload))
        .route("/workload/{id}", delete(handlers::delete_workload))
        .route("/nodes", post(handlers::update_nodes))
        .route("/nodes", get(handlers::get_nodes))
        .with_state(state)
}
