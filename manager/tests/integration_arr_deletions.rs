mod common;

use axum_test::TestServer;
use common::*;
use serde_json::json;
use stignore_lib::*;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_radarr_zero_copy_movie_deletion() {
    let mock_agent_server = setup_mock_agent_server().await;
    let mock_radarr_server = MockServer::start().await;

    // Mock Radarr GET movies
    Mock::given(method("GET"))
        .and(path("/api/v3/movie"))
        .and(header("X-Api-Key", "test-radarr-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": 42,
                "title": "Inception",
                "cleanTitle": "inception",
                "year": 2010,
                "path": "/data/movies/Inception (2010)",
                "monitored": true,
                "hasFile": true
            }
        ])))
        .mount(&mock_radarr_server)
        .await;

    // Mock Radarr DELETE movie
    Mock::given(method("DELETE"))
        .and(path("/api/v3/movie/42"))
        .and(query_param("deleteFiles", "false"))
        .and(query_param("addImportExclusion", "false"))
        .and(header("X-Api-Key", "test-radarr-key"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_radarr_server)
        .await;

    // Mock Agent DELETE endpoint
    Mock::given(method("POST"))
        .and(path("/api/v1/delete"))
        .and(header("X-API-Key", "test-key-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "message": "Item deleted from filesystem"
        })))
        .mount(&mock_agent_server)
        .await;

    let config = ManagerData {
        manager: ManagerConfig {
            port: 8080,
            minimum_copies: 1,
            agent_timeout_seconds: 5,
            auth: AuthConfig::default(),
        },
        integrations: IntegrationsConfig {
            radarr: Some(RadarrConfig {
                enabled: true,
                url: mock_radarr_server.uri(),
                api_key: "test-radarr-key".to_string(),
                category_id: "movies".to_string(),
                delete_files: false,
                add_import_exclusion: false,
            }),
            sonarr: None,
        },
        agents: vec![Agent {
            name: "test-agent-1".to_string(),
            hostname: mock_agent_server.uri().replace("http://", ""),
            api_key: "test-key-1".to_string(),
        }],
    };

    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    // 1. Check delete-details inspects Radarr
    let details_resp = server
        .post("/components/delete-details")
        .json(&json!({
            "agent_name": "test-agent-1",
            "item_path": ["movies", "Inception (2010)"]
        }))
        .await;

    details_resp.assert_status_ok();
    let details_json = details_resp.json::<serde_json::Value>();
    assert!(details_json["is_last_copy"].as_bool().unwrap());
    assert_eq!(details_json["matched_media"]["service"], "Radarr");
    assert_eq!(details_json["matched_media"]["title"], "Inception");

    // 2. Perform delete
    let delete_resp = server
        .post("/components/delete")
        .json(&json!({
            "agent_name": "test-agent-1",
            "item_path": ["movies", "Inception (2010)"]
        }))
        .await;

    delete_resp.assert_status_ok();
    let delete_json = delete_resp.json::<serde_json::Value>();
    assert!(delete_json["success"].as_bool().unwrap());
    assert!(
        delete_json["message"]
            .as_str()
            .unwrap()
            .contains("Inception")
    );
    assert_eq!(delete_json["media_result"]["service"], "Radarr");
    assert!(delete_json["media_result"]["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_radarr_custom_import_exclusion_override() {
    let mock_agent_server = setup_mock_agent_server().await;
    let mock_radarr_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v3/movie"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": 99,
                "title": "Dune: Part Two",
                "year": 2024,
                "path": "/data/movies/Dune: Part Two (2024)",
                "monitored": true,
                "hasFile": true
            }
        ])))
        .mount(&mock_radarr_server)
        .await;

    // Expect addImportExclusion=true
    Mock::given(method("DELETE"))
        .and(path("/api/v3/movie/99"))
        .and(query_param("addImportExclusion", "true"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_radarr_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/delete"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "message": "Item deleted"
        })))
        .mount(&mock_agent_server)
        .await;

    let config = ManagerData {
        manager: ManagerConfig {
            port: 8080,
            minimum_copies: 1,
            agent_timeout_seconds: 5,
            auth: AuthConfig::default(),
        },
        integrations: IntegrationsConfig {
            radarr: Some(RadarrConfig {
                enabled: true,
                url: mock_radarr_server.uri(),
                api_key: "test-radarr-key".to_string(),
                category_id: "movies".to_string(),
                delete_files: false,
                add_import_exclusion: false,
            }),
            sonarr: None,
        },
        agents: vec![Agent {
            name: "test-agent-1".to_string(),
            hostname: mock_agent_server.uri().replace("http://", ""),
            api_key: "test-key-1".to_string(),
        }],
    };

    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let delete_resp = server
        .post("/components/delete")
        .json(&json!({
            "agent_name": "test-agent-1",
            "item_path": ["movies", "Dune: Part Two (2024)"],
            "add_import_exclusion": true
        }))
        .await;

    delete_resp.assert_status_ok();
}

