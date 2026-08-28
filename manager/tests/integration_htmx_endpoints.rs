mod common;

use axum_test::TestServer;
use common::*;
use serde_json::json;
use stignore_lib::*;
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
    response.assert_text_contains("filter-pill-btn");
    response.assert_text_contains("data-name=");
    response.assert_text_contains("data-insufficient=");
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
    response.assert_text_contains("Unignore Selected");
}

#[tokio::test]
async fn test_infopanel_with_ignored_item_renders_unignore_button() {
    let mock_server = setup_mock_agent_server().await;

    // Mock bulk ignore status endpoint returning ignored: true
    Mock::given(method("POST"))
        .and(path("/api/v1/ignore-status-bulk"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [
                {
                    "ignored": true
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "category_id": "Movies",
        "item_path": ["Movies", "Action", "movie.mkv"]
    });

    let response = server
        .post("/components/infopanel.html")
        .json(&request_body)
        .await;

    response.assert_status_ok();
    response.assert_text_contains("Unignore");
    response.assert_text_contains("setupUnignoreModal(this)");
    response.assert_text_contains("unignore-btn-test-agent-1");
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
async fn test_delete_details_endpoint_success() {
    let mock_server = MockServer::start().await;

    // Mock items endpoint
    Mock::given(method("POST"))
        .and(path("/api/v1/items"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "item": {
                "id": "file.tmp",
                "name": "file.tmp",
                "size_kb": 1024,
                "items": [],
                "leaf": true,
                "copy_count": 1
            }
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

    let response = server
        .post("/components/delete-details")
        .json(&request_body)
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], true);
    assert_eq!(body["name"], "file.tmp");
    assert_eq!(body["is_leaf"], true);
    assert_eq!(body["is_dir"], true);
    assert_eq!(body["size_kb"], 1024);
    assert_eq!(body["item_count"], 0);
    assert_eq!(body["agent_name"], "test-agent-1");
}

#[tokio::test]
async fn test_delete_details_endpoint_invalid_agent() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "agent_name": "non-existent-agent",
        "item_path": ["temp", "file.tmp"]
    });

    let response = server
        .post("/components/delete-details")
        .json(&request_body)
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], false);
    assert!(body["message"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_delete_details_endpoint_empty_path() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "agent_name": "test-agent-1",
        "item_path": []
    });

    let response = server
        .post("/components/delete-details")
        .json(&request_body)
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], false);
    assert!(body["message"].as_str().unwrap().contains("No valid path"));
}

#[tokio::test]
async fn test_bulk_ignore_endpoint_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/ignore"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "message": "Item ignored successfully"
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "agent_names": ["test-agent-1"],
        "item_path": ["Movies", "Action", "movie.mkv"]
    });

    let response = server
        .post("/components/bulk-ignore")
        .json(&request_body)
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], true);
    assert_eq!(body["results"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_bulk_delete_endpoint_success() {
    let mock_server = MockServer::start().await;

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
        "agent_names": ["test-agent-1"],
        "item_path": ["Movies", "Action", "movie.mkv"]
    });

    let response = server
        .post("/components/bulk-delete")
        .json(&request_body)
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], true);
    assert_eq!(body["results"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_bulk_delete_with_missing_agent_treated_as_skipped() {
    let mock_server1 = MockServer::start().await;
    let mock_server2 = MockServer::start().await;

    // Agent 1 succeeds
    Mock::given(method("POST"))
        .and(path("/api/v1/delete"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "message": "Item deleted"
        })))
        .mount(&mock_server1)
        .await;

    // Agent 2 returns 404 Not Found (item already absent)
    Mock::given(method("POST"))
        .and(path("/api/v1/delete"))
        .and(header("X-API-Key", "test-key-2"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "success": false,
            "message": "Path 'movie.mkv' not found"
        })))
        .mount(&mock_server2)
        .await;

    let config = ManagerData {
        manager: ManagerConfig {
            port: 8080,
            minimum_copies: 1,
            agent_timeout_seconds: 5,
            auth: AuthConfig::default(),
        },
        integrations: IntegrationsConfig::default(),
        agents: vec![
            Agent {
                name: "agent-1".to_string(),
                hostname: mock_server1.uri().replace("http://", ""),
                api_key: "test-key-1".to_string(),
            },
            Agent {
                name: "agent-2".to_string(),
                hostname: mock_server2.uri().replace("http://", ""),
                api_key: "test-key-2".to_string(),
            },
        ],
    };

    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "agent_names": ["agent-1", "agent-2"],
        "item_path": ["Movies", "Action", "movie.mkv"]
    });

    let response = server
        .post("/components/bulk-delete")
        .json(&request_body)
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], true);
    assert_eq!(body["results"].as_array().unwrap().len(), 2);
    assert!(
        body["results"][0]["message"]
            .as_str()
            .unwrap()
            .contains("Deleted item on agent-1")
    );
    assert!(
        body["results"][1]["message"]
            .as_str()
            .unwrap()
            .contains("already absent on agent-2 (skipped)")
    );
}

