use std::collections::BTreeMap;
use std::process;
use std::sync::{Arc, RwLock};

use axum::body::{Body, to_bytes};
use axum::response::IntoResponse;
use axum::http::{Request, StatusCode};
use axum::routing::Router;
use serde_json;
use tower::ServiceExt;

use shared::models::{
    CustomContainer,
    ExecutionState,
    ExecutionType,
    NodeInfo, 
    NodeMetrics,
    Process,
    Workload,
    WorkloadStatus,
};

use pivot_agent::{
    handlers,
    state::{AppState, AgentState, RuntimeHandle},
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
    let (_, app) = setup_test_router(AgentState::new());

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

#[tokio::test]
async fn process_get_workload_running() {
    let mut agent_state = AgentState::new();
    
    let child = process::Command::new("sleep")
        .arg("30")
        .spawn()
        .unwrap();

    agent_state.workloads.insert(
        "test-workload".to_string(),
        RuntimeHandle::Process(child),
    );

    let (state, app) = setup_test_router(agent_state);

    let response = app
        .oneshot(
            Request::builder()
            .uri("/workload/test-workload")
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let workload_status: WorkloadStatus = serde_json::from_slice(&body).unwrap();

    assert_eq!(workload_status.state, ExecutionState::Running);
    
    {
        let mut agent_state = state.agent.write().unwrap();

        if let Some(RuntimeHandle::Process(child)) =
            agent_state.workloads.get_mut("test-workload")
        {
            child.kill().unwrap();
            child.wait().unwrap();
        }
    }  
}

#[tokio::test]
async fn docker_get_workload_running() {
    let (state, app) = setup_test_router(AgentState::new());

    let container_id = state
        .runtime
        .docker_create_container(
            "pivot-test-running",
            Some("busybox:latest".to_string()),
            Some(vec!["sleep".to_string(), "30".to_string()]),
        )
        .await
        .unwrap();

    state
        .runtime
        .docker_start_container(&container_id)
        .await
        .unwrap();

    state
        .agent
        .write()
        .unwrap()
        .workloads
        .insert(
            "test-workload".to_string(),
            RuntimeHandle::Docker { 
                container_id: container_id.clone(),
            },
        );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/workload/test-workload")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let workload_status: WorkloadStatus = serde_json::from_slice(&body).unwrap();

    assert_eq!(workload_status.state, ExecutionState::Running);

    state
        .runtime
        .docker_delete_container(&container_id)
        .await
        .unwrap();
}

#[tokio::test]
async fn custom_container_single_get_workload_running() {
     let (state, app) = setup_test_router(AgentState::new());

    let process = Process::new(
        "sleep".to_string(),
        vec!["30".to_string()],
        BTreeMap::new(),
        None,
    );

    let workload = Workload::new(
        "test-workload".to_string(),
        "default".to_string(),
        ExecutionState::Running,
        ExecutionType::CustomContainer(CustomContainer {
            processes: vec![process]
        }),
    );

    state.runtime.execute_workload(workload).await.unwrap();

    let response = app
        .oneshot(
            Request::builder()
            .uri("/workload/test-workload")
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let workload_status: WorkloadStatus = serde_json::from_slice(&body).unwrap();

    assert_eq!(workload_status.state, ExecutionState::Running);

    let pids = {
        let agent = state.agent.read().unwrap();

        match agent.workloads.get("test-workload").unwrap() {
            RuntimeHandle::Custom { processes } => {
                processes.iter().map(|p| p.pid).collect::<Vec<_>>()
            }
            _ => panic!("Expected custom runtime handle"),
        }
    };

    for pid in pids {
        state.runtime.custom_terminate_process(pid).unwrap();
        state.runtime.custom_wait_process_blocking(pid).unwrap();
    }
}

//--------Helper Functions----------

fn setup_test_router(
    agent_state: AgentState
) -> (AppState, Router) {
    let agent = Arc::new(RwLock::new(agent_state));
    let runtime = Arc::new(RuntimeManager::build(agent.clone()));

    let state = AppState {
        agent: agent.clone(),
        runtime,
    };

    (state.clone(), routes::create_router(state))
}