#[tokio::test]
async fn test_sonarr_zero_copy_series_deletion() {
    let mock_agent_server = setup_mock_agent_server().await;
    let mock_sonarr_server = MockServer::start().await;

    // Mock Sonarr GET series
    Mock::given(method("GET"))
        .and(path("/api/v3/series"))
        .and(header("X-Api-Key", "test-sonarr-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": 101,
                "title": "Severance",
                "cleanTitle": "severance",
                "year": 2022,
                "path": "/data/tv/Severance (2022)",
                "monitored": true,
                "seasons": [
                    { "seasonNumber": 1, "monitored": true }
                ]
            }
        ])))
        .mount(&mock_sonarr_server)
        .await;

    // Mock Sonarr DELETE series
    Mock::given(method("DELETE"))
        .and(path("/api/v3/series/101"))
        .and(query_param("deleteFiles", "false"))
        .and(query_param("addImportListExclusion", "false"))
        .and(header("X-Api-Key", "test-sonarr-key"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_sonarr_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/delete"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "message": "Item deleted"
        })))
        .mount(&mock_agent_server)
        .await;

    let config = ManagerData {
        manager: ManagerConfig {
            port: 8080,
            minimum_copies: 1,
            agent_timeout_seconds: 5,
            auth: AuthConfig::default(),
        },
        integrations: IntegrationsConfig {
            radarr: None,
            sonarr: Some(SonarrConfig {
                enabled: true,
                url: mock_sonarr_server.uri(),
                api_key: "test-sonarr-key".to_string(),
                category_id: "tv".to_string(),
                delete_files: false,
                add_import_list_exclusion: false,
            }),
        },
        agents: vec![Agent {
            name: "test-agent-1".to_string(),
            hostname: mock_agent_server.uri().replace("http://", ""),
            api_key: "test-key-1".to_string(),
        }],
    };

    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let delete_resp = server
        .post("/components/delete")
        .json(&json!({
            "agent_name": "test-agent-1",
            "item_path": ["tv", "Severance (2022)"]
        }))
        .await;

    delete_resp.assert_status_ok();
    let delete_json = delete_resp.json::<serde_json::Value>();
    assert!(delete_json["success"].as_bool().unwrap());
    assert_eq!(delete_json["media_result"]["service"], "Sonarr");
}