#[tokio::test]
async fn test_delete_item_already_absent_returns_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/delete"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "success": false,
            "message": "Path 'movie.mkv' not found"
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/components/delete")
        .json(&json!({
            "agent_name": "test-agent-1",
            "item_path": ["Movies", "Action", "movie.mkv"]
        }))
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], true);
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("already absent on test-agent-1")
    );
}

#[tokio::test]
async fn test_unignore_item_endpoint() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/unignore"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "message": "Item unignored successfully"
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "agent_name": "test-agent-1",
        "item_path": ["Movies", "Action", "movie.mkv"]
    });

    let response = server
        .post("/components/unignore")
        .json(&request_body)
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn test_bulk_unignore_endpoint_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/unignore"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "message": "Item unignored successfully"
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "agent_names": ["test-agent-1"],
        "item_path": ["Movies", "Action", "movie.mkv"]
    });

    let response = server
        .post("/components/bulk-unignore")
        .json(&request_body)
        .await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], true);
    assert_eq!(body["results"].as_array().unwrap().len(), 1);
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

#[tokio::test]
async fn test_toggle_agent_endpoint() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    // Toggle test-agent-1 off (disabled)
    let toggle_request = json!({
        "agent_name": "test-agent-1",
        "enabled": false
    });

    let response = server
        .post("/components/agents/toggle")
        .json(&toggle_request)
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], true);
    assert_eq!(body["enabled"], false);

    // Verify agents overview shows disabled status
    let overview_resp = server.get("/agents").await;
    overview_resp.assert_status_ok();
    overview_resp.assert_text_contains("Disabled");

    // Toggle test-agent-1 back on (enabled)
    let toggle_on_request = json!({
        "agent_name": "test-agent-1",
        "enabled": true
    });

    let response = server
        .post("/components/agents/toggle")
        .json(&toggle_on_request)
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], true);
    assert_eq!(body["enabled"], true);
}

