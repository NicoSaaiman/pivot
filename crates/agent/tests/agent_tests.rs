use std::sync::{Arc, RwLock};

use axum::body::{Body, to_bytes};
use axum::response::IntoResponse;
use axum::http::{Request, StatusCode};
use serde_json;
use tower::ServiceExt;

use shared::models::{
    NodeInfo, 
    NodeMetrics,
};

use pivot_agent::{
    handlers,
    state::{AppState, AgentState},
    routes,
    runtime::RuntimeManager,
};

#[tokio::test]
async fn get_node_info() {
    let response = handlers::get_node_info()
        .await
        .unwrap()
        .into_response();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let info: NodeInfo = serde_json::from_slice(&body).unwrap();

    assert!(!info.hostname.is_empty());
    assert!(!info.machine_id.is_empty());
    assert_eq!(info.address, format!("http://localhost:{}", info.port));
}

#[tokio::test]
async fn get_node_metrics() {
    let response = handlers::get_node_metrics()
        .await
        .unwrap()
        .into_response();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let metrics: NodeMetrics = serde_json::from_slice(&body).unwrap();

    assert!(metrics.cpu_usage.is_finite());
    assert!(metrics.uptime.is_finite());
    assert!(metrics.memory_usage.total_kb.is_finite());
    assert!(metrics.memory_usage.used_kb.is_finite());
    assert!(metrics.load_average.min_1.is_finite());
    assert!(metrics.load_average.min_5.is_finite());
    assert!(metrics.load_average.min_15.is_finite());
}

#[tokio::test]
async fn get_workload_returns_not_found_for_unknown_id() {
    let agent = Arc::new(RwLock::new(AgentState::new()));
    let runtime = Arc::new(RuntimeManager::build(agent.clone()));

    let state = AppState {
        agent,
        runtime,
    };

    let app = routes::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
            .uri("/workload/nonexistent-workload")
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
