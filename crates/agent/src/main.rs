use std::{
    env,
    error::Error,
    sync::{Arc, RwLock,}
};

use tokio::net::TcpListener;

use pivot_agent::{
    routes, 
    runtime::RuntimeManager,
    state::{AgentState, AppState},
};

use pivot_agent::controller_client::ControllerClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let port = env::args()
        .nth(2)
        .unwrap_or("3000".to_string());

    let agent = Arc::new(RwLock::new(AgentState::new()));

    let shared_state = AppState { 
        agent: agent.clone(),
        runtime: Arc::new(RuntimeManager::build(agent)?),
    };

    let client = {
        let state = shared_state
            .agent
            .read()
            .expect("Agent state lock poisoned during startup");

        state
            .get_http_controller_client()
            .clone()
    };

    let router = routes::create_router(shared_state);

    let url = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&url)
        .await?;

    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
    });

    client.register_node().await?;

    server.await??;

    Ok(())
}
