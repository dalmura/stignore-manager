#![allow(dead_code)] // Test helpers are used across different test files

use axum::Router;
use std::io::Write;
use tempfile::NamedTempFile;
use tera::Tera;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use stignore_lib::*;
use stignore_manager::{AppState, Context, TeraEngine, agent_client, humansize_filter};

pub fn create_test_config() -> ManagerData {
    ManagerData {
        manager: ManagerConfig {
            port: 8080,
            minimum_copies: 2,
            agent_timeout_seconds: 5,
            auth: AuthConfig::default(),
        },
        integrations: IntegrationsConfig::default(),
        agents: vec![
            Agent {
                name: "test-agent-1".to_string(),
                hostname: "localhost:3001".to_string(),
                api_key: "test-key-1".to_string(),
            },
            Agent {
                name: "test-agent-2".to_string(),
                hostname: "localhost:3002".to_string(),
                api_key: "test-key-2".to_string(),
            },
        ],
    }
}

pub async fn create_test_config_file() -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().unwrap();
    let config_content = r#"
[manager]
port = 8080
minimum_copies = 2
agent_timeout_seconds = 5

[[agents]]
name = "test-agent-1"
hostname = "localhost:3001"
api_key = "test-key-1"

[[agents]]
name = "test-agent-2"
hostname = "localhost:3002"
api_key = "test-key-2"
"#;
    temp_file.write_all(config_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();
    temp_file
}

pub fn create_test_app_state(config: ManagerData) -> AppState {
    let mut tera = Tera::default();
    tera.register_filter("humansize", humansize_filter);
    tera.load_from_glob("html/**/*.html").unwrap();
    let mut context = Context::new();
    context.insert("title", "stignore-manager-test");
    context.insert("version", "0.1.0-test");
    context.insert("repo_url", "https://github.com/dalmura/stignore-manager");

    let integrations =
        std::sync::Arc::new(stignore_manager::integrations::IntegrationsManager::new(
            &config.integrations,
            config.manager.agent_timeout_seconds,
        ));

    AppState {
        engine: TeraEngine(tera),
        context,
        config: config.clone(),
        agent_client: agent_client::AgentClient::with_timeout(config.manager.agent_timeout_seconds),
        disabled_agents: std::sync::Arc::new(std::sync::RwLock::new(
            std::collections::HashSet::new(),
        )),
        integrations,
    }
}

pub fn create_test_app(config: ManagerData) -> Router {
    let app_state = create_test_app_state(config);
    stignore_manager::create_app(app_state)
}

pub fn create_mock_category_response() -> AgentCategoryListingResponse {
    let item1 = ItemGroup {
        id: "Movies".to_string(),
        name: "Movies".to_string(),
        size_kb: 1024,
        items: vec![ItemGroup {
            id: "Movies/Action".to_string(),
            name: "Action".to_string(),
            size_kb: 512,
            items: vec![],
            leaf: false,
            copy_count: 1,
            ..Default::default()
        }],
        leaf: false,
        copy_count: 1,
        ..Default::default()
    };

    AgentCategoryListingResponse { items: vec![item1] }
}

pub fn create_mock_item_info_response() -> AgentItemInfoResponse {
    let item = ItemGroup {
        id: "Movies/Action/movie.mkv".to_string(),
        name: "movie.mkv".to_string(),
        size_kb: 2048,
        items: vec![],
        leaf: true,
        copy_count: 1,
        ..Default::default()
    };

    AgentItemInfoResponse { item }
}

pub async fn setup_mock_agent_server() -> MockServer {
    let mock_server = MockServer::start().await;

    // Mock categories endpoint
    Mock::given(method("GET"))
        .and(path("/api/v1/categories"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_mock_category_response()))
        .mount(&mock_server)
        .await;

    // Mock item info endpoint
    Mock::given(method("POST"))
        .and(path("/api/v1/items"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_mock_item_info_response()))
        .mount(&mock_server)
        .await;

    // Mock ignore endpoint
    Mock::given(method("POST"))
        .and(path("/api/v1/ignore"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": true,
            "message": "Item ignored successfully"
        })))
        .mount(&mock_server)
        .await;

    // Mock unignore endpoint
    Mock::given(method("POST"))
        .and(path("/api/v1/unignore"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": true,
            "message": "Item unignored successfully"
        })))
        .mount(&mock_server)
        .await;

    mock_server
}

pub fn create_test_config_with_mock_server(server_uri: &str) -> ManagerData {
    ManagerData {
        manager: ManagerConfig {
            port: 8080,
            minimum_copies: 2,
            agent_timeout_seconds: 5,
            auth: AuthConfig::default(),
        },
        integrations: IntegrationsConfig::default(),
        agents: vec![Agent {
            name: "test-agent-1".to_string(),
            hostname: server_uri.replace("http://", ""),
            api_key: "test-key-1".to_string(),
        }],
    }
}
