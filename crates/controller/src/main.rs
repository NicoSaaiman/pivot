use std::{
    error::Error,
    sync::{Arc, RwLock}
};

use tokio::net::TcpListener;

use pivot_controller::{node_monitor, reconciler, routes, state::ControllerState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let shared_state = Arc::new(RwLock::new(ControllerState::new()));

    tokio::spawn(node_monitor::poll_nodes(shared_state.clone()));
    tokio::spawn(reconciler::reconcile_workloads(shared_state.clone()));

    let router = routes::create_router(shared_state);

    let listener = TcpListener::bind("0.0.0.0:3000")
        .await?;

    axum::serve(listener, router).await?;

    Ok(())
}
