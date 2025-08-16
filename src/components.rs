use axum::{
    Json, Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::{get, post},
};

use crate::agents;
use crate::types::ItemGroup;
use axum_template::{Key, RenderHtml};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::AppState;

fn sanitize_id(id: &str) -> String {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(id.as_bytes());
    format!("id_{}", encoded)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/navbar.html", get(navbar))
        .route("/itemlist.html", get(itemlist))
        .route("/infopanel.html", post(infopanel))
        .route("/agent-modal.html", get(agent_modal))
        .route("/ignore", post(ignore_item))
        .route("/delete", post(delete_item))
}

async fn navbar(State(state): State<AppState>) -> impl IntoResponse {
    let mut context = state.context.clone();
    context.insert("page_title", "Index");
    context.insert("message", "Welcome to stignore-manager.");

    RenderHtml(
        Key("components/navbar.html".to_string()),
        state.engine,
        context.into_json(),
    )
}

#[derive(Serialize, Debug, Clone)]
struct ItemGroupWithFlags {
    pub id: String,
    pub safe_id: String,
    pub name: String,
    pub size_kb: u64,
    pub items: Vec<ItemGroupWithFlags>,
    pub leaf: bool,
    pub copy_count: u8,
    pub has_insufficient_copies: bool,
}

impl From<&ItemGroup> for ItemGroupWithFlags {
    fn from(item: &ItemGroup) -> Self {
        Self {
            id: item.id.clone(),
            safe_id: sanitize_id(&item.id),
            name: item.name.clone(),
            size_kb: item.size_kb,
            items: vec![], // Will be filled separately
            leaf: item.leaf,
            copy_count: item.copy_count,
            has_insufficient_copies: false, // Will be set separately
        }
    }
}

fn convert_item_with_flags(item: &ItemGroup, minimum_copies: u8) -> ItemGroupWithFlags {
    let mut converted = ItemGroupWithFlags::from(item);
    converted.has_insufficient_copies = item.has_insufficient_copies(minimum_copies);
    converted.items = item
        .items
        .iter()
        .map(|child| convert_item_with_flags(child, minimum_copies))
        .collect();
    converted
}

async fn itemlist(State(state): State<AppState>) -> impl IntoResponse {
    let mut context = state.context.clone();

    match agents::list_categories(state.config.agents).await {
        Ok(response) => {
            let mut sorted_items = response.items;
            sorted_items.sort_by(|a, b| a.name.cmp(&b.name));

            // Convert to ItemGroupWithFlags with has_insufficient_copies field
            let items_with_flags: Vec<ItemGroupWithFlags> = sorted_items
                .iter()
                .map(|item| convert_item_with_flags(item, state.config.manager.minimum_copies))
                .collect();

            context.insert("items", &items_with_flags);
        }
        Err(_) => {
            let items: Vec<ItemGroupWithFlags> = vec![];
            context.insert("items", &items);
        }
    }

    context.insert("minimum_copies", &state.config.manager.minimum_copies);

    RenderHtml(
        Key("components/itemlist.html".to_string()),
        state.engine,
        context.into_json(),
    )
}