#[tokio::test]
async fn test_sonarr_season_deletion_and_unmonitor() {
    let mock_agent_server = setup_mock_agent_server().await;
    let mock_sonarr_server = MockServer::start().await;

    // Mock Sonarr GET series
    Mock::given(method("GET"))
        .and(path("/api/v3/series"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": 101,
                "title": "Severance",
                "cleanTitle": "severance",
                "year": 2022,
                "path": "/data/tv/Severance",
                "monitored": true,
                "seasons": [
                    { "seasonNumber": 1, "monitored": true },
                    { "seasonNumber": 2, "monitored": true }
                ]
            }
        ])))
        .mount(&mock_sonarr_server)
        .await;

    // Mock Sonarr GET episode files
    Mock::given(method("GET"))
        .and(path("/api/v3/episodefile"))
        .and(query_param("seriesId", "101"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": 501,
                "seriesId": 101,
                "seasonNumber": 1,
                "relativePath": "Season 01/S01E01.mkv",
                "path": "/data/tv/Severance/Season 01/S01E01.mkv"
            },
            {
                "id": 502,
                "seriesId": 101,
                "seasonNumber": 2,
                "relativePath": "Season 02/S02E01.mkv",
                "path": "/data/tv/Severance/Season 02/S02E01.mkv"
            }
        ])))
        .mount(&mock_sonarr_server)
        .await;

    // Mock Sonarr DELETE episode file for season 1
    Mock::given(method("DELETE"))
        .and(path("/api/v3/episodefile/501"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_sonarr_server)
        .await;

    // Mock Sonarr GET series by id
    Mock::given(method("GET"))
        .and(path("/api/v3/series/101"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 101,
            "title": "Severance",
            "year": 2022,
            "path": "/data/tv/Severance",
            "monitored": true,
            "seasons": [
                { "seasonNumber": 1, "monitored": true },
                { "seasonNumber": 2, "monitored": true }
            ]
        })))
        .mount(&mock_sonarr_server)
        .await;

    // Mock Sonarr PUT series with season 1 unmonitored
    Mock::given(method("PUT"))
        .and(path("/api/v3/series/101"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&mock_sonarr_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/delete"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "message": "Season folder deleted"
        })))
        .mount(&mock_agent_server)
        .await;

    let config = ManagerData {
        manager: ManagerConfig {
            port: 8080,
            minimum_copies: 1,
            agent_timeout_seconds: 5,
            auth: AuthConfig::default(),
        },
        integrations: IntegrationsConfig {
            radarr: None,
            sonarr: Some(SonarrConfig {
                enabled: true,
                url: mock_sonarr_server.uri(),
                api_key: "test-sonarr-key".to_string(),
                category_id: "tv".to_string(),
                delete_files: false,
                add_import_list_exclusion: false,
            }),
        },
        agents: vec![Agent {
            name: "test-agent-1".to_string(),
            hostname: mock_agent_server.uri().replace("http://", ""),
            api_key: "test-key-1".to_string(),
        }],
    };

    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let delete_resp = server
        .post("/components/delete")
        .json(&json!({
            "agent_name": "test-agent-1",
            "item_path": ["tv", "Severance", "Season 01"]
        }))
        .await;

    delete_resp.assert_status_ok();
    let delete_json = delete_resp.json::<serde_json::Value>();
    assert!(delete_json["success"].as_bool().unwrap());
    assert_eq!(delete_json["media_result"]["service"], "Sonarr");
    assert!(
        delete_json["media_result"]["message"]
            .as_str()
            .unwrap()
            .contains("Season 1")
    );
}

#[tokio::test]
async fn test_bulk_delete_with_radarr_integration() {
    let mock_agent_server = setup_mock_agent_server().await;
    let mock_radarr_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v3/movie"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": 12,
                "title": "Interstellar",
                "year": 2014,
                "path": "/data/movies/Interstellar (2014)",
                "monitored": true,
                "hasFile": true
            }
        ])))
        .mount(&mock_radarr_server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/api/v3/movie/12"))
        .and(query_param("deleteFiles", "false"))
        .and(query_param("addImportExclusion", "false"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_radarr_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/delete"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "message": "Deleted"
        })))
        .mount(&mock_agent_server)
        .await;

    let config = ManagerData {
        manager: ManagerConfig {
            port: 8080,
            minimum_copies: 1,
            agent_timeout_seconds: 5,
            auth: AuthConfig::default(),
        },
        integrations: IntegrationsConfig {
            radarr: Some(RadarrConfig {
                enabled: true,
                url: mock_radarr_server.uri(),
                api_key: "test-radarr-key".to_string(),
                category_id: "movies".to_string(),
                delete_files: false,
                add_import_exclusion: false,
            }),
            sonarr: None,
        },
        agents: vec![Agent {
            name: "test-agent-1".to_string(),
            hostname: mock_agent_server.uri().replace("http://", ""),
            api_key: "test-key-1".to_string(),
        }],
    };

    let app = create_test_app(config);
    let server = TestServer::new(app).unwrap();

    let bulk_delete_resp = server
        .post("/components/bulk-delete")
        .json(&json!({
            "agent_names": ["test-agent-1"],
            "item_path": ["movies", "Interstellar (2014)"]
        }))
        .await;

    bulk_delete_resp.assert_status_ok();
    let bulk_json = bulk_delete_resp.json::<serde_json::Value>();
    assert!(bulk_json["success"].as_bool().unwrap());
    assert_eq!(bulk_json["media_result"]["service"], "Radarr");
    assert!(bulk_json["media_result"]["success"].as_bool().unwrap());
}
