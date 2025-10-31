use crate::filesystem;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use std::path::PathBuf;
use stignore_lib::*;

/// Helper function to build the category base path
fn build_category_base_path(agent_config: &AgentConfig, category: &Category) -> PathBuf {
    std::path::Path::new(&agent_config.base_path).join(&category.relative_path)
}

pub async fn help() -> Html<&'static str> {
    Html(
        "Please visit <a href='https://github.com/dalmura/stignore-agent'>the documentation</a> for further information",
    )
}

// GET categories
// Returns all configured categories that the agent is configured for!
pub async fn category_list(State(data): State<AgentData>) -> impl IntoResponse {
    let items = data
        .categories
        .iter()
        .map(|c| {
            let category_path = build_category_base_path(&data.agent, c);
            let children = filesystem::build_items(&category_path, false);

            ItemGroup {
                id: c.id.clone(),
                name: c.name.clone(),
                size_kb: children.iter().map(|c| c.size_kb).sum(),
                items: children,
                leaf: false,
                copy_count: 1,
            }
        })
        .collect();

    (StatusCode::OK, Json(CategoryListingResponse { items }))
}

// GET category info
// Returns specific info for a given category
pub async fn category_info(
    State(data): State<AgentData>,
    Path(category_id): Path<String>,
) -> Response {
    match data.categories.iter().find(|x| x.id == category_id) {
        Some(category) => {
            let category_path = build_category_base_path(&data.agent, category);

            (
                StatusCode::OK,
                Json(CategoryInfoResponse {
                    name: category.name.clone(),
                    items: filesystem::build_items(&category_path, false),
                }),
            )
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(NotFoundResponse {
                message: format!("Category ID {} not found", category_id),
            }),
        )
            .into_response(),
    }
}

// POST itemgroup info
// Returns specific info for a given itemgroup
// We must be given a series of correct itemgroup names to traverse
pub async fn post_item_info(
    State(data): State<AgentData>,
    Json(payload): Json<ItemInfoRequest>,
) -> Response {
    let item_path: Vec<&str> = payload.item_path.iter().map(AsRef::as_ref).collect();

    // Validate that the first item in the path corresponds to a valid category
    if item_path.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(NotFoundResponse {
                message: "Item path cannot be empty".to_string(),
            }),
        )
            .into_response();
    }

    let category_id = &item_path[0];
    let category = match data.categories.iter().find(|c| c.id == *category_id) {
        Some(cat) => cat,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(NotFoundResponse {
                    message: format!("Category ID '{}' not found", category_id),
                }),
            )
                .into_response();
        }
    };

    let category_path = build_category_base_path(&data.agent, category);

    if item_path.len() == 1 {
        // Return the category itself
        let items = filesystem::build_items(&category_path, false);
        let category_item = ItemGroup {
            id: category.id.clone(),
            name: category.name.clone(),
            size_kb: items.iter().map(|c| c.size_kb).sum(),
            items,
            leaf: false,
            copy_count: 1,
        };
        return (
            StatusCode::OK,
            Json(ItemInfoResponse {
                item: category_item,
            }),
        )
            .into_response();
    }

    // Navigate to the specific item within the category
    let item_path_within_category = &item_path[1..];
    match filesystem::get_item(&category_path, item_path_within_category) {
        Some(item) => (StatusCode::OK, Json(ItemInfoResponse { item })).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(NotFoundResponse {
                message: format!("Item Path '{:?}' not found", &item_path),
            }),
        )
            .into_response(),
    }
}