#[derive(Deserialize, Debug)]
struct InfoPanelRequest {
    hierarchy_names: Vec<String>,
    item_path: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct AgentModalQuery {
    agent_name: String,
    item_path: String,
}

#[derive(Deserialize, Debug)]
struct IgnoreItemRequest {
    agent_name: String,
    item_path: Vec<String>,
    hierarchy_names: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct DeleteItemRequest {
    agent_name: String,
    item_path: Vec<String>,
    hierarchy_names: Vec<String>,
}

#[derive(Serialize, Debug)]
struct IgnoreItemResponse {
    success: bool,
    message: String,
}

#[derive(Serialize, Debug)]
struct DeleteItemResponse {
    success: bool,
    message: String,
}

#[derive(Serialize, Debug)]
struct AgentIgnoreRequest {
    category_id: String,
    folder_path: Vec<String>,
}

#[derive(Serialize, Debug)]
struct AgentDeleteRequest {
    category_id: String,
    folder_path: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct AgentIgnoreResponse {
    success: bool,
    message: String,
    #[allow(dead_code)]
    ignored_path: Option<String>,
}

#[derive(Deserialize, Debug)]
struct AgentDeleteResponse {
    success: bool,
    message: String,
    #[allow(dead_code)]
    deleted_path: Option<String>,
}

#[derive(Serialize, Debug)]
struct AgentIgnoreStatusRequest {
    category_id: String,
    folder_path: Vec<String>,
}

#[derive(Serialize, Debug)]
struct AgentBulkIgnoreStatusRequest {
    items: Vec<AgentIgnoreStatusRequest>,
}

#[derive(Deserialize, Debug)]
struct AgentBulkIgnoreStatusItem {
    #[allow(dead_code)]
    category_id: String,
    #[allow(dead_code)]
    folder_path: Vec<String>,
    ignored: bool,
}

#[derive(Deserialize, Debug)]
struct AgentBulkIgnoreStatusResponse {
    items: Vec<AgentBulkIgnoreStatusItem>,
}

#[derive(Serialize, Debug)]
struct AgentItemWithStatus {
    agent: crate::config::Agent,
    item: ItemGroup,
    sync_status: String,
    ignored: bool,
}

#[derive(Serialize, Debug)]
struct MergedItem {
    name: String,
    present: bool,
    size_kb: u64,
    items: usize,
    is_partial: bool,
}

fn collect_all_item_ids(item_group: &ItemGroup) -> HashSet<String> {
    let mut ids = HashSet::new();

    // Add this item's id if it is not empty
    if !item_group.id.is_empty() {
        ids.insert(item_group.id.clone());
    }

    // Recursively collect ids from all sub-items
    for item in &item_group.items {
        ids.extend(collect_all_item_ids(item));
    }

    ids
}

async fn check_ignored_status_bulk(
    agent_items: &[(crate::config::Agent, ItemGroup)],
    item_path: &[String],
    hierarchy_names: &[String],
) -> std::collections::HashMap<String, bool> {
    let mut results = std::collections::HashMap::new();
    let client = reqwest::Client::new();

    // Filter out empty strings from item_path (same as ignore request)
    let filtered_item_path: Vec<String> = item_path
        .iter()
        .filter(|i| !i.is_empty())
        .cloned()
        .collect();

    // Filter out empty strings from hierarchy_names (same as item_path filtering)
    let filtered_hierarchy_names: Vec<String> = hierarchy_names
        .iter()
        .filter(|name| !name.is_empty())
        .cloned()
        .collect();

    // Extract category_id (first non-empty item from item_path) and folder_path (from hierarchy_names)
    let (category_id, folder_path) = if filtered_item_path.is_empty() {
        return results; // Return empty results if no valid path
    } else {
        let category_id = filtered_item_path[0].clone();
        // Use hierarchy_names for folder path (skip the first one which is the category)
        let folder_path = if filtered_hierarchy_names.len() > 1 {
            filtered_hierarchy_names[1..].to_vec()
        } else {
            vec![]
        };

        (category_id, folder_path)
    };

    // Create one bulk request per agent
    for (agent, _) in agent_items {
        let url = format!("http://{}/api/v1/ignore-status-bulk", agent.hostname);

        let bulk_request = AgentBulkIgnoreStatusRequest {
            items: vec![AgentIgnoreStatusRequest {
                category_id: category_id.clone(),
                folder_path: folder_path.clone(),
            }],
        };

        match client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&bulk_request)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<AgentBulkIgnoreStatusResponse>().await {
                        Ok(bulk_response) => {
                            // For this simple case, we only sent one item so take the first result
                            if let Some(first_result) = bulk_response.items.first() {
                                results.insert(agent.name.clone(), first_result.ignored);
                            } else {
                                results.insert(agent.name.clone(), false);
                            }
                        }
                        Err(_) => {
                            results.insert(agent.name.clone(), false);
                        }
                    }
                } else {
                    results.insert(agent.name.clone(), false);
                }
            }
            Err(_) => {
                results.insert(agent.name.clone(), false);
            }
        }
    }

    results
}

async fn calculate_sync_status(
    agent_items: &[(crate::config::Agent, ItemGroup)],
    item_path: &[String],
    hierarchy_names: &[String],
) -> Vec<AgentItemWithStatus> {
    // Collect all unique item IDs from all agents (including nested items)
    let mut all_item_ids: HashSet<String> = HashSet::new();
    for (_, item_group) in agent_items {
        all_item_ids.extend(collect_all_item_ids(item_group));
    }

    // Get ignore status for all agents in bulk
    let ignore_status_results =
        check_ignored_status_bulk(agent_items, item_path, hierarchy_names).await;

    let mut result = Vec::new();

    for (agent, item_group) in agent_items {
        let agent_item_ids = collect_all_item_ids(item_group);

        let sync_status = if item_group.size_kb == 0 {
            "Missing".to_string()
        } else if agent_item_ids == all_item_ids {
            "In Sync".to_string()
        } else if agent_item_ids.is_empty() {
            "Empty".to_string()
        } else {
            "Partial".to_string()
        };

        // Get the ignore status from our bulk results
        let ignored = ignore_status_results
            .get(&agent.name)
            .copied()
            .unwrap_or(false);

        result.push(AgentItemWithStatus {
            agent: agent.clone(),
            item: item_group.clone(),
            sync_status,
            ignored,
        });
    }

    result
}

