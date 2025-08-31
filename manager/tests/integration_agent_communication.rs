mod common;

use common::*;
use std::time::Duration;
use stignore_lib::*;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_agent_category_listing_success() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let client = stignore_manager::agent_client::AgentClient::with_timeout(5);

    let result = client.get_categories(&config.agents[0]).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].name, "Movies");
}

#[tokio::test]
async fn test_agent_item_info_success() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let client = stignore_manager::agent_client::AgentClient::with_timeout(5);

    let request = AgentItemInfoRequest {
        item_path: vec!["Action".to_string(), "movie.mkv".to_string()],
    };

    let result = client.get_item_info(&config.agents[0], &request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.item.name, "movie.mkv");
}

#[tokio::test]
async fn test_agent_ignore_item_success() {
    let mock_server = setup_mock_agent_server().await;
    let config = create_test_config_with_mock_server(&mock_server.uri());
    let client = stignore_manager::agent_client::AgentClient::with_timeout(5);

    let request = AgentIgnoreRequest {
        category_id: "Movies".to_string(),
        folder_path: vec!["Action".to_string(), "movie.mkv".to_string()],
    };

    let result = client.ignore_item(&config.agents[0], &request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_agent_timeout_handling() {
    let mock_server = MockServer::start().await;

    // Configure slow response (longer than timeout)
    Mock::given(method("GET"))
        .and(path("/api/v1/categories"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(10)) // 10 second delay
                .set_body_json(create_mock_category_response()),
        )
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let client = stignore_manager::agent_client::AgentClient::with_timeout(2); // 2 second timeout

    let result = client.get_categories(&config.agents[0]).await;
    assert!(result.is_err());

    // Verify it's a timeout error
    match result.unwrap_err() {
        stignore_manager::agent_client::AgentError::Timeout(_) => {
            // Expected timeout error
        }
        stignore_manager::agent_client::AgentError::RequestFailed(e) if e.is_timeout() => {
            // Also acceptable - reqwest timeout
        }
        other => panic!("Expected timeout error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_agent_authentication_required() {
    let mock_server = MockServer::start().await;

    // Mock server requires correct API key
    Mock::given(method("GET"))
        .and(path("/api/v1/categories"))
        .and(header("X-API-Key", "wrong-key"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let mut config = create_test_config_with_mock_server(&mock_server.uri());
    config.agents[0].api_key = "wrong-key".to_string();

    let client = stignore_manager::agent_client::AgentClient::with_timeout(5);
    let result = client.get_categories(&config.agents[0]).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_agent_invalid_response_handling() {
    let mock_server = MockServer::start().await;

    // Return invalid JSON
    Mock::given(method("GET"))
        .and(path("/api/v1/categories"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json response"))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let client = stignore_manager::agent_client::AgentClient::with_timeout(5);

    let result = client.get_categories(&config.agents[0]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_agent_server_error_handling() {
    let mock_server = MockServer::start().await;

    // Return 500 internal server error
    Mock::given(method("GET"))
        .and(path("/api/v1/categories"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_mock_server(&mock_server.uri());
    let client = stignore_manager::agent_client::AgentClient::with_timeout(5);

    let result = client.get_categories(&config.agents[0]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_multi_agent_aggregation_success() {
    // Test that the agents module can aggregate data from multiple agents
    let mock_server1 = setup_mock_agent_server().await;
    let mock_server2 = MockServer::start().await;

    // Setup second agent with different data
    Mock::given(method("GET"))
        .and(path("/api/v1/categories"))
        .and(header("X-API-Key", "test-key-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_mock_category_response()))
        .mount(&mock_server2)
        .await;

    let config = ManagerData {
        manager: ManagerConfig {
            port: 8080,
            minimum_copies: 2,
            agent_timeout_seconds: 5,
        },
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

    let client = stignore_manager::agent_client::AgentClient::with_timeout(5);
    let result = stignore_manager::agents::list_categories(&client, config.agents).await;

    // Should aggregate data from both agents
    assert!(!result.items.is_empty());
}

#[tokio::test]
async fn test_agent_partial_failure_handling() {
    // Test scenario where one agent succeeds and another fails
    let mock_server_good = setup_mock_agent_server().await;
    let mock_server_bad = MockServer::start().await;

    // Bad agent returns error
    Mock::given(method("GET"))
        .and(path("/api/v1/categories"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server_bad)
        .await;

    let config = ManagerData {
        manager: ManagerConfig {
            port: 8080,
            minimum_copies: 2,
            agent_timeout_seconds: 5,
        },
        agents: vec![
            Agent {
                name: "good-agent".to_string(),
                hostname: mock_server_good.uri().replace("http://", ""),
                api_key: "test-key-1".to_string(),
            },
            Agent {
                name: "bad-agent".to_string(),
                hostname: mock_server_bad.uri().replace("http://", ""),
                api_key: "test-key-1".to_string(),
            },
        ],
    };

    let client = stignore_manager::agent_client::AgentClient::with_timeout(5);
    let result = stignore_manager::agents::list_categories(&client, config.agents).await;

    // Should still return data from the good agent
    assert!(!result.items.is_empty());
}