// POST ignore
// Adds a folder path to .stignore in the appropriate category
pub async fn post_ignore(
    State(data): State<AgentData>,
    Json(payload): Json<IgnoreRequest>,
) -> Response {
    tracing::info!(
        "Processing ignore request for category: '{}', folder_path: {:?}",
        payload.category_id,
        payload.folder_path
    );

    // Validate folder path is not empty
    if payload.folder_path.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(IgnoreResponse {
                success: false,
                message: "Folder path cannot be empty".to_string(),
                ignored_path: None,
            }),
        )
            .into_response();
    }

    // Find the category by matching the category ID
    let category = match data.categories.iter().find(|c| c.id == payload.category_id) {
        Some(cat) => cat,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(IgnoreResponse {
                    success: false,
                    message: format!("Category ID '{}' not found", payload.category_id),
                    ignored_path: None,
                }),
            )
                .into_response();
        }
    };

    let category_base_path = build_category_base_path(&data.agent, category);

    // Add to .stignore using the folder path components directly
    match filesystem::add_to_stignore(&category_base_path, &payload.folder_path, &category.name) {
        filesystem::StignoreResult::Success {
            ignored_path,
            message,
        } => (
            StatusCode::OK,
            Json(IgnoreResponse {
                success: true,
                message,
                ignored_path: Some(ignored_path),
            }),
        )
            .into_response(),
        filesystem::StignoreResult::AlreadyIgnored { ignored_path } => (
            StatusCode::OK,
            Json(IgnoreResponse {
                success: true,
                message: "Path is already ignored".to_string(),
                ignored_path: Some(ignored_path),
            }),
        )
            .into_response(),
        filesystem::StignoreResult::Error { message } => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(IgnoreResponse {
                success: false,
                message,
                ignored_path: None,
            }),
        )
            .into_response(),
    }
}

// POST ignore status
// Checks if a folder is ignored in .stignore
pub async fn post_ignore_status(
    State(data): State<AgentData>,
    Json(payload): Json<IgnoreStatusRequest>,
) -> Response {
    // Validate folder path is not empty
    if payload.folder_path.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(IgnoreStatusResponse { ignored: false }),
        )
            .into_response();
    }

    // Find the category by matching the category ID
    let category = match data.categories.iter().find(|c| c.id == payload.category_id) {
        Some(cat) => cat,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(IgnoreStatusResponse { ignored: false }),
            )
                .into_response();
        }
    };

    let category_base_path = build_category_base_path(&data.agent, category);

    // Check if the folder path is ignored
    let ignored = filesystem::is_path_ignored(&category_base_path, &payload.folder_path);

    (StatusCode::OK, Json(IgnoreStatusResponse { ignored })).into_response()
}

// POST ignore-status-bulk
// Checks ignore status for multiple folders at once
pub async fn post_ignore_status_bulk(
    State(data): State<AgentData>,
    Json(payload): Json<BulkIgnoreStatusRequest>,
) -> Response {
    let mut results = Vec::new();

    for item in payload.items {
        // Use the same logic as the single ignore status check
        let ignored = if item.folder_path.is_empty() {
            false
        } else {
            // Find the category by matching the category ID
            match data.categories.iter().find(|c| c.id == item.category_id) {
                Some(category) => {
                    let category_base_path = build_category_base_path(&data.agent, category);

                    // Check if the folder path is ignored
                    filesystem::is_path_ignored(&category_base_path, &item.folder_path)
                }
                None => false, // Invalid category
            }
        };

        results.push(BulkIgnoreStatusItem {
            category_id: item.category_id,
            folder_path: item.folder_path,
            ignored,
        });
    }

    (
        StatusCode::OK,
        Json(BulkIgnoreStatusResponse { items: results }),
    )
        .into_response()
}