async fn infopanel(
    State(state): State<AppState>,
    Json(payload): Json<InfoPanelRequest>,
) -> impl IntoResponse {
    let mut context = state.context.clone();

    let item_path: Vec<&str> = payload
        .item_path
        .iter()
        .filter(|i| !i.is_empty())
        .map(AsRef::as_ref)
        .collect();

    match agents::item_info(state.config.agents, item_path).await {
        Ok(response) => {
            let agent_items_with_status = calculate_sync_status(
                &response.agent_items,
                &payload.item_path,
                &payload.hierarchy_names,
            )
            .await;

            context.insert("item", &response.item);
            context.insert("agent_items", &agent_items_with_status);
            context.insert("parent_names", &payload.hierarchy_names[1..]);
            context.insert("item_path", &payload.item_path);
            context.insert("hierarchy_names", &payload.hierarchy_names);
        }
        Err(_) => {
            // Insert empty defaults to prevent template errors
            context.insert("agent_items", &Vec::<()>::new());
            context.insert("parent_names", &Vec::<String>::new());
            context.insert("item_path", &Vec::<String>::new());
            context.insert("hierarchy_names", &Vec::<String>::new());
        }
    }

    RenderHtml(
        Key("components/infopanel.html".to_string()),
        state.engine,
        context.into_json(),
    )
}

async fn agent_modal(
    State(state): State<AppState>,
    Query(query): Query<AgentModalQuery>,
) -> impl IntoResponse {
    let mut context = state.context.clone();

    // Parse the item_path (Axum already URL-decodes query parameters)
    let item_path_parts: Vec<&str> = if query.item_path.is_empty() {
        vec![]
    } else {
        let parts: Vec<&str> = query
            .item_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        parts
    };

    match agents::item_info(state.config.agents, item_path_parts.clone()).await {
        Ok(response) => {
            // Calculate sync status for all agents first
            let item_path_vec: Vec<String> =
                item_path_parts.iter().map(|s| s.to_string()).collect();
            // For agent modal, we don't have hierarchy_names, so create empty vector
            // The ignore status check might not work correctly here
            let empty_hierarchy_names: Vec<String> = vec![];
            let agent_items_with_status = calculate_sync_status(
                &response.agent_items,
                &item_path_vec,
                &empty_hierarchy_names,
            )
            .await;

            // Find the specific agent's data
            if let Some(agent_item_with_status) = agent_items_with_status
                .iter()
                .find(|a| a.agent.name == query.agent_name)
            {
                // Collect all unique item IDs and find max sizes
                let mut all_item_ids = std::collections::HashSet::new();
                let mut max_sizes: std::collections::HashMap<String, u64> =
                    std::collections::HashMap::new();

                for agent_with_status in &agent_items_with_status {
                    for item in &agent_with_status.item.items {
                        all_item_ids.insert(item.name.clone());
                        max_sizes
                            .entry(item.name.clone())
                            .and_modify(|max| *max = (*max).max(item.size_kb))
                            .or_insert(item.size_kb);
                    }
                }

                // Create merged items showing availability across agents
                let mut merged_items = Vec::new();
                for item_name in all_item_ids {
                    let current_agent_item = agent_item_with_status
                        .item
                        .items
                        .iter()
                        .find(|i| i.name == item_name);

                    let current_size = current_agent_item.map(|i| i.size_kb).unwrap_or(0);
                    let max_size = max_sizes.get(&item_name).unwrap_or(&0);
                    let is_partial =
                        current_agent_item.is_some() && current_size < *max_size && *max_size > 0;

                    merged_items.push(MergedItem {
                        name: item_name.to_string(),
                        present: current_agent_item.is_some(),
                        size_kb: current_size,
                        items: current_agent_item.map(|i| i.items.len()).unwrap_or(0),
                        is_partial,
                    });
                }

                // Sort merged items alphabetically
                merged_items.sort_by(|a, b| a.name.cmp(&b.name));

                context.insert("agent", &agent_item_with_status.agent);
                context.insert("agent_item", &agent_item_with_status);
                context.insert("merged_items", &merged_items);
                context.insert("item_path", &item_path_parts);
            } else {
                // Agent not found, insert empty data
                let error_msg = format!("Agent '{}' not found", query.agent_name);
                context.insert("error", &error_msg);
            }
        }
        Err(e) => {
            let error_msg = format!("Error loading agent data: {}", e);
            context.insert("error", &error_msg);
        }
    }

    RenderHtml(
        Key("components/agent-modal.html".to_string()),
        state.engine,
        context.into_json(),
    )
}

