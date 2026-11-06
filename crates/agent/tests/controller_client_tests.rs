use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

use pivot_agent::controller_client::{
    ControllerClient, 
    HttpControllerClient
};

use test_utils;

#[tokio::test]
async fn register_node_success() {
    let mock_server = test_utils::get_mock_server("POST", "/nodes", ResponseTemplate::new(200)).await;

    let controller_client = HttpControllerClient {
        client: reqwest::Client::new(),
        controller_url: mock_server.uri(),
    };

    let result = controller_client.register_node().await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn register_node_controller_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/nodes"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&mock_server)
        .await;

    let controller_client = HttpControllerClient {
        client: reqwest::Client::new(),
        controller_url: mock_server.uri(),
    };

    let result = controller_client.register_node().await;

    assert!(result.is_err());
}

#[tokio::test]
async fn register_node_rejected() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/nodes"))
        .respond_with(ResponseTemplate::new(400))
        .expect(1)
        .mount(&mock_server)
        .await;

    let controller_client = HttpControllerClient {
        client: reqwest::Client::new(),
        controller_url: mock_server.uri(),
    };

    let result = controller_client.register_node().await;

    assert!(result.is_err());
   
}

#[tokio::test]
async fn register_node_controller_unavailable() {
    let controller_client = HttpControllerClient {
        client: reqwest::Client::new(),
        controller_url: "http://127.0.0.1:1".to_string(),
    };   

    let result = controller_client.register_node().await;

    assert!(result.is_err());
}
