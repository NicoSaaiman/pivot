use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use wiremock::{
    ResponseTemplate, 
    matchers::{method, path},
    Mock, 
    MockServer,
};

use pivot_controller::{
    routes,
    state::{
        ActualWorkload,
        ControllerState, 
        ScheduledWorkload, 
        WorkloadDetails,
        WorkloadStatus,
    }
};

use shared::models::{
    ExecutionState,
    ExecutionType,
    Node,
    NodeId,
    NodeInfo,
    NodeMetrics,
    MemoryUsage,
    LoadAverage,
    Process,
    Workload,
};

//---------execute_workload()----------

#[tokio::test]
async fn execute_workload_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/execute"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut controller_state = ControllerState::new();

    let info = NodeInfo {
        address: mock_server.uri(),
        hostname: "beep_boop".to_string(),
        machine_id: "12345".to_string(),
        node_id: "node1".to_string(),
        port: "3000".to_string(),
    };

    let memory_usage = MemoryUsage {
        used_kb: 0.0,
        total_kb: 0.0,
    };

    let load_average = LoadAverage {
        min_1: 0.0,
        min_5: 0.0,
        min_15: 0.0,
    };

    let metrics = NodeMetrics {
        cpu_usage: 0.0,
        memory_usage,
        load_average,
        uptime: 0.0,
    };

    let node = Node {
        id: "node1".to_string(),
        info,
        metrics,
        is_online: true,
    };
    controller_state.update_nodes(node);

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
        ExecutionType::Process(process)
    );

    let state = Arc::new(RwLock::new(controller_state));
    let app = routes::create_router(Arc::clone(&state));
    
    let response = app
        .oneshot(
            Request::builder()
            .method("POST")
            .header("Content-Type", "application/json")
            .uri("/execute")
            .body(Body::from(serde_json::to_vec(&workload).unwrap()))
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    
    let message: String = serde_json::from_slice(&body).unwrap();
    assert_eq!(message, "Workload scheduled");
    {
        let controller_state = state.read().unwrap();
        assert!(controller_state.desired_workloads.contains_key("test-workload"));

        let scheduled = controller_state
            .desired_workloads
            .get("test-workload")
            .unwrap();

        assert_eq!(scheduled.assigned_node.as_deref(), Some("node1"));
    }

    mock_server.verify().await;
}

//---------get_workload()------------

#[tokio::test]
async fn get_workload_success() {
    let mut controller_state = ControllerState::new();

    let info = NodeInfo {
        address: "http://localhost".to_string(),
        hostname: "beep_boop".to_string(),
        machine_id: "12345".to_string(),
        node_id: "node1".to_string(),
        port: "3000".to_string(),
    };

    let memory_usage = MemoryUsage {
        used_kb: 0.0,
        total_kb: 0.0,
    };

    let load_average = LoadAverage {
        min_1: 0.0,
        min_5: 0.0,
        min_15: 0.0,
    };

    let metrics = NodeMetrics {
        cpu_usage: 0.0,
        memory_usage,
        load_average,
        uptime: 0.0,
    };

    let node = Node {
        id: "node1".to_string(),
        info,
        metrics,
        is_online: true,
    };
    controller_state.update_nodes(node);

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
        ExecutionType::Process(process)
    );

    let workload_id = workload.get_workload_id();

    let scheduled_workload = ScheduledWorkload {
        workload,
        assigned_node: Some("node1".to_string()),
        scheduled_at: Instant::now(),
    };

    controller_state.update_desired_workloads(workload_id.clone(), scheduled_workload);

    let actual_workload = ActualWorkload {
        status: WorkloadStatus {
            state: ExecutionState::Running,
            last_updated: Instant::now(),
        },
        node: "node1".to_string(), 
    };

    controller_state.actual_workloads.insert("test-workload".to_string(), actual_workload);

    let state = Arc::new(RwLock::new(controller_state));
    let app = routes::create_router(Arc::clone(&state));
    
    let response = app
        .oneshot(
            Request::builder()
            .uri(format!("/workload/{}", workload_id.clone()))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    
    let workload_details: WorkloadDetails = serde_json::from_slice(&body).unwrap();
    assert_eq!(workload_details.id, workload_id);

}

// --------delete_workload()----------