async fn ignore_item(
    State(state): State<AppState>,
    Json(payload): Json<IgnoreItemRequest>,
) -> impl IntoResponse {
    // Find the agent by name
    let agent = match state
        .config
        .agents
        .iter()
        .find(|a| a.name == payload.agent_name)
    {
        Some(agent) => agent,
        None => {
            return Json(IgnoreItemResponse {
                success: false,
                message: format!("Agent '{}' not found", payload.agent_name),
            });
        }
    };

    // Filter out empty strings from item_path (same as infopanel does)
    let filtered_item_path: Vec<String> = payload
        .item_path
        .iter()
        .filter(|i| !i.is_empty())
        .cloned()
        .collect();

    // Filter out empty strings from hierarchy_names (same as item_path filtering)
    let filtered_hierarchy_names: Vec<String> = payload
        .hierarchy_names
        .iter()
        .filter(|name| !name.is_empty())
        .cloned()
        .collect();

    // Extract category_id (first non-empty item from item_path) and folder_path (from hierarchy_names)
    let (category_id, folder_path) = if filtered_item_path.is_empty() {
        return Json(IgnoreItemResponse {
            success: false,
            message: "No valid path provided".to_string(),
        });
    } else {
        let category_id = filtered_item_path[0].clone();
        // Use hierarchy_names for folder path (skip the first one which is the category)
        let folder_path = if filtered_hierarchy_names.len() > 1 {
            filtered_hierarchy_names[1..].to_vec()
        } else {
            vec![]
        };

        (category_id, folder_path)
    };

    // Build the ignore request for the agent
    let ignore_request = AgentIgnoreRequest {
        category_id,
        folder_path,
    };

    // Send the ignore request to the agent
    let client = reqwest::Client::new();
    let url = format!("http://{}/api/v1/ignore", agent.hostname);

    match client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&ignore_request)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();

            match response.json::<AgentIgnoreResponse>().await {
                Ok(agent_response) => {
                    if status.is_success() && agent_response.success {
                        Json(IgnoreItemResponse {
                            success: true,
                            message: format!("Successfully ignored item on {}", agent.name),
                        })
                    } else {
                        Json(IgnoreItemResponse {
                            success: false,
                            message: format!("Agent error: {}", agent_response.message),
                        })
                    }
                }
                Err(parse_error) => Json(IgnoreItemResponse {
                    success: false,
                    message: format!("Failed to parse agent response: {}", parse_error),
                }),
            }
        }
        Err(e) => Json(IgnoreItemResponse {
            success: false,
            message: format!("Failed to connect to agent: {}", e),
        }),
    }
}

async fn delete_item(
    State(state): State<AppState>,
    Json(payload): Json<DeleteItemRequest>,
) -> impl IntoResponse {
    // Find the agent by name
    let agent = match state
        .config
        .agents
        .iter()
        .find(|a| a.name == payload.agent_name)
    {
        Some(agent) => agent,
        None => {
            return Json(DeleteItemResponse {
                success: false,
                message: format!("Agent '{}' not found", payload.agent_name),
            });
        }
    };

    // Filter out empty strings from item_path (same as infopanel does)
    let filtered_item_path: Vec<String> = payload
        .item_path
        .iter()
        .filter(|i| !i.is_empty())
        .cloned()
        .collect();

    // Filter out empty strings from hierarchy_names (same as item_path filtering)
    let filtered_hierarchy_names: Vec<String> = payload
        .hierarchy_names
        .iter()
        .filter(|name| !name.is_empty())
        .cloned()
        .collect();

    // Extract category_id (first non-empty item from item_path) and folder_path (from hierarchy_names)
    let (category_id, folder_path) = if filtered_item_path.is_empty() {
        return Json(DeleteItemResponse {
            success: false,
            message: "No valid path provided".to_string(),
        });
    } else {
        let category_id = filtered_item_path[0].clone();
        // Use hierarchy_names for folder path (skip the first one which is the category)
        let folder_path = if filtered_hierarchy_names.len() > 1 {
            filtered_hierarchy_names[1..].to_vec()
        } else {
            vec![]
        };

        (category_id, folder_path)
    };

    // Build the delete request for the agent
    let delete_request = AgentDeleteRequest {
        category_id,
        folder_path,
    };

    // Send the delete request to the agent
    let client = reqwest::Client::new();
    let url = format!("http://{}/api/v1/delete", agent.hostname);

    match client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&delete_request)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();

            match response.json::<AgentDeleteResponse>().await {
                Ok(agent_response) => {
                    if status.is_success() && agent_response.success {
                        Json(DeleteItemResponse {
                            success: true,
                            message: format!("Successfully deleted item on {}", agent.name),
                        })
                    } else {
                        Json(DeleteItemResponse {
                            success: false,
                            message: format!("Agent error: {}", agent_response.message),
                        })
                    }
                }
                Err(parse_error) => Json(DeleteItemResponse {
                    success: false,
                    message: format!("Failed to parse agent response: {}", parse_error),
                }),
            }
        }
        Err(e) => Json(DeleteItemResponse {
            success: false,
            message: format!("Failed to connect to agent: {}", e),
        }),
    }
}
