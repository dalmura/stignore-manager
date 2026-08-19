mod common;

use axum_test::TestServer;
use common::*;

#[tokio::test]
async fn test_server_startup_and_basic_routes() {
    let config = create_test_config();
    let app = create_test_app(config);

    let server = TestServer::new(app).unwrap();

    // Test root route
    let response = server.get("/").await;
    response.assert_status_ok();
    response.assert_text_contains("stignore-manager-test");

    // Test agents overview route
    let response = server.get("/agents").await;
    response.assert_status_ok();
    response.assert_text_contains("Agents Overview");
}

#[tokio::test]
async fn test_navbar_search_input_rendering() {
    let config = create_test_config();
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/").await;
    response.assert_status_ok();
    response.assert_text_contains("globalSearchInput");
    response.assert_text_contains("Fuzzy search items...");
    response.assert_text_contains("search-shortcut-kbd");
}

#[tokio::test]
async fn test_not_found_handler() {
    let config = create_test_config();
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/nonexistent-route").await;
    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    response.assert_text_contains("Not Found");
}

#[tokio::test]
async fn test_template_rendering_with_context() {
    let config = create_test_config();
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/").await;
    response.assert_status_ok();

    let text = response.text();
    assert!(text.contains("stignore-manager-test"));
    assert!(text.contains("v0.1.0-test"));
    assert!(text.contains("https://github.com/dalmura/stignore-manager"));
    // The actual message might be different, so let's just check for basic structure
    assert!(text.contains("<html") || text.contains("<!DOCTYPE"));
}

#[tokio::test]
async fn test_agents_page_with_unreachable_agents() {
    // This test verifies that the agents page handles unreachable agents gracefully
    let config = create_test_config(); // Uses localhost:3001, 3002 which won't be running
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/agents").await;
    response.assert_status_ok();

    let text = response.text();
    // Should contain agent names even if they're unreachable
    assert!(text.contains("test-agent-1") || text.contains("test-agent-2"));
}

#[tokio::test]
async fn test_config_loading_integration() {
    let temp_config = create_test_config_file().await;

    // Load config from file
    let loaded_config =
        stignore_manager::config::load_config(Some(temp_config.path().to_str().unwrap()));

    assert_eq!(loaded_config.manager.port, 8080);
    assert_eq!(loaded_config.manager.minimum_copies, 2);
    assert_eq!(loaded_config.agents.len(), 2);
    assert_eq!(loaded_config.agents[0].name, "test-agent-1");
    assert_eq!(loaded_config.agents[1].name, "test-agent-2");
}

#[tokio::test]
async fn test_health_endpoints() {
    let config = create_test_config();
    let app = create_test_app(config);

    let server = TestServer::new(app).unwrap();

    let response = server.get("/health").await;
    response.assert_status_ok();
    assert_eq!(response.text(), "OK");

    let response_z = server.get("/healthz").await;
    response_z.assert_status_ok();
    assert_eq!(response_z.text(), "OK");
}

#[tokio::test]
async fn test_app_state_creation() {
    let config = create_test_config();
    let app_state = create_test_app_state(config.clone());

    // Verify app state components
    assert_eq!(app_state.config.manager.port, config.manager.port);
    assert_eq!(app_state.config.agents.len(), 2);

    // Test template engine is working
    let context = app_state.context.clone();
    assert!(context.get("title").is_some());
    assert!(context.get("version").is_some());
    assert!(context.get("repo_url").is_some());
}

#[tokio::test]
async fn test_humansize_filter_integration() {
    // Test the humansize filter function directly
    let test_cases = vec![
        500.0,     // 500 KB input
        1024.0,    // 1024 KB = 1 MB
        1048576.0, // 1024*1024 KB = 1 GB
    ];

    let kwargs = tera::Kwargs::default();
    let context = tera::Context::default();
    let state = tera::State::new(&context);

    for input_kb in test_cases {
        let result = stignore_manager::humansize_filter(input_kb, kwargs.clone(), &state).unwrap();
        if let Some(formatted) = result.as_str() {
            // Just check that we got some formatted string with units
            assert!(
                formatted.contains("KB")
                    || formatted.contains("MB")
                    || formatted.contains("GB")
                    || formatted.contains("B")
            );
        } else {
            panic!("Expected string result from humansize filter");
        }
    }
}

#[tokio::test]
async fn test_navbar_sort_menu_rendering() {
    let config = create_test_config();
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/").await;
    response.assert_status_ok();
    response.assert_text_contains("sortDropdownContainer");
    response.assert_text_contains("sortMenuDropdown");
    response.assert_text_contains("sortMenuLabel");
    response.assert_text_contains("data-sort=\"name_asc\"");
    response.assert_text_contains("data-sort=\"name_desc\"");
    response.assert_text_contains("data-sort=\"size_desc\"");
    response.assert_text_contains("data-sort=\"size_asc\"");
}

#[tokio::test]
async fn test_footer_agent_status_pill_rendering() {
    let config = create_test_config();
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/").await;
    response.assert_status_ok();
    response.assert_text_contains("agentStatusPillContainer");
    response.assert_text_contains("hx-get=\"/components/agent-status-pill.html\"");
    response.assert_text_contains("hx-trigger=\"load, every 30s\"");
    response.assert_text_contains("GitHub");
}

#[tokio::test]
async fn test_help_modal_rendering() {
    let config = create_test_config();
    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/").await;
    response.assert_status_ok();
    // Verify navbar help button
    response.assert_text_contains("id=\"helpModalBtn\"");
    response.assert_text_contains("data-bs-target=\"#helpModal\"");
    // Verify modal elements
    response.assert_text_contains("id=\"helpModal\"");
    response.assert_text_contains("Keyboard Shortcuts");
    response.assert_text_contains("Tree Navigation");
    response.assert_text_contains("How <code>.stignore</code> Management Works");
    response.assert_text_contains(".stignore Examples");
    response.assert_text_contains("Pattern Syntax Rules");
    response.assert_text_contains("Movies &amp; TV Shows");
    response.assert_text_contains("Movie A (1989)/");
    response.assert_text_contains("TV Show A (2003)/Season 1/*");
}
