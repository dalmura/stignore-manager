use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use crate::agents;
use axum_template::{Key, RenderHtml};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use stignore_lib::*;

use super::AppState;

fn sanitize_id(id: &str) -> String {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(id.as_bytes());
    format!("id_{}", encoded)
}

fn unsanitize_id(safe_id: &str) -> Result<String, base64::DecodeError> {
    use base64::Engine;
    let id_part = safe_id.strip_prefix("id_").unwrap_or(safe_id);
    let decoded_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(id_part)?;
    Ok(String::from_utf8_lossy(&decoded_bytes).to_string())
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/itemlist.html", get(itemlist))
        .route("/dynamic-items.html", get(dynamic_items))
        .route("/infopanel.html", post(infopanel))
        .route("/agent-modal.html", post(agent_modal))
        .route("/ignore", post(ignore_item))
        .route("/delete", post(delete_item))
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

    let response = agents::list_categories(&state.agent_client, state.config.agents).await;
    // Items are already sorted by agents::list_categories

    // Convert to ItemGroupWithFlags with has_insufficient_copies field
    let items_with_flags: Vec<ItemGroupWithFlags> = response
        .items
        .iter()
        .map(|item| convert_item_with_flags(item, state.config.manager.minimum_copies))
        .collect();

    context.insert("items", &items_with_flags);

    context.insert("minimum_copies", &state.config.manager.minimum_copies);

    RenderHtml(
        Key("components/itemlist.html".to_string()),
        state.engine,
        context.into_json(),
    )
}