#[tokio::test]
async fn test_dynamic_items_sorting_options() {
    let mock_server = MockServer::start().await;

    // Custom mock category response with multiple items of varying sizes
    let category_response = AgentCategoryListingResponse {
        items: vec![ItemGroup {
            id: "Media".to_string(),
            name: "Media".to_string(),
            size_kb: 5000,
            items: vec![
                ItemGroup {
                    id: "Media/Small".to_string(),
                    name: "Small Folder".to_string(),
                    size_kb: 100,
                    items: vec![],
                    leaf: false,
                    copy_count: 1,
                    ..Default::default()
                },
                ItemGroup {
                    id: "Media/Big".to_string(),
                    name: "Big Folder".to_string(),
                    size_kb: 4000,
                    items: vec![],
                    leaf: false,
                    copy_count: 1,
                    ..Default::default()
                },
                ItemGroup {
                    id: "Media/Medium".to_string(),
                    name: "Medium Folder".to_string(),
                    size_kb: 900,
                    items: vec![],
                    leaf: false,
                    copy_count: 1,
                    ..Default::default()
                },
            ],
            leaf: false,
            copy_count: 1,
            ..Default::default()
        }],
    };

    Mock::given(method("GET"))
        .and(path("/api/v1/categories"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(category_response))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    // Sanitize ID for "Media" -> "id_TWVkaWE"
    let media_parent_path = "id_TWVkaWE";

    // 1. Test size_desc (Biggest first)

    let resp_size_desc = server
        .get("/components/dynamic-items.html")
        .add_query_param("parent_id", "Media")
        .add_query_param("parent_path", media_parent_path)
        .add_query_param("level", "2")
        .add_query_param("sort", "size_desc")
        .await;

    resp_size_desc.assert_status_ok();
    let text_size_desc = resp_size_desc.text();
    let big_pos = text_size_desc.find("Big Folder").unwrap();
    let medium_pos = text_size_desc.find("Medium Folder").unwrap();
    let small_pos = text_size_desc.find("Small Folder").unwrap();
    assert!(big_pos < medium_pos);
    assert!(medium_pos < small_pos);

    // 2. Test size_asc (Smallest first)
    let resp_size_asc = server
        .get("/components/dynamic-items.html")
        .add_query_param("parent_id", "Media")
        .add_query_param("parent_path", media_parent_path)
        .add_query_param("level", "2")
        .add_query_param("sort", "size_asc")
        .await;

    resp_size_asc.assert_status_ok();
    let text_size_asc = resp_size_asc.text();
    let small_pos_asc = text_size_asc.find("Small Folder").unwrap();
    let medium_pos_asc = text_size_asc.find("Medium Folder").unwrap();
    let big_pos_asc = text_size_asc.find("Big Folder").unwrap();
    assert!(small_pos_asc < medium_pos_asc);
    assert!(medium_pos_asc < big_pos_asc);

    // 3. Test name_asc (A to Z)
    let resp_name_asc = server
        .get("/components/dynamic-items.html")
        .add_query_param("parent_id", "Media")
        .add_query_param("parent_path", media_parent_path)
        .add_query_param("level", "2")
        .add_query_param("sort", "name_asc")
        .await;

    resp_name_asc.assert_status_ok();
    let text_name_asc = resp_name_asc.text();
    let big_pos_name = text_name_asc.find("Big Folder").unwrap();
    let medium_pos_name = text_name_asc.find("Medium Folder").unwrap();
    let small_pos_name = text_name_asc.find("Small Folder").unwrap();
    assert!(big_pos_name < medium_pos_name);
    assert!(medium_pos_name < small_pos_name);

    // 4. Test name_desc (Z to A)
    let resp_name_desc = server
        .get("/components/dynamic-items.html")
        .add_query_param("parent_id", "Media")
        .add_query_param("parent_path", media_parent_path)
        .add_query_param("level", "2")
        .add_query_param("sort", "name_desc")
        .await;

    resp_name_desc.assert_status_ok();
    let text_name_desc = resp_name_desc.text();
    let small_pos_desc = text_name_desc.find("Small Folder").unwrap();
    let medium_pos_desc = text_name_desc.find("Medium Folder").unwrap();
    let big_pos_desc = text_name_desc.find("Big Folder").unwrap();
    assert!(small_pos_desc < medium_pos_desc);
    assert!(medium_pos_desc < big_pos_desc);

    // 5. Test resilience to duplicate sort query params
    let resp_duplicate_sort = server
        .get(&format!(
            "/components/dynamic-items.html?parent_id=Media&parent_path={}&level=2&sort=name_asc&sort=size_desc",
            media_parent_path
        ))
        .await;

    resp_duplicate_sort.assert_status_ok();
    let text_dup = resp_duplicate_sort.text();
    // With sort=size_desc as the last param, Big should appear before Small
    let big_pos_dup = text_dup.find("Big Folder").unwrap();
    let small_pos_dup = text_dup.find("Small Folder").unwrap();
    assert!(big_pos_dup < small_pos_dup);
}

#[tokio::test]
async fn test_agent_status_pill_endpoint_all_online() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/components/agent-status-pill.html").await;
    response.assert_status_ok();
    response.assert_text_contains("agent-status-pill");
    response.assert_text_contains("status-healthy");
    response.assert_text_contains("1/1 Online");
    response.assert_text_contains("href=\"/agents\"");
}

