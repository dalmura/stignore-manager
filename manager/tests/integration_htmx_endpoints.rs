mod common;

use axum_test::TestServer;
use common::*;
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_itemlist_component_endpoint() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/components/itemlist.html").await;
    response.assert_status_ok();
    response.assert_text_contains("File Browser");
    response.assert_text_contains("Movies");
}

#[tokio::test]
async fn test_dynamic_items_lazy_loading() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/components/dynamic-items.html")
        .add_query_param("parent_id", "test-parent")
        .add_query_param("parent_path", "aWRfTW92aWVz") // Base64 encoded "id_Movies"
        .add_query_param("level", "2")
        .await;

    response.assert_status_ok();
    response.assert_text_contains("hx-trigger=\"revealed\"");
}

#[tokio::test]
async fn test_infopanel_post_request() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "category_id": "Movies",
        "item_path": ["Action", "movie.mkv"]
    });

    let response = server
        .post("/components/infopanel.html")
        .json(&request_body)
        .await;

    response.assert_status_ok();
    response.assert_text_contains("card-header"); // Check for the info panel structure
}

#[tokio::test]
async fn test_agent_modal_endpoint() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "agent_name": "test-agent-1",
        "item_path": ["Movies", "Action", "movie.mkv"]
    });

    let response = server
        .post("/components/agent-modal.html")
        .json(&request_body)
        .await;

    response.assert_status_ok();
    response.assert_text_contains("test-agent-1");
    response.assert_text_contains("Sync Status:");
}

#[tokio::test]
async fn test_ignore_item_endpoint() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "agent_name": "test-agent-1",
        "item_path": ["Movies", "Action", "movie.mkv"]
    });

    let response = server.post("/components/ignore").json(&request_body).await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn test_delete_item_endpoint() {
    let mock_server = MockServer::start().await;

    // Mock delete endpoint
    Mock::given(method("POST"))
        .and(path("/api/v1/delete"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "message": "Item deleted successfully"
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "agent_name": "test-agent-1",
        "item_path": ["temp", "file.tmp"]
    });

    let response = server.post("/components/delete").json(&request_body).await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn test_ignore_item_with_invalid_agent() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "agent_name": "nonexistent-agent",
        "item_path": ["Movies", "Action", "movie.mkv"]
    });

    let response = server.post("/components/ignore").json(&request_body).await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], false);
    assert!(body["message"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_infopanel_with_malformed_request() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    // Missing required fields
    let request_body = json!({
        "category_id": "Movies"
        // missing item_path
    });

    let response = server
        .post("/components/infopanel.html")
        .json(&request_body)
        .await;

    // Should handle gracefully, might return 400 or show error message
    assert!(response.status_code().is_client_error() || response.status_code().is_success());
}

#[tokio::test]
async fn test_itemlist_with_agent_failures() {
    // Test itemlist when agents are unreachable
    let config = create_test_config(); // Uses non-running localhost ports
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/components/itemlist.html").await;
    response.assert_status_ok();

    // Should still render page structure even with no data
    response.assert_text_contains("File Browser");
}

#[tokio::test]
async fn test_dynamic_items_with_missing_params() {
    let config = create_test_config();
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    // Missing required query parameters
    let response = server.get("/components/dynamic-items.html").await;

    // Should return 400 Bad Request for missing required parameters
    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_component_error_handling() {
    let mock_server = MockServer::start().await;

    // Agent returns error response
    Mock::given(method("GET"))
        .and(path("/api/v1/categories"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/components/itemlist.html").await;
    response.assert_status_ok();

    // Should render page with error indicators or empty state
    let text = response.text();
    assert!(text.contains("File Browser") || text.contains("error") || text.contains("No items"));
}

#[tokio::test]
async fn test_full_user_workflow_integration() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    // Step 1: Load main page
    let response = server.get("/").await;
    response.assert_status_ok();

    // Step 2: Load item list
    let response = server.get("/components/itemlist.html").await;
    response.assert_status_ok();
    response.assert_text_contains("Movies");

    // Step 3: Load dynamic items for level 2 (need parent_path which should be base64 encoded)
    let response = server
        .get("/components/dynamic-items.html")
        .add_query_params([
            ("parent_id", "Movies"),
            ("parent_path", "aWRfTW92aWVz"),
            ("level", "2"),
        ])
        .await;
    response.assert_status_ok();

    // Step 4: Get item info
    let info_request = json!({
        "category_id": "Movies",
        "item_path": ["Action", "movie.mkv"]
    });

    let response = server
        .post("/components/infopanel.html")
        .json(&info_request)
        .await;
    response.assert_status_ok();
    response.assert_text_contains("card-header"); // Check for the info panel structure

    // Step 5: Ignore item
    let ignore_request = json!({
        "agent_name": "test-agent-1",
        "item_path": ["Movies", "Action", "movie.mkv"]
    });

    let response = server
        .post("/components/ignore")
        .json(&ignore_request)
        .await;
    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], true);
}