#[derive(Deserialize, Debug)]
struct InfoPanelRequest {
    item_path: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct AgentModalRequest {
    agent_name: String,
    item_path: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct DynamicItemsQuery {
    parent_id: String,
    parent_path: String,
    level: u8, // 2 or 3
}

#[derive(Deserialize, Debug)]
struct IgnoreItemRequest {
    agent_name: String,
    item_path: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct DeleteItemRequest {
    agent_name: String,
    item_path: Vec<String>,
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
struct AgentItemWithStatus {
    agent: Agent,
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
    agent_client: &crate::agent_client::AgentClient,
    agent_items: &[(Agent, ItemGroup)],
    item_path: &[String],
) -> std::collections::HashMap<String, bool> {
    let mut results = std::collections::HashMap::new();

    // Filter out empty strings from item_path
    let filtered_item_path: Vec<String> = item_path
        .iter()
        .filter(|i| !i.is_empty())
        .cloned()
        .collect();

    // Extract category_id (first item) and folder_path (remaining items) from item_path
    let (category_id, folder_path) = if filtered_item_path.is_empty() {
        return results; // Return empty results if no valid path
    } else {
        let category_id = filtered_item_path[0].clone();
        // Use remaining items from item_path for folder path (skip the first one which is the category)
        let folder_path = if filtered_item_path.len() > 1 {
            filtered_item_path[1..].to_vec()
        } else {
            vec![]
        };

        (category_id, folder_path)
    };

    // Create one bulk request per agent
    for (agent, _) in agent_items {
        let bulk_request = AgentBulkIgnoreStatusRequest {
            items: vec![AgentIgnoreStatusRequest {
                category_id: category_id.clone(),
                folder_path: folder_path.clone(),
            }],
        };

        match agent_client
            .check_ignore_status_bulk(agent, &bulk_request)
            .await
        {
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
    }

    results
}

async fn calculate_sync_status(
    agent_client: &crate::agent_client::AgentClient,
    agent_items: &[(Agent, ItemGroup)],
    item_path: &[String],
) -> Vec<AgentItemWithStatus> {
    // Collect all unique item IDs from all agents (including nested items)
    let mut all_item_ids: HashSet<String> = HashSet::new();
    for (_, item_group) in agent_items {
        all_item_ids.extend(collect_all_item_ids(item_group));
    }

    // Get ignore status for all agents in bulk
    let ignore_status_results =
        check_ignored_status_bulk(agent_client, agent_items, item_path).await;

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

    match agents::item_info(&state.agent_client, state.config.agents, item_path).await {
        Ok(response) => {
            let agent_items_with_status = calculate_sync_status(
                &state.agent_client,
                &response.agent_items,
                &payload.item_path,
            )
            .await;

            // Filter out empty strings from item_path for display as parent names
            let filtered_item_path: Vec<String> = payload
                .item_path
                .iter()
                .filter(|name| !name.is_empty())
                .cloned()
                .collect();

            context.insert("item", &response.item);
            context.insert("agent_items", &agent_items_with_status);
            context.insert("parent_names", &filtered_item_path);
            context.insert("item_path", &payload.item_path);
        }
        Err(_) => {
            // Insert empty defaults to prevent template errors
            context.insert("agent_items", &Vec::<()>::new());
            context.insert("parent_names", &Vec::<String>::new());
            context.insert("item_path", &Vec::<String>::new());
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
    Json(payload): Json<AgentModalRequest>,
) -> impl IntoResponse {
    let mut context = state.context.clone();

    // Filter out empty strings from item_path
    let item_path_parts: Vec<&str> = payload
        .item_path
        .iter()
        .filter(|i| !i.is_empty())
        .map(AsRef::as_ref)
        .collect();

    match agents::item_info(
        &state.agent_client,
        state.config.agents,
        item_path_parts.clone(),
    )
    .await
    {
        Ok(response) => {
            // Calculate sync status for all agents first
            let item_path_vec: Vec<String> =
                item_path_parts.iter().map(|s| s.to_string()).collect();
            // Calculate sync status for agent modal
            let agent_items_with_status =
                calculate_sync_status(&state.agent_client, &response.agent_items, &item_path_vec)
                    .await;

            // Find the specific agent's data
            if let Some(agent_item_with_status) = agent_items_with_status
                .iter()
                .find(|a| a.agent.name == payload.agent_name)
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
                let error_msg = format!("Agent '{}' not found", payload.agent_name);
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

    // Filter out empty strings from item_path
    let filtered_item_path: Vec<String> = payload
        .item_path
        .iter()
        .filter(|i| !i.is_empty())
        .cloned()
        .collect();

    // Extract category_id (first item) and folder_path (remaining items) from item_path
    let (category_id, folder_path) = if filtered_item_path.is_empty() {
        return Json(IgnoreItemResponse {
            success: false,
            message: "No valid path provided".to_string(),
        });
    } else {
        let category_id = filtered_item_path[0].clone();
        // Use remaining items from item_path for folder path (skip the first one which is the category)
        let folder_path = if filtered_item_path.len() > 1 {
            filtered_item_path[1..].to_vec()
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
    match state.agent_client.ignore_item(agent, &ignore_request).await {
        Ok(_) => Json(IgnoreItemResponse {
            success: true,
            message: format!("Successfully ignored item on {}", agent.name),
        }),
        Err(e) => Json(IgnoreItemResponse {
            success: false,
            message: format!("Failed to ignore item: {}", e),
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

    // Filter out empty strings from item_path
    let filtered_item_path: Vec<String> = payload
        .item_path
        .iter()
        .filter(|i| !i.is_empty())
        .cloned()
        .collect();

    // Extract category_id (first item) and folder_path (remaining items) from item_path
    let (category_id, folder_path) = if filtered_item_path.is_empty() {
        return Json(DeleteItemResponse {
            success: false,
            message: "No valid path provided".to_string(),
        });
    } else {
        let category_id = filtered_item_path[0].clone();
        // Use remaining items from item_path for folder path (skip the first one which is the category)
        let folder_path = if filtered_item_path.len() > 1 {
            filtered_item_path[1..].to_vec()
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
    match state.agent_client.delete_item(agent, &delete_request).await {
        Ok(_) => Json(DeleteItemResponse {
            success: true,
            message: format!("Successfully deleted item on {}", agent.name),
        }),
        Err(e) => Json(DeleteItemResponse {
            success: false,
            message: format!("Failed to delete item: {}", e),
        }),
    }
}

async fn dynamic_items(
    State(state): State<AppState>,
    Query(query): Query<DynamicItemsQuery>,
) -> impl IntoResponse {
    let mut context = state.context.clone();

    // Decode the parent_path from base64
    let decoded_parent_path = match unsanitize_id(&query.parent_path) {
        Ok(path) => path,
        Err(_) => {
            // If decoding fails, return empty items
            context.insert("items", &Vec::<ItemGroupWithFlags>::new());
            context.insert("parent_id", &query.parent_id);
            context.insert("parent_path", &query.parent_path);
            context.insert("level", &query.level);
            context.insert("minimum_copies", &state.config.manager.minimum_copies);
            return RenderHtml(
                Key("components/dynamic-items.html".to_string()),
                state.engine,
                context.into_json(),
            );
        }
    };

    let response = agents::list_categories(&state.agent_client, state.config.agents).await;

    let found_items = if query.level == 2 {
        // Level 2: Find direct children of top-level category
        response
            .items
            .iter()
            .find(|item| item.id == decoded_parent_path)
            .map(|item| &item.items)
    } else {
        // Level 3: Find children of level 2 item
        let mut found_item: Option<&Vec<ItemGroup>> = None;
        for top_level_item in &response.items {
            for level2_item in &top_level_item.items {
                if level2_item.id == decoded_parent_path {
                    found_item = Some(&level2_item.items);
                    break;
                }
            }
            if found_item.is_some() {
                break;
            }
        }
        found_item
    };

    if let Some(items) = found_items {
        let items_with_flags: Vec<ItemGroupWithFlags> = items
            .iter()
            .map(|item| convert_item_with_flags(item, state.config.manager.minimum_copies))
            .collect();

        context.insert("items", &items_with_flags);
    } else {
        context.insert("items", &Vec::<ItemGroupWithFlags>::new());
    }

    context.insert("parent_id", &query.parent_id);
    context.insert("parent_path", &query.parent_path);
    context.insert("parent_path_raw", &decoded_parent_path);
    context.insert("level", &query.level);
    context.insert("minimum_copies", &state.config.manager.minimum_copies);

    // For level 3, extract and decode category_id from parent_id (format: categoryId-level2Id)
    if query.level == 3 {
        let parts: Vec<&str> = query.parent_id.split('-').collect();
        if parts.len() >= 2 {
            let category_id_encoded = parts[0];
            if let Ok(category_id_raw) = unsanitize_id(category_id_encoded) {
                context.insert("category_id_raw", &category_id_raw);
            }
        }
    }

    RenderHtml(
        Key("components/dynamic-items.html".to_string()),
        state.engine,
        context.into_json(),
    )
}