#[tokio::test]
async fn test_agent_status_pill_endpoint_with_offline_agent() {
    let config = create_test_config(); // Test config has unreachable localhost:3001, localhost:3002
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/components/agent-status-pill.html").await;
    response.assert_status_ok();
    response.assert_text_contains("agent-status-pill");
    response.assert_text_contains("status-danger");
    response.assert_text_contains("0/2 Online");
}

#[tokio::test]
async fn test_agent_status_pill_endpoint_no_agents() {
    let mut config = create_test_config();
    config.agents.clear();
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/components/agent-status-pill.html").await;
    response.assert_status_ok();
    response.assert_text_contains("agent-status-pill");
    response.assert_text_contains("status-empty");
    response.assert_text_contains("0 Agents");
}

#[tokio::test]
async fn test_agent_status_pill_endpoint_with_disabled_agent() {
    let mock_server = setup_mock_agent_server().await;
    let mut config = create_test_config_with_mock_server(&mock_server.uri());
    config.agents.push(Agent {
        name: "disabled-agent".to_string(),
        hostname: "localhost:9999".to_string(),
        api_key: "key".to_string(),
    });

    let app_state = create_test_app_state(config);
    {
        let mut disabled = app_state.disabled_agents.write().unwrap();
        disabled.insert("disabled-agent".to_string());
    }

    let app = stignore_manager::create_app(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/components/agent-status-pill.html").await;
    response.assert_status_ok();
    response.assert_text_contains("status-warning");
    response.assert_text_contains("1/1 Online");
    response.assert_text_contains("1 disabled");
}

#[tokio::test]
async fn test_stignore_modal_endpoint_success() {
    let mock_server = MockServer::start().await;
    let stignore_resp = AgentGetStignoreResponse {
        content: "// Test ignore\n(?d).DS_Store\nMovie 1 (2023)/\n".to_string(),
        hash: "a1b2c3d4e5f67890".to_string(),
        exists: true,
        backups: vec![StignoreBackupInfo {
            filename: ".stignore.bak.1700000000".to_string(),
            timestamp: 1700000000,
            size_bytes: 42,
            content: "(?d).DS_Store\n".to_string(),
        }],
    };

    Mock::given(method("POST"))
        .and(path("/api/v1/stignore/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&stignore_resp))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let req_body = json!({
        "agent_name": "test-agent-1",
        "category_id": "movies"
    });

    let response = server
        .post("/components/stignore-modal.html")
        .json(&req_body)
        .await;
    response.assert_status_ok();
    response.assert_text_contains("stignoreTextarea");
    response.assert_text_contains("(?d).DS_Store");
    response.assert_text_contains("#a1b2c3d4e5f67890");
    response.assert_text_contains(".stignore.bak.1700000000");
    response.assert_text_contains("Valid Syntax");
}

#[tokio::test]
async fn test_stignore_modal_endpoint_invalid_agent() {
    let config = create_test_config();
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let req_body = json!({
        "agent_name": "Nonexistent Agent",
        "category_id": "movies"
    });

    let response = server
        .post("/components/stignore-modal.html")
        .json(&req_body)
        .await;
    response.assert_status_ok();
    response.assert_text_contains("Nonexistent Agent");
}

#[tokio::test]
async fn test_stignore_save_endpoint_success() {
    let mock_server = MockServer::start().await;
    let save_resp = AgentSetStignoreResponse {
        success: true,
        message: "Successfully updated .stignore in 'Movies'".to_string(),
        new_hash: "fedcba9876543210".to_string(),
        backup_created: Some(".stignore.bak.1700000001".to_string()),
    };

    Mock::given(method("POST"))
        .and(path("/api/v1/stignore/set"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&save_resp))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let req_body = json!({
        "agent_name": "test-agent-1",
        "category_id": "movies",
        "content": "// Updated rules\n(?d).DS_Store\n",
        "expected_hash": "a1b2c3d4e5f67890"
    });

    let response = server
        .post("/components/stignore/save")
        .json(&req_body)
        .await;
    response.assert_status_ok();
    let json: serde_json::Value = response.json();
    assert_eq!(json["success"], true);
    assert_eq!(json["new_hash"], "fedcba9876543210");
    assert_eq!(json["backup_created"], ".stignore.bak.1700000001");
}

#[tokio::test]
async fn test_stignore_save_endpoint_validation_failure() {
    let config = create_test_config();
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let req_body = json!({
        "agent_name": "test-agent-1",
        "category_id": "movies",
        "content": "unclosed_bracket_[invalid\n",
        "expected_hash": null
    });

    let response = server
        .post("/components/stignore/save")
        .json(&req_body)
        .await;
    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
    let json: serde_json::Value = response.json();
    assert_eq!(json["success"], false);
    assert!(
        json["message"]
            .as_str()
            .unwrap()
            .contains("Validation error")
    );
}

#[tokio::test]
async fn test_stignore_save_endpoint_conflict() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/stignore/set"))
        .respond_with(ResponseTemplate::new(409).set_body_string("Conflict: modified on disk"))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let req_body = json!({
        "agent_name": "test-agent-1",
        "category_id": "movies",
        "content": "(?d).DS_Store\n",
        "expected_hash": "stale_hash"
    });

    let response = server
        .post("/components/stignore/save")
        .json(&req_body)
        .await;
    response.assert_status(axum::http::StatusCode::CONFLICT);
    let json: serde_json::Value = response.json();
    assert_eq!(json["success"], false);
    assert_eq!(json["conflict"], true);
}

