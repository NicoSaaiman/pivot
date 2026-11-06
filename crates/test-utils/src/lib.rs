use std::collections::BTreeMap;

use wiremock::{
    matchers::{method, path},
    Mock,
    MockServer,
    ResponseTemplate,
};

use shared::models::{
    ExecutionState,
    ExecutionType,
    LoadAverage,
    MemoryUsage,
    Node,
    NodeId,
    NodeInfo,
    NodeMetrics,
    Process,
    Workload,
};

pub fn create_test_workload() -> Workload {
    let process = Process::new(
        "sleep".to_string(),
        vec!["30".to_string()],
        BTreeMap::new(),
        None,
    );

    Workload::new(
        "test-workload".to_string(),
        "default".to_string(),
        ExecutionState::Running,
        ExecutionType::Process(process)
    )
}

pub fn create_test_node(id: NodeId, address: Option<String>) -> Node {
    Node {
        id: id.clone(),
        info: NodeInfo {
            address: address.unwrap_or_else(|| "http://localhost".to_string()),
            hostname: "beep_boop".to_string(),
            machine_id: "12345".to_string(),
            node_id: id,
            port: "3000".to_string(),
        },
        metrics: NodeMetrics {
            cpu_usage: 0.0,
            memory_usage: MemoryUsage {
                used_kb: 0.0,
                total_kb: 0.0,
            },
            load_average: LoadAverage {
                min_1: 0.0,
                min_5: 0.0,
                min_15: 0.0,
            },
            uptime: 0.0,
        },
        is_online: true,
    }
}

pub async fn get_mock_server(
    method_str: &str, 
    path_str: &str,
    response: ResponseTemplate,
) -> MockServer {
    let mock_server = MockServer::start().await;

    Mock::given(method(method_str))
        .and(path(path_str))
        .respond_with(response)
        .expect(1)
        .mount(&mock_server)
        .await;

    mock_server
}