#[tokio::test]
async fn delete_workload_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/workload/test-workload"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut controller_state = ControllerState::new();

    let info = NodeInfo {
        address: mock_server.uri(),
        hostname: "beep_boop".to_string(),
        machine_id: "12345".to_string(),
        node_id: "node1".to_string(),
        port: "3000".to_string(),
    };

    let memory_usage = MemoryUsage {
        used_kb: 0.0,
        total_kb: 0.0,
    };

    let load_average = LoadAverage {
        min_1: 0.0,
        min_5: 0.0,
        min_15: 0.0,
    };

    let metrics = NodeMetrics {
        cpu_usage: 0.0,
        memory_usage,
        load_average,
        uptime: 0.0,
    };

    let node = Node {
        id: "node1".to_string(),
        info,
        metrics,
        is_online: true,
    };
    controller_state.update_nodes(node);

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
        ExecutionType::Process(process)
    );

    let workload_id = workload.get_workload_id();

    let scheduled_workload = ScheduledWorkload {
        workload,
        assigned_node: Some("node1".to_string()),
        scheduled_at: Instant::now(),
    };

    controller_state.update_desired_workloads(workload_id.clone(), scheduled_workload);

    let state = Arc::new(RwLock::new(controller_state));
    let app = routes::create_router(Arc::clone(&state));
    
    let response = app
        .oneshot(
            Request::builder()
            .method("DELETE")
            .uri(format!("/workload/{}", workload_id))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    
    let message: String = serde_json::from_slice(&body).unwrap();
    assert_eq!(message, "Workload deleted");
    {
        let controller_state = state.read().unwrap();
        assert!(controller_state.desired_workloads.is_empty());
    }
    mock_server.verify().await;
}

//-----------update_nodes()------------

#[tokio::test]
async fn update_nodes_success() {
    let controller_state = ControllerState::new();

    let info = NodeInfo {
        address: "localhost".to_string(),
        hostname: "beep_boop".to_string(),
        machine_id: "12345".to_string(),
        node_id: "node1".to_string(),
        port: "3000".to_string(),
    };

    let memory_usage = MemoryUsage {
        used_kb: 0.0,
        total_kb: 0.0,
    };

    let load_average = LoadAverage {
        min_1: 0.0,
        min_5: 0.0,
        min_15: 0.0,
    };

    let metrics = NodeMetrics {
        cpu_usage: 0.0,
        memory_usage,
        load_average,
        uptime: 0.0,
    };

    let node = Node {
        id: "node1".to_string(),
        info,
        metrics,
        is_online: true,
    };

    let state = Arc::new(RwLock::new(controller_state));
    let app = routes::create_router(state);
    
    let response = app
        .oneshot(
            Request::builder()
            .method("POST")
            .uri("/nodes")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&node).unwrap()))
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    
    let message: String = serde_json::from_slice(&body).unwrap();

    assert_eq!(message, "Node state updated");
}

//-------------get_nodes()-------------

#[tokio::test]
async fn get_nodes_empty() {
    let controller_state = ControllerState::new();

    let state = Arc::new(RwLock::new(controller_state));
    let app = routes::create_router(state);
    
    let response = app
        .oneshot(
            Request::builder()
            .uri("/nodes")
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    
    let nodes: HashMap<NodeId, Node> = serde_json::from_slice(&body).unwrap();

    assert_eq!(nodes.len(), 0);
}

#[tokio::test]
async fn get_nodes_success() {
    let mut controller_state = ControllerState::new();

    let info = NodeInfo {
        address: "localhost".to_string(),
        hostname: "beep_boop".to_string(),
        machine_id: "12345".to_string(),
        node_id: "node1".to_string(),
        port: "3000".to_string(),
    };

    let memory_usage = MemoryUsage {
        used_kb: 0.0,
        total_kb: 0.0,
    };

    let load_average = LoadAverage {
        min_1: 0.0,
        min_5: 0.0,
        min_15: 0.0,
    };

    let metrics = NodeMetrics {
        cpu_usage: 0.0,
        memory_usage,
        load_average,
        uptime: 0.0,
    };

    let node = Node {
        id: "node1".to_string(),
        info,
        metrics,
        is_online: true,
    };
    controller_state.update_nodes(node);
    let state = Arc::new(RwLock::new(controller_state));
    let app = routes::create_router(state);
    
    let response = app
        .oneshot(
            Request::builder()
            .uri("/nodes")
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    
    let nodes: HashMap<NodeId, Node> = serde_json::from_slice(&body).unwrap();

    assert_eq!(nodes.len(), 1);
    assert!(nodes.contains_key("node1"));
    assert_eq!(nodes.get("node1").unwrap().info.hostname, "beep_boop");
}