#[tokio::test]
async fn test_stignore_restore_endpoint_success() {
    let mock_server = MockServer::start().await;
    let restore_resp = AgentRestoreStignoreResponse {
        success: true,
        message: "Successfully restored".to_string(),
        restored_content: "(?d).DS_Store\n".to_string(),
        new_hash: "restoredhash1234".to_string(),
    };

    Mock::given(method("POST"))
        .and(path("/api/v1/stignore/restore"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&restore_resp))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let req_body = json!({
        "agent_name": "test-agent-1",
        "category_id": "movies",
        "backup_filename": ".stignore.bak.1700000000"
    });

    let response = server
        .post("/components/stignore/restore")
        .json(&req_body)
        .await;
    response.assert_status_ok();
    let json: serde_json::Value = response.json();
    assert_eq!(json["success"], true);
    assert_eq!(json["restored_content"], "(?d).DS_Store\n");
    assert_eq!(json["new_hash"], "restoredhash1234");
}

#[tokio::test]
async fn test_stignore_validate_endpoint() {
    let config = create_test_config();
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let req_body = json!({
        "content": "// Comment\n(?d).DS_Store\n#include custom_ignores\n"
    });

    let response = server
        .post("/components/stignore/validate")
        .json(&req_body)
        .await;
    response.assert_status_ok();
    let json: serde_json::Value = response.json();
    assert_eq!(json["report"]["is_valid"], true);
    assert_eq!(json["report"]["rule_count"], 1);
    assert_eq!(json["report"]["comment_count"], 1);
    assert_eq!(json["report"]["include_count"], 1);
}

#[tokio::test]
async fn test_itemlist_filters_hidden_when_no_conflicts_or_syncing() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/components/itemlist.html").await;
    response.assert_status_ok();
    // In default mock without conflicts/syncing, these buttons should not appear
    let text = response.text();
    assert!(!text.contains("data-filter=\"conflicts\""));
    assert!(!text.contains("data-filter=\"syncing\""));
    assert!(text.contains("data-filter=\"all\""));
    assert!(text.contains("data-filter=\"insufficient\""));
    assert!(text.contains("data-filter=\"synced\""));
}