// POST delete
// Deletes a folder path from the filesystem
pub async fn post_delete(
    State(data): State<AgentData>,
    Json(payload): Json<DeleteRequest>,
) -> Response {
    tracing::info!(
        "Processing delete request for category: '{}', folder_path: {:?}",
        payload.category_id,
        payload.folder_path
    );

    // Validate folder path is not empty
    if payload.folder_path.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(DeleteResponse {
                success: false,
                message: "Folder path cannot be empty".to_string(),
                deleted_path: None,
            }),
        )
            .into_response();
    }

    // Find the category by matching the category ID
    let category = match data.categories.iter().find(|c| c.id == payload.category_id) {
        Some(cat) => cat,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(DeleteResponse {
                    success: false,
                    message: format!("Category ID '{}' not found", payload.category_id),
                    deleted_path: None,
                }),
            )
                .into_response();
        }
    };

    let category_base_path = build_category_base_path(&data.agent, category);

    // Delete from filesystem
    match filesystem::delete_from_filesystem(
        &category_base_path,
        &payload.folder_path,
        &category.name,
    ) {
        filesystem::DeleteResult::Success {
            deleted_path,
            message,
        } => (
            StatusCode::OK,
            Json(DeleteResponse {
                success: true,
                message,
                deleted_path: Some(deleted_path),
            }),
        )
            .into_response(),
        filesystem::DeleteResult::NotFound { requested_path } => (
            StatusCode::NOT_FOUND,
            Json(DeleteResponse {
                success: false,
                message: format!("Path '{}' not found", requested_path),
                deleted_path: None,
            }),
        )
            .into_response(),
        filesystem::DeleteResult::Error { message } => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(DeleteResponse {
                success: false,
                message,
                deleted_path: None,
            }),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use std::fs;
    use stignore_lib::{AgentConfig, AgentData, Category};
    use tempfile::TempDir;

    // Test constants
    const MOVIES_ID: &str = "movies";
    const NONEXISTENT_ID: &str = "nonexistent_id";

    struct TestDirectoryPaths {
        movie1: std::path::PathBuf,
        movie2: std::path::PathBuf,
        show1_season1: std::path::PathBuf,
        show1_season2: std::path::PathBuf,
        show2_season1: std::path::PathBuf,
        show3_season1: std::path::PathBuf,
        show3_season2: std::path::PathBuf,
        show3_season3: std::path::PathBuf,
    }

    fn create_directory_structure(temp_dir: &TempDir) -> TestDirectoryPaths {
        let movies_dir = temp_dir.path().join("movies");
        let tv_dir = temp_dir.path().join("tv");
        fs::create_dir_all(&movies_dir).unwrap();
        fs::create_dir_all(&tv_dir).unwrap();

        // Create Syncthing system files/folders that should be filtered out
        fs::write(movies_dir.join(".stfolder"), "").unwrap();
        fs::write(movies_dir.join(".stignore"), "").unwrap(); // Empty .stignore for other tests
        fs::create_dir_all(movies_dir.join(".stversions")).unwrap();
        fs::write(tv_dir.join(".stfolder"), "").unwrap();

        // Create Movies structure
        let movie1_dir = movies_dir.join("Movie 1 (2023)");
        let movie2_dir = movies_dir.join("Movie 2 (2024)");
        fs::create_dir_all(&movie1_dir).unwrap();
        fs::create_dir_all(&movie2_dir).unwrap();

        // Create TV show structure
        let shows = [
            ("Show 1 (2021)", vec!["Season 1", "Season 2"]),
            ("Show 2 (2022)", vec!["Season 1"]),
            ("Show 3 (2023)", vec!["Season 1", "Season 2", "Season 3"]),
        ];

        for (show_name, seasons) in shows {
            let show_dir = tv_dir.join(show_name);
            fs::create_dir_all(&show_dir).unwrap();

            for season in seasons {
                fs::create_dir_all(show_dir.join(season)).unwrap();
            }
        }

        TestDirectoryPaths {
            movie1: movie1_dir,
            movie2: movie2_dir,
            show1_season1: tv_dir.join("Show 1 (2021)").join("Season 1"),
            show1_season2: tv_dir.join("Show 1 (2021)").join("Season 2"),
            show2_season1: tv_dir.join("Show 2 (2022)").join("Season 1"),
            show3_season1: tv_dir.join("Show 3 (2023)").join("Season 1"),
            show3_season2: tv_dir.join("Show 3 (2023)").join("Season 2"),
            show3_season3: tv_dir.join("Show 3 (2023)").join("Season 3"),
        }
    }

    fn create_test_files(paths: &TestDirectoryPaths) {
        // Create movie files
        fs::write(
            paths.movie1.join("Movie 1 (2023).mkv"),
            "test movie 1 content",
        )
        .unwrap();
        fs::write(
            paths.movie2.join("Movie 2 (2024).mp4"),
            "test movie 2 content",
        )
        .unwrap();

        // Create TV show files
        let tv_episodes = [
            (
                &paths.show1_season1,
                vec!["S01E01 - Ep 1.mkv", "S01E02 - Ep 2.mkv"],
            ),
            (
                &paths.show1_season2,
                vec![
                    "S02E01 - Ep 1.mkv",
                    "S02E02 - Ep 2.mkv",
                    "S02E03 - Ep 3.mkv",
                ],
            ),
            (&paths.show2_season1, vec!["S01E01 - Ep 1.mkv"]),
            (&paths.show3_season1, vec!["S01E01 - Ep 1.mkv"]),
            (
                &paths.show3_season2,
                vec!["S02E01 - Ep 1.mkv", "S02E02 - Ep 2.mkv"],
            ),
            (
                &paths.show3_season3,
                vec![
                    "S03E01 - Ep 1.mkv",
                    "S03E02 - Ep 2.mkv",
                    "S03E03 - Ep 3.mkv",
                ],
            ),
        ];

        for (season_path, episodes) in tv_episodes {
            for episode in episodes {
                fs::write(season_path.join(episode), "test episode content").unwrap();
            }
        }
    }

    fn create_test_data() -> (AgentData, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_string_lossy().to_string();

        let paths = create_directory_structure(&temp_dir);
        create_test_files(&paths);

        let data = AgentData {
            agent: AgentConfig {
                name: "Test Agent".to_string(),
                port: 3000,
                base_path,
                api_key: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            },
            categories: vec![
                Category {
                    id: "movies".to_string(),
                    name: "Movies".to_string(),
                    relative_path: "movies".to_string(),
                },
                Category {
                    id: "tv".to_string(),
                    name: "TV Shows".to_string(),
                    relative_path: "tv".to_string(),
                },
            ],
        };

        (data, temp_dir)
    }

    fn create_test_router(data: AgentData) -> Router {
        Router::new()
            .route("/", axum::routing::get(help))
            .route("/api/v1/categories", axum::routing::get(category_list))
            .route("/api/v1/categories/{id}", axum::routing::get(category_info))
            .route("/api/v1/items", axum::routing::post(post_item_info))
            .route("/api/v1/ignore", axum::routing::post(post_ignore))
            .route(
                "/api/v1/ignore-status",
                axum::routing::post(post_ignore_status),
            )
            .route(
                "/api/v1/ignore-status-bulk",
                axum::routing::post(post_ignore_status_bulk),
            )
            .route("/api/v1/delete", axum::routing::post(post_delete))
            .layer(axum::middleware::from_fn_with_state(
                data.clone(),
                crate::auth_middleware,
            ))
            .with_state(data)
    }

    async fn setup_test_server() -> (TestServer, TempDir) {
        let (data, temp_dir) = create_test_data();
        let app = create_test_router(data);
        let server = TestServer::new(app).unwrap();
        (server, temp_dir)
    }

    // Helper endpoint tests
    #[tokio::test]
    async fn test_help_endpoint() {
        let (server, _temp_dir) = setup_test_server().await;

        let response = server.get("/").await;
        response.assert_status(StatusCode::OK);
        let text = response.text();
        assert!(text.contains("documentation"));
    }

    #[tokio::test]
    async fn test_unauthorized_access() {
        let (server, _temp_dir) = setup_test_server().await;

        // Test without API key
        let response = server.get("/api/v1/categories").await;
        response.assert_status(StatusCode::UNAUTHORIZED);

        // Test with wrong API key
        let response = server
            .get("/api/v1/categories")
            .add_header("X-API-Key", "wrong-key")
            .await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    // Category endpoint tests
    #[tokio::test]
    async fn test_category_list() {
        let (server, _temp_dir) = setup_test_server().await;

        let response = server
            .get("/api/v1/categories")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .await;
        response.assert_status(StatusCode::OK);

        let json: CategoryListingResponse = response.json();
        assert_eq!(json.items.len(), 2);

        // Check for movies category
        let movies_category = json.items.iter().find(|item| item.id == "movies").unwrap();
        assert_eq!(movies_category.name, "Movies");

        // Check for tv category
        let tv_category = json.items.iter().find(|item| item.id == "tv").unwrap();
        assert_eq!(tv_category.name, "TV Shows");
    }

    #[tokio::test]
    async fn test_category_list_excludes_syncthing_files() {
        let (server, _temp_dir) = setup_test_server().await;

        let response = server
            .get("/api/v1/categories")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .await;
        response.assert_status(StatusCode::OK);

        let json: CategoryListingResponse = response.json();

        // Check movies category items
        let movies_category = json.items.iter().find(|item| item.id == "movies").unwrap();
        let movie_names: Vec<&String> = movies_category
            .items
            .iter()
            .map(|item| &item.name)
            .collect();

        // Verify regular movies are present
        assert!(movie_names.contains(&&"Movie 1 (2023)".to_string()));
        assert!(movie_names.contains(&&"Movie 2 (2024)".to_string()));

        // Verify Syncthing system files are excluded
        assert!(!movie_names.contains(&&".stfolder".to_string()));
        assert!(!movie_names.contains(&&".stignore".to_string()));
        assert!(!movie_names.contains(&&".stversions".to_string()));

        // Check TV category items
        let tv_category = json.items.iter().find(|item| item.id == "tv").unwrap();
        let tv_names: Vec<&String> = tv_category.items.iter().map(|item| &item.name).collect();

        // Verify regular TV shows are present
        assert!(tv_names.contains(&&"Show 1 (2021)".to_string()));
        assert!(tv_names.contains(&&"Show 2 (2022)".to_string()));
        assert!(tv_names.contains(&&"Show 3 (2023)".to_string()));

        // Verify Syncthing system files are excluded from TV category too
        assert!(!tv_names.contains(&&".stfolder".to_string()));
    }

    #[tokio::test]
    async fn test_category_info_found() {
        let (server, _temp_dir) = setup_test_server().await;

        let response = server
            .get("/api/v1/categories/movies")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .await;
        response.assert_status(StatusCode::OK);

        let json: CategoryInfoResponse = response.json();
        assert_eq!(json.name, "Movies");
        assert_eq!(json.items.len(), 2); // Movie 1 (2023) and Movie 2 (2024) directories
    }

    #[tokio::test]
    async fn test_category_info_not_found() {
        let (server, _temp_dir) = setup_test_server().await;

        let response = server
            .get("/api/v1/categories/nonexistent")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .await;
        response.assert_status(StatusCode::NOT_FOUND);

        let json: NotFoundResponse = response.json();
        assert!(json.message.contains("Category ID nonexistent not found"));
    }

    // Item endpoint tests (POST)
    #[tokio::test]
    async fn test_post_item_info_success() {
        let (server, _temp_dir) = setup_test_server().await;

        let request_body = ItemInfoRequest {
            item_path: vec![MOVIES_ID.to_string()],
        };

        let response = server
            .post("/api/v1/items")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::OK);

        let json: ItemInfoResponse = response.json();
        assert_eq!(json.item.id, "movies");
        assert_eq!(json.item.name, "Movies");
        assert_eq!(json.item.items.len(), 2);
    }

    #[tokio::test]
    async fn test_post_item_info_not_found() {
        let (server, _temp_dir) = setup_test_server().await;

        let request_body = ItemInfoRequest {
            item_path: vec![NONEXISTENT_ID.to_string()],
        };

        let response = server
            .post("/api/v1/items")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::NOT_FOUND);

        let json: NotFoundResponse = response.json();
        assert!(json.message.contains("Category ID"));
    }

    #[tokio::test]
    async fn test_post_item_info_invalid_category() {
        let (server, _temp_dir) = setup_test_server().await;

        let request_body = ItemInfoRequest {
            item_path: vec!["invalid_category".to_string()],
        };

        let response = server
            .post("/api/v1/items")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::NOT_FOUND);

        let json: NotFoundResponse = response.json();
        assert!(
            json.message
                .contains("Category ID 'invalid_category' not found")
        );
    }

    // Ignore endpoint tests
    #[tokio::test]
    async fn test_post_ignore_success() {
        let (server, temp_dir) = setup_test_server().await;

        // Test ignoring an existing folder

        let request_body = IgnoreRequest {
            category_id: MOVIES_ID.to_string(),
            folder_path: vec!["Movie 1 (2023)".to_string()],
        };

        let response = server
            .post("/api/v1/ignore")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::OK);

        let json: IgnoreResponse = response.json();
        assert!(json.success);
        assert!(json.ignored_path.is_some());
        assert!(json.message.contains("Successfully added"));

        // Verify .stignore file was created
        let stignore_path = temp_dir.path().join("movies").join(".stignore");
        assert!(stignore_path.exists());
        let content = std::fs::read_to_string(&stignore_path).unwrap();
        assert!(!content.is_empty());
    }

    #[tokio::test]
    async fn test_post_ignore_already_ignored() {
        let (server, temp_dir) = setup_test_server().await;

        // Pre-create .stignore file with the movie directory
        let stignore_path = temp_dir.path().join("movies").join(".stignore");
        std::fs::write(&stignore_path, "Movie 1 (2023)\n").unwrap();

        let request_body = IgnoreRequest {
            category_id: MOVIES_ID.to_string(),
            folder_path: vec!["Movie 1 (2023)".to_string()],
        };

        let response = server
            .post("/api/v1/ignore")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::OK);

        let json: IgnoreResponse = response.json();
        assert!(json.success);
        assert!(json.message.contains("already ignored"));
    }

    #[tokio::test]
    async fn test_post_ignore_empty_path() {
        let (server, _temp_dir) = setup_test_server().await;

        // Try to ignore with empty folder path - should be rejected
        let request_body = IgnoreRequest {
            category_id: MOVIES_ID.to_string(),
            folder_path: vec![],
        };

        let response = server
            .post("/api/v1/ignore")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::BAD_REQUEST);

        let json: IgnoreResponse = response.json();
        assert!(!json.success);
        assert!(json.message.contains("Folder path cannot be empty"));
    }

    #[tokio::test]
    async fn test_post_ignore_invalid_category() {
        let (server, _temp_dir) = setup_test_server().await;

        let request_body = IgnoreRequest {
            category_id: NONEXISTENT_ID.to_string(),
            folder_path: vec!["Some Movie".to_string()],
        };

        let response = server
            .post("/api/v1/ignore")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::BAD_REQUEST);

        let json: IgnoreResponse = response.json();
        assert!(!json.success);
        assert!(
            json.message
                .contains("Category ID 'nonexistent_id' not found")
        );
    }

    #[tokio::test]
    async fn test_post_ignore_nonexistent_folder() {
        let (server, temp_dir) = setup_test_server().await;

        // Test ignoring a folder that doesn't exist on disk - this should work now!
        let request_body = IgnoreRequest {
            category_id: MOVIES_ID.to_string(),
            folder_path: vec!["Non-existent Movie (2025)".to_string()],
        };

        let response = server
            .post("/api/v1/ignore")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::OK);

        let json: IgnoreResponse = response.json();
        assert!(json.success);
        assert!(json.ignored_path.is_some());
        assert_eq!(json.ignored_path.unwrap(), "Non-existent Movie (2025)");
        assert!(json.message.contains("Successfully added"));

        // Verify .stignore file was created with the non-existent folder
        let stignore_path = temp_dir.path().join("movies").join(".stignore");
        assert!(stignore_path.exists());
        let content = std::fs::read_to_string(&stignore_path).unwrap();
        assert!(content.contains("Non-existent Movie (2025)"));
    }

    // Ignore status endpoint tests
    #[tokio::test]
    async fn test_post_ignore_status_not_ignored() {
        let (server, _temp_dir) = setup_test_server().await;

        let request_body = IgnoreStatusRequest {
            category_id: MOVIES_ID.to_string(),
            folder_path: vec!["Movie 1 (2023)".to_string()],
        };

        let response = server
            .post("/api/v1/ignore-status")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::OK);

        let json: IgnoreStatusResponse = response.json();
        assert!(!json.ignored);
    }

    #[tokio::test]
    async fn test_post_ignore_status_ignored() {
        let (server, temp_dir) = setup_test_server().await;

        // Pre-create .stignore file with the movie directory
        let stignore_path = temp_dir.path().join("movies").join(".stignore");
        std::fs::write(&stignore_path, "Movie 1 (2023)\n").unwrap();

        let request_body = IgnoreStatusRequest {
            category_id: MOVIES_ID.to_string(),
            folder_path: vec!["Movie 1 (2023)".to_string()],
        };

        let response = server
            .post("/api/v1/ignore-status")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::OK);

        let json: IgnoreStatusResponse = response.json();
        assert!(json.ignored);
    }

    #[tokio::test]
    async fn test_post_ignore_status_empty_path() {
        let (server, _temp_dir) = setup_test_server().await;

        let request_body = IgnoreStatusRequest {
            category_id: MOVIES_ID.to_string(),
            folder_path: vec![],
        };

        let response = server
            .post("/api/v1/ignore-status")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::BAD_REQUEST);

        let json: IgnoreStatusResponse = response.json();
        assert!(!json.ignored);
    }

    #[tokio::test]
    async fn test_post_ignore_status_invalid_category() {
        let (server, _temp_dir) = setup_test_server().await;

        let request_body = IgnoreStatusRequest {
            category_id: NONEXISTENT_ID.to_string(),
            folder_path: vec!["Some Movie".to_string()],
        };

        let response = server
            .post("/api/v1/ignore-status")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::BAD_REQUEST);

        let json: IgnoreStatusResponse = response.json();
        assert!(!json.ignored);
    }

    #[tokio::test]
    async fn test_post_ignore_status_missing_item_ignored() {
        let (server, temp_dir) = setup_test_server().await;

        // Pre-create .stignore file with a movie that doesn't exist on disk
        let stignore_path = temp_dir.path().join("movies").join(".stignore");
        std::fs::write(&stignore_path, "Non-existent Movie (2025)\n").unwrap();

        let request_body = IgnoreStatusRequest {
            category_id: MOVIES_ID.to_string(),
            folder_path: vec!["Non-existent Movie (2025)".to_string()],
        };

        let response = server
            .post("/api/v1/ignore-status")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::OK);

        let json: IgnoreStatusResponse = response.json();
        assert!(json.ignored);
    }

    #[tokio::test]
    async fn test_post_ignore_status_missing_item_not_ignored() {
        let (server, _temp_dir) = setup_test_server().await;

        // Check status of a non-existent movie that's not in .stignore
        let request_body = IgnoreStatusRequest {
            category_id: MOVIES_ID.to_string(),
            folder_path: vec!["Another Non-existent Movie (2026)".to_string()],
        };

        let response = server
            .post("/api/v1/ignore-status")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::OK);

        let json: IgnoreStatusResponse = response.json();
        assert!(!json.ignored);
    }

    #[tokio::test]
    async fn test_post_ignore_status_bulk() {
        let (server, temp_dir) = setup_test_server().await;

        // Pre-create .stignore file with one ignored item
        let stignore_path = temp_dir.path().join("movies").join(".stignore");
        std::fs::write(&stignore_path, "Movie 1 (2023)\n").unwrap();

        // Test with multiple items - some ignored, some not, some invalid
        let request_body = BulkIgnoreStatusRequest {
            items: vec![
                IgnoreStatusRequest {
                    category_id: MOVIES_ID.to_string(),
                    folder_path: vec!["Movie 1 (2023)".to_string()], // ignored
                },
                IgnoreStatusRequest {
                    category_id: MOVIES_ID.to_string(),
                    folder_path: vec!["Movie 2 (2023)".to_string()], // not ignored
                },
                IgnoreStatusRequest {
                    category_id: "invalid_category".to_string(),
                    folder_path: vec!["Any Movie".to_string()], // invalid category
                },
            ],
        };

        let response = server
            .post("/api/v1/ignore-status-bulk")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::OK);

        let json: BulkIgnoreStatusResponse = response.json();

        // Should return 3 items
        assert_eq!(json.items.len(), 3);

        // First item should be ignored
        assert_eq!(json.items[0].category_id, MOVIES_ID);
        assert_eq!(json.items[0].folder_path, vec!["Movie 1 (2023)"]);
        assert!(json.items[0].ignored);

        // Second item should not be ignored
        assert_eq!(json.items[1].category_id, MOVIES_ID);
        assert_eq!(json.items[1].folder_path, vec!["Movie 2 (2023)"]);
        assert!(!json.items[1].ignored);

        // Third item (invalid category) should not be ignored
        assert_eq!(json.items[2].category_id, "invalid_category");
        assert_eq!(json.items[2].folder_path, vec!["Any Movie"]);
        assert!(!json.items[2].ignored);
    }

    // Delete endpoint tests
    #[tokio::test]
    async fn test_post_delete_success() {
        let (server, _temp_dir) = setup_test_server().await;

        let request_body = DeleteRequest {
            category_id: MOVIES_ID.to_string(),
            folder_path: vec!["Movie 1 (2023)".to_string()],
        };

        let response = server
            .post("/api/v1/delete")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::OK);

        let json: DeleteResponse = response.json();
        assert!(json.success);
        assert!(json.deleted_path.is_some());
        assert_eq!(json.deleted_path.unwrap(), "Movie 1 (2023)");
        assert!(json.message.contains("Successfully deleted"));
    }

    #[tokio::test]
    async fn test_post_delete_not_found() {
        let (server, _temp_dir) = setup_test_server().await;

        let request_body = DeleteRequest {
            category_id: MOVIES_ID.to_string(),
            folder_path: vec!["Non-existent Movie (2025)".to_string()],
        };

        let response = server
            .post("/api/v1/delete")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::NOT_FOUND);

        let json: DeleteResponse = response.json();
        assert!(!json.success);
        assert!(json.message.contains("not found"));
        assert!(json.deleted_path.is_none());
    }

    #[tokio::test]
    async fn test_post_delete_empty_path() {
        let (server, _temp_dir) = setup_test_server().await;

        let request_body = DeleteRequest {
            category_id: MOVIES_ID.to_string(),
            folder_path: vec![],
        };

        let response = server
            .post("/api/v1/delete")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::BAD_REQUEST);

        let json: DeleteResponse = response.json();
        assert!(!json.success);
        assert!(json.message.contains("Folder path cannot be empty"));
        assert!(json.deleted_path.is_none());
    }

    #[tokio::test]
    async fn test_post_delete_invalid_category() {
        let (server, _temp_dir) = setup_test_server().await;

        let request_body = DeleteRequest {
            category_id: NONEXISTENT_ID.to_string(),
            folder_path: vec!["Some Movie".to_string()],
        };

        let response = server
            .post("/api/v1/delete")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::BAD_REQUEST);

        let json: DeleteResponse = response.json();
        assert!(!json.success);
        assert!(
            json.message
                .contains("Category ID 'nonexistent_id' not found")
        );
        assert!(json.deleted_path.is_none());
    }

    #[tokio::test]
    async fn test_post_delete_file() {
        let (server, temp_dir) = setup_test_server().await;

        // Create a test file to delete
        let test_file_path = temp_dir.path().join("movies").join("test-file.txt");
        std::fs::write(&test_file_path, "test content").unwrap();

        let request_body = DeleteRequest {
            category_id: MOVIES_ID.to_string(),
            folder_path: vec!["test-file.txt".to_string()],
        };

        let response = server
            .post("/api/v1/delete")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::OK);

        let json: DeleteResponse = response.json();
        assert!(json.success);
        assert!(json.deleted_path.is_some());
        assert_eq!(json.deleted_path.unwrap(), "test-file.txt");
        assert!(json.message.contains("Successfully deleted"));

        // Verify file was actually deleted
        assert!(!test_file_path.exists());
    }

    #[tokio::test]
    async fn test_syncthing_system_files_filtered() {
        let (server, _temp_dir) = setup_test_server().await;

        // Test that Syncthing system files (.st*) are filtered out from listing
        let request_body = ItemInfoRequest {
            item_path: vec![MOVIES_ID.to_string()],
        };

        let response = server
            .post("/api/v1/items")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;
        response.assert_status(StatusCode::OK);

        let json: ItemInfoResponse = response.json();

        let items = json.item.items;

        // Verify that no Syncthing system files are returned
        for item in &items {
            assert!(
                !item.name.starts_with(".st"),
                "Syncthing system file '{}' should be filtered out",
                item.name
            );
        }

        // Verify we still get regular movies (Movie 1 and Movie 2)
        let movie_names: Vec<&String> = items.iter().map(|item| &item.name).collect();
        assert!(movie_names.contains(&&"Movie 1 (2023)".to_string()));
        assert!(movie_names.contains(&&"Movie 2 (2024)".to_string()));

        // Verify specific system files are not present
        assert!(!movie_names.contains(&&".stfolder".to_string()));
        assert!(!movie_names.contains(&&".stignore".to_string()));
        assert!(!movie_names.contains(&&".stversions".to_string()));
    }

    #[tokio::test]
    async fn test_post_ignore_with_corrupted_stignore_file() {
        let (server, temp_dir) = setup_test_server().await;

        // Create a .stignore file with invalid UTF-8 content
        let stignore_path = temp_dir.path().join("movies").join(".stignore");
        let invalid_utf8: Vec<u8> = vec![
            b'M', b'o', b'v', b'i', b'e', b' ', b'1', b'\n', 0xFF, 0xFE, 0xFD, b'\n',
        ];
        std::fs::write(&stignore_path, invalid_utf8).unwrap();

        // Try to add another movie to the ignore list
        let request_body = IgnoreRequest {
            category_id: MOVIES_ID.to_string(),
            folder_path: vec!["Movie 2 (2024)".to_string()],
        };

        let response = server
            .post("/api/v1/ignore")
            .add_header("X-API-Key", "550e8400-e29b-41d4-a716-446655440000")
            .json(&request_body)
            .await;

        // Should return error status
        response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);

        let json: IgnoreResponse = response.json();
        assert!(!json.success);
        assert!(
            json.message
                .contains("Failed to read existing .stignore file")
        );
        assert!(json.message.contains("encoding") || json.message.contains("UTF"));

        // Verify the corrupted file still exists and wasn't replaced
        assert!(stignore_path.exists());
        let file_content = std::fs::read(&stignore_path).unwrap();
        assert_eq!(
            file_content,
            vec![
                b'M', b'o', b'v', b'i', b'e', b' ', b'1', b'\n', 0xFF, 0xFE, 0xFD, b'\n',
            ]
        );
    }
}
