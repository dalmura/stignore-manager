mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use common::*;
use serde_json::json;
use stignore_lib::*;

fn create_auth_config(enabled: bool) -> ManagerData {
    let mut config = create_test_config();
    config.manager.auth = AuthConfig {
        enabled,
        user_header: "X-Proxy-User".to_string(),
        role_header: "X-Proxy-Role".to_string(),
        admin_role: "Admin".to_string(),
        reader_role: "Reader".to_string(),
    };
    config
}

#[tokio::test]
async fn test_auth_disabled_defaults_to_admin() {
    let config = create_auth_config(false);
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    // GET page should render cleanly
    let response = server.get("/").await;
    response.assert_status_ok();

    // POST ignore endpoint should not return 403 Forbidden
    let response = server
        .post("/components/ignore")
        .json(&json!({
            "agent_name": "test-agent-1",
            "item_path": ["Movies"]
        }))
        .await;

    assert_ne!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_auth_enabled_reader_role_forbidden() {
    let config = create_auth_config(true);
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    // Request with Reader role header
    let response = server
        .post("/components/ignore")
        .add_header("X-Proxy-User", "bob")
        .add_header("X-Proxy-Role", "Reader")
        .json(&json!({
            "agent_name": "test-agent-1",
            "item_path": ["Movies"]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], false);
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("Admin role required")
    );
}

#[tokio::test]
async fn test_auth_enabled_missing_header_defaults_to_reader() {
    let config = create_auth_config(true);
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    // Request without auth headers
    let response = server
        .post("/components/delete")
        .json(&json!({
            "agent_name": "test-agent-1",
            "item_path": ["Movies"]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_auth_enabled_admin_role_allowed() {
    let config = create_auth_config(true);
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    // Request with Admin role header
    let response = server
        .post("/components/ignore")
        .add_header("X-Proxy-User", "alice")
        .add_header("X-Proxy-Role", "Admin")
        .json(&json!({
            "agent_name": "nonexistent-agent",
            "item_path": ["Movies"]
        }))
        .await;

    assert_ne!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_auth_enabled_multi_group_admin_precedence() {
    let config = create_auth_config(true);
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    // Request with comma-separated roles containing both Reader and Admin
    let response = server
        .post("/components/agents/toggle")
        .add_header("X-Proxy-User", "alice")
        .add_header("X-Proxy-Role", "authentik-users, Reader, Admin")
        .json(&json!({
            "agent_name": "test-agent-1",
            "enabled": false
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn test_auth_custom_headers_and_roles() {
    let mut config = create_test_config();
    config.manager.auth = AuthConfig {
        enabled: true,
        user_header: "X-Authentik-Username".to_string(),
        role_header: "X-Authentik-Groups".to_string(),
        admin_role: "stignore-admins".to_string(),
        reader_role: "stignore-readers".to_string(),
    };

    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    // Request with custom Authentik headers
    let response = server
        .post("/components/bulk-ignore")
        .add_header("X-Authentik-Username", "charlie")
        .add_header("X-Authentik-Groups", "stignore-admins")
        .json(&json!({
            "agent_names": ["test-agent-1"],
            "item_path": ["Movies"]
        }))
        .await;

    assert_ne!(response.status_code(), StatusCode::FORBIDDEN);

    // Request with non-admin custom group
    let response = server
        .post("/components/bulk-ignore")
        .add_header("X-Authentik-Username", "charlie")
        .add_header("X-Authentik-Groups", "stignore-readers")
        .json(&json!({
            "agent_names": ["test-agent-1"],
            "item_path": ["Movies"]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}