#[tokio::test]
async fn test_itemlist_component_renders_conflict_and_syncing_filters_when_present() {
    let mock_server = MockServer::start().await;

    let cat_resp = AgentCategoryListingResponse {
        items: vec![ItemGroup {
            id: "Movies".to_string(),
            name: "Movies".to_string(),
            size_kb: 1024,
            items: vec![],
            leaf: false,
            copy_count: 1,
            has_conflicts: true,
            conflict_count: 1,
            is_syncing: true,
            ..Default::default()
        }],
    };

    Mock::given(method("GET"))
        .and(path("/api/v1/categories"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&cat_resp))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/components/itemlist.html").await;
    response.assert_status_ok();
    response.assert_text_contains("data-filter=\"conflicts\"");
    response.assert_text_contains("data-filter=\"syncing\"");
    response.assert_text_contains("data-conflicts=");
    response.assert_text_contains("data-syncing=");
}

#[tokio::test]
async fn test_infopanel_with_conflicts_and_syncing_renders_alerts() {
    let mock_server = MockServer::start().await;

    let item_with_conflict = ItemGroup {
        id: "Movies/Action/movie.mkv".to_string(),
        name: "movie.mkv".to_string(),
        size_kb: 2048,
        items: vec![],
        leaf: true,
        copy_count: 1,
        has_conflicts: true,
        conflict_count: 2,
        is_syncing: true,
        stversions_size_kb: 512,
        stfolder_present: true,
    };

    Mock::given(method("POST"))
        .and(path("/api/v1/items"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(AgentItemInfoResponse {
                item: item_with_conflict,
            }),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/ignore-status-bulk"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [{"ignored": false}]
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "category_id": "Movies",
        "item_path": ["Movies", "Action", "movie.mkv"]
    });

    let response = server
        .post("/components/infopanel.html")
        .json(&request_body)
        .await;

    response.assert_status_ok();
    response.assert_text_contains("Sync Conflicts Detected");
    response.assert_text_contains("2 conflict file(s) found");
    response.assert_text_contains("Active Sync in Progress");
}

#[tokio::test]
async fn test_agent_modal_renders_conflict_and_syncing_badges() {
    let mock_server = MockServer::start().await;

    let parent_with_conflict_child = ItemGroup {
        id: "Movies/Action".to_string(),
        name: "Action".to_string(),
        size_kb: 4096,
        items: vec![
            ItemGroup {
                id: "movie.sync-conflict-20230101.mkv".to_string(),
                name: "movie.sync-conflict-20230101.mkv".to_string(),
                size_kb: 2048,
                items: vec![],
                leaf: true,
                copy_count: 1,
                has_conflicts: true,
                conflict_count: 1,
                is_syncing: false,
                ..Default::default()
            },
            ItemGroup {
                id: "movie_transfer.mkv".to_string(),
                name: "movie_transfer.mkv".to_string(),
                size_kb: 2048,
                items: vec![],
                leaf: true,
                copy_count: 1,
                has_conflicts: false,
                conflict_count: 0,
                is_syncing: true,
                ..Default::default()
            },
        ],
        leaf: false,
        copy_count: 1,
        has_conflicts: true,
        conflict_count: 1,
        is_syncing: true,
        stversions_size_kb: 0,
        stfolder_present: true,
    };

    Mock::given(method("POST"))
        .and(path("/api/v1/items"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(AgentItemInfoResponse {
                item: parent_with_conflict_child,
            }),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/ignore-status-bulk"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [{"ignored": false}]
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "agent_name": "test-agent-1",
        "item_path": ["Movies", "Action"]
    });

    let response = server
        .post("/components/agent-modal.html")
        .json(&request_body)
        .await;

    response.assert_status_ok();
    response.assert_text_contains("⚠️ Conflict");
    response.assert_text_contains("Syncing in progress");
}
