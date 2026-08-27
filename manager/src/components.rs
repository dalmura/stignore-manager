use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};

use crate::agents;
use crate::auth::{self, AuthUser};
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
        .route("/stignore-modal.html", post(stignore_modal))
        .route("/agents/toggle", post(toggle_agent))
        .route("/agents-table.html", get(agents_table))
        .route("/agent-status-pill.html", get(agent_status_pill))
        .route("/ignore", post(ignore_item))
        .route("/unignore", post(unignore_item))
        .route("/delete", post(delete_item))
        .route("/delete-details", post(delete_item_details))
        .route("/bulk-ignore", post(bulk_ignore_item))
        .route("/bulk-unignore", post(bulk_unignore_item))
        .route("/bulk-delete", post(bulk_delete_item))
        .route("/stignore/save", post(save_stignore))
        .route("/stignore/restore", post(restore_stignore))
        .route("/stignore/validate", post(validate_stignore))
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
    pub has_conflicts: bool,
    pub conflict_count: u32,
    pub is_syncing: bool,
    pub stversions_size_kb: u64,
    pub stfolder_present: bool,
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
            has_conflicts: item.has_conflicts,
            conflict_count: item.conflict_count,
            is_syncing: item.is_syncing,
            stversions_size_kb: item.stversions_size_kb,
            stfolder_present: item.stfolder_present,
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

async fn itemlist(State(state): State<AppState>, auth_user: AuthUser) -> impl IntoResponse {
    let mut context = state.context.clone();
    auth::inject_auth_context(&mut context, &auth_user);

    let disabled_agents = state.disabled_agents.read().unwrap().clone();
    let response =
        agents::list_categories(&state.agent_client, state.config.agents, &disabled_agents).await;
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
struct IgnoreItemRequest {
    agent_name: String,
    item_path: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct DeleteItemRequest {
    agent_name: String,
    item_path: Vec<String>,
    #[serde(default)]
    delete_from_arr: Option<bool>,
    #[serde(default)]
    add_import_exclusion: Option<bool>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    media_result: Option<crate::integrations::MediaDeleteResult>,
}

#[derive(Deserialize, Debug)]
struct ToggleAgentRequest {
    agent_name: String,
    enabled: bool,
}

#[derive(Serialize, Debug)]
struct ToggleAgentResponse {
    success: bool,
    message: String,
    agent_name: String,
    enabled: bool,
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
    auth_user: AuthUser,
    Json(payload): Json<InfoPanelRequest>,
) -> impl IntoResponse {
    let mut context = state.context.clone();
    auth::inject_auth_context(&mut context, &auth_user);

    let item_path: Vec<&str> = payload
        .item_path
        .iter()
        .filter(|i| !i.is_empty())
        .map(AsRef::as_ref)
        .collect();

    let disabled_agents = state.disabled_agents.read().unwrap().clone();

    match agents::item_info(
        &state.agent_client,
        state.config.agents,
        item_path,
        &disabled_agents,
    )
    .await
    {
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
            context.insert("item_path", &filtered_item_path);
            if let Some(cat_id) = filtered_item_path.first() {
                context.insert("category_id", cat_id);
            }
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
    auth_user: AuthUser,
    Json(payload): Json<AgentModalRequest>,
) -> impl IntoResponse {
    let mut context = state.context.clone();
    auth::inject_auth_context(&mut context, &auth_user);

    // Filter out empty strings from item_path
    let item_path_parts: Vec<&str> = payload
        .item_path
        .iter()
        .filter(|i| !i.is_empty())
        .map(AsRef::as_ref)
        .collect();

    let disabled_agents = state.disabled_agents.read().unwrap().clone();

    match agents::item_info(
        &state.agent_client,
        state.config.agents,
        item_path_parts.clone(),
        &disabled_agents,
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
                if let Some(cat_id) = item_path_parts.first() {
                    context.insert("category_id", cat_id);
                }
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
    auth_user: AuthUser,
    Json(payload): Json<IgnoreItemRequest>,
) -> impl IntoResponse {
    if !auth_user.is_admin() {
        return (
            StatusCode::FORBIDDEN,
            Json(IgnoreItemResponse {
                success: false,
                message: "Access denied: Admin role required".to_string(),
            }),
        )
            .into_response();
    }
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
            })
            .into_response();
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
        })
        .into_response();
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
        })
        .into_response(),
        Err(e) => Json(IgnoreItemResponse {
            success: false,
            message: format!("Failed to ignore item: {}", e),
        })
        .into_response(),
    }
}

async fn unignore_item(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<IgnoreItemRequest>,
) -> impl IntoResponse {
    if !auth_user.is_admin() {
        return (
            StatusCode::FORBIDDEN,
            Json(IgnoreItemResponse {
                success: false,
                message: "Access denied: Admin role required".to_string(),
            }),
        )
            .into_response();
    }
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
            })
            .into_response();
        }
    };

    let filtered_item_path: Vec<String> = payload
        .item_path
        .iter()
        .filter(|i| !i.is_empty())
        .cloned()
        .collect();

    let (category_id, folder_path) = if filtered_item_path.is_empty() {
        return Json(IgnoreItemResponse {
            success: false,
            message: "No valid path provided".to_string(),
        })
        .into_response();
    } else {
        let category_id = filtered_item_path[0].clone();
        let folder_path = if filtered_item_path.len() > 1 {
            filtered_item_path[1..].to_vec()
        } else {
            vec![]
        };

        (category_id, folder_path)
    };

    let unignore_request = AgentUnignoreRequest {
        category_id,
        folder_path,
    };

    match state
        .agent_client
        .unignore_item(agent, &unignore_request)
        .await
    {
        Ok(_) => Json(IgnoreItemResponse {
            success: true,
            message: format!("Successfully un-ignored item on {}", agent.name),
        })
        .into_response(),
        Err(e) => Json(IgnoreItemResponse {
            success: false,
            message: format!("Failed to un-ignore item: {}", e),
        })
        .into_response(),
    }
}

async fn delete_item(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<DeleteItemRequest>,
) -> impl IntoResponse {
    if !auth_user.is_admin() {
        return (
            StatusCode::FORBIDDEN,
            Json(DeleteItemResponse {
                success: false,
                message: "Access denied: Admin role required".to_string(),
                media_result: None,
            }),
        )
            .into_response();
    }
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
                media_result: None,
            })
            .into_response();
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
            media_result: None,
        })
        .into_response();
    } else {
        let category_id = filtered_item_path[0].clone();
        let folder_path = if filtered_item_path.len() > 1 {
            filtered_item_path[1..].to_vec()
        } else {
            vec![]
        };

        (category_id, folder_path)
    };

    // Check how many agents currently hold this item before deletion
    let disabled_agents = state.disabled_agents.read().unwrap().clone();
    let item_path_refs: Vec<&str> = filtered_item_path.iter().map(AsRef::as_ref).collect();
    let is_last_copy = match agents::item_info(
        &state.agent_client,
        state.config.agents.clone(),
        item_path_refs,
        &disabled_agents,
    )
    .await
    {
        Ok(info_resp) => {
            let copies = info_resp
                .agent_items
                .iter()
                .filter(|(_, it)| !it.id.is_empty() && (it.size_kb > 0 || !it.items.is_empty()))
                .count();
            copies <= 1
        }
        Err(_) => true,
    };

    // Build the delete request for the agent
    let delete_request = AgentDeleteRequest {
        category_id: category_id.clone(),
        folder_path: folder_path.clone(),
    };

    // Send the delete request to the agent
    match state.agent_client.delete_item(agent, &delete_request).await {
        Ok(_) => {
            let mut message = format!("Successfully deleted item on {}", agent.name);
            let mut media_result = None;

            let should_delete_arr = (is_last_copy || payload.delete_from_arr == Some(true))
                && payload.delete_from_arr != Some(false);

            if should_delete_arr
                && let Some(media_res) = state
                    .integrations
                    .execute_media_deletion(
                        &category_id,
                        &folder_path,
                        payload.add_import_exclusion,
                    )
                    .await
            {
                message = format!("{}. {}", message, media_res.message);
                media_result = Some(media_res);
            }

            Json(DeleteItemResponse {
                success: true,
                message,
                media_result,
            })
            .into_response()
        }
        Err(e) if e.is_not_found() => {
            let mut message = format!("Item was already absent on {}", agent.name);
            let mut media_result = None;

            let should_delete_arr = (is_last_copy || payload.delete_from_arr == Some(true))
                && payload.delete_from_arr != Some(false);

            if should_delete_arr
                && let Some(media_res) = state
                    .integrations
                    .execute_media_deletion(
                        &category_id,
                        &folder_path,
                        payload.add_import_exclusion,
                    )
                    .await
            {
                message = format!("{}. {}", message, media_res.message);
                media_result = Some(media_res);
            }

            Json(DeleteItemResponse {
                success: true,
                message,
                media_result,
            })
            .into_response()
        }
        Err(e) => Json(DeleteItemResponse {
            success: false,
            message: format!("Failed to delete item: {}", e),
            media_result: None,
        })
        .into_response(),
    }
}

#[derive(Deserialize, Debug)]
pub struct DeleteItemDetailsRequest {
    pub agent_name: String,
    pub item_path: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteItemDetailsResponse {
    pub success: bool,
    pub message: Option<String>,
    pub name: String,
    pub is_leaf: bool,
    pub is_dir: bool,
    pub size_kb: u64,
    pub item_count: usize,
    pub agent_name: String,
    #[serde(default)]
    pub is_last_copy: bool,
    #[serde(default)]
    pub total_copies: u8,
    #[serde(default)]
    pub matched_media: Option<crate::integrations::MediaMatch>,
}

async fn delete_item_details(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<DeleteItemDetailsRequest>,
) -> impl IntoResponse {
    if !auth_user.is_admin() {
        return (
            StatusCode::FORBIDDEN,
            Json(DeleteItemDetailsResponse {
                success: false,
                message: Some("Access denied: Admin role required".to_string()),
                name: String::new(),
                is_leaf: false,
                is_dir: false,
                size_kb: 0,
                item_count: 0,
                agent_name: payload.agent_name,
                is_last_copy: false,
                total_copies: 0,
                matched_media: None,
            }),
        )
            .into_response();
    }
    // Find the agent by name
    let agent = match state
        .config
        .agents
        .iter()
        .find(|a| a.name == payload.agent_name)
    {
        Some(agent) => agent,
        None => {
            return Json(DeleteItemDetailsResponse {
                success: false,
                message: Some(format!("Agent '{}' not found", payload.agent_name)),
                name: String::new(),
                is_leaf: false,
                is_dir: false,
                size_kb: 0,
                item_count: 0,
                agent_name: payload.agent_name,
                is_last_copy: false,
                total_copies: 0,
                matched_media: None,
            })
            .into_response();
        }
    };

    // Filter out empty strings from item_path
    let filtered_item_path: Vec<String> = payload
        .item_path
        .iter()
        .filter(|i| !i.is_empty())
        .cloned()
        .collect();

    if filtered_item_path.is_empty() {
        return Json(DeleteItemDetailsResponse {
            success: false,
            message: Some("No valid path provided".to_string()),
            name: String::new(),
            is_leaf: false,
            is_dir: false,
            size_kb: 0,
            item_count: 0,
            agent_name: payload.agent_name,
            is_last_copy: false,
            total_copies: 0,
            matched_media: None,
        })
        .into_response();
    }

    let category_id = filtered_item_path[0].clone();
    let folder_path = if filtered_item_path.len() > 1 {
        filtered_item_path[1..].to_vec()
    } else {
        vec![]
    };

    let request = AgentItemInfoRequest {
        item_path: filtered_item_path.clone(),
    };

    let matched_media = state
        .integrations
        .inspect_deletion(&category_id, &folder_path)
        .await;

    let disabled_agents = state.disabled_agents.read().unwrap().clone();
    let item_path_refs: Vec<&str> = filtered_item_path.iter().map(AsRef::as_ref).collect();
    let (total_copies, is_last_copy) = match agents::item_info(
        &state.agent_client,
        state.config.agents.clone(),
        item_path_refs,
        &disabled_agents,
    )
    .await
    {
        Ok(info_resp) => {
            let copies = info_resp
                .agent_items
                .iter()
                .filter(|(_, it)| !it.id.is_empty() && (it.size_kb > 0 || !it.items.is_empty()))
                .count() as u8;
            (copies, copies <= 1)
        }
        Err(_) => (1, true),
    };

    match state.agent_client.get_item_info(agent, &request).await {
        Ok(resp) => {
            let is_dir = !resp.item.items.is_empty() || request.item_path.len() <= 2;
            Json(DeleteItemDetailsResponse {
                success: true,
                message: None,
                name: resp.item.name,
                is_leaf: resp.item.leaf,
                is_dir,
                size_kb: resp.item.size_kb,
                item_count: resp.item.items.len(),
                agent_name: agent.name.clone(),
                is_last_copy,
                total_copies,
                matched_media,
            })
            .into_response()
        }
        Err(e) => Json(DeleteItemDetailsResponse {
            success: false,
            message: Some(format!("Failed to retrieve item details: {}", e)),
            name: String::new(),
            is_leaf: false,
            is_dir: false,
            size_kb: 0,
            item_count: 0,
            agent_name: payload.agent_name,
            is_last_copy,
            total_copies,
            matched_media,
        })
        .into_response(),
    }
}

#[derive(Deserialize, Debug)]
pub struct BulkIgnoreRequest {
    pub agent_names: Vec<String>,
    pub item_path: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct BulkDeleteRequest {
    pub agent_names: Vec<String>,
    pub item_path: Vec<String>,
    #[serde(default)]
    pub delete_from_arr: Option<bool>,
    #[serde(default)]
    pub add_import_exclusion: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BulkActionResult {
    pub agent_name: String,
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BulkActionResponse {
    pub success: bool,
    pub message: String,
    pub results: Vec<BulkActionResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_result: Option<crate::integrations::MediaDeleteResult>,
}

async fn bulk_ignore_item(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<BulkIgnoreRequest>,
) -> impl IntoResponse {
    if !auth_user.is_admin() {
        return (
            StatusCode::FORBIDDEN,
            Json(BulkActionResponse {
                success: false,
                message: "Access denied: Admin role required".to_string(),
                results: vec![],
                media_result: None,
            }),
        )
            .into_response();
    }
    let filtered_item_path: Vec<String> = payload
        .item_path
        .iter()
        .filter(|i| !i.is_empty())
        .cloned()
        .collect();

    if filtered_item_path.is_empty() {
        return Json(BulkActionResponse {
            success: false,
            message: "No valid path provided".to_string(),
            results: vec![],
            media_result: None,
        })
        .into_response();
    }

    let category_id = filtered_item_path[0].clone();
    let folder_path = if filtered_item_path.len() > 1 {
        filtered_item_path[1..].to_vec()
    } else {
        vec![]
    };

    let ignore_request = AgentIgnoreRequest {
        category_id,
        folder_path,
    };

    let mut results = Vec::new();
    let mut overall_success = true;

    for agent_name in &payload.agent_names {
        if let Some(agent) = state.config.agents.iter().find(|a| &a.name == agent_name) {
            match state.agent_client.ignore_item(agent, &ignore_request).await {
                Ok(_) => results.push(BulkActionResult {
                    agent_name: agent_name.clone(),
                    success: true,
                    message: format!("Ignored item on {}", agent_name),
                }),
                Err(e) => {
                    overall_success = false;
                    results.push(BulkActionResult {
                        agent_name: agent_name.clone(),
                        success: false,
                        message: format!("Failed on {}: {}", agent_name, e),
                    });
                }
            }
        } else {
            overall_success = false;
            results.push(BulkActionResult {
                agent_name: agent_name.clone(),
                success: false,
                message: format!("Agent '{}' not found", agent_name),
            });
        }
    }

    Json(BulkActionResponse {
        success: overall_success,
        message: if overall_success {
            format!("Successfully ignored item across {} agents", results.len())
        } else {
            "Bulk ignore completed with errors".to_string()
        },
        results,
        media_result: None,
    })
    .into_response()
}

async fn bulk_unignore_item(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<BulkIgnoreRequest>,
) -> impl IntoResponse {
    if !auth_user.is_admin() {
        return (
            StatusCode::FORBIDDEN,
            Json(BulkActionResponse {
                success: false,
                message: "Access denied: Admin role required".to_string(),
                results: vec![],
                media_result: None,
            }),
        )
            .into_response();
    }
    let filtered_item_path: Vec<String> = payload
        .item_path
        .iter()
        .filter(|i| !i.is_empty())
        .cloned()
        .collect();

    if filtered_item_path.is_empty() {
        return Json(BulkActionResponse {
            success: false,
            message: "No valid path provided".to_string(),
            results: vec![],
            media_result: None,
        })
        .into_response();
    }

    let category_id = filtered_item_path[0].clone();
    let folder_path = if filtered_item_path.len() > 1 {
        filtered_item_path[1..].to_vec()
    } else {
        vec![]
    };

    let unignore_request = AgentUnignoreRequest {
        category_id,
        folder_path,
    };

    let mut results = Vec::new();
    let mut overall_success = true;

    for agent_name in &payload.agent_names {
        if let Some(agent) = state.config.agents.iter().find(|a| &a.name == agent_name) {
            match state
                .agent_client
                .unignore_item(agent, &unignore_request)
                .await
            {
                Ok(_) => results.push(BulkActionResult {
                    agent_name: agent_name.clone(),
                    success: true,
                    message: format!("Un-ignored item on {}", agent_name),
                }),
                Err(e) => {
                    overall_success = false;
                    results.push(BulkActionResult {
                        agent_name: agent_name.clone(),
                        success: false,
                        message: format!("Failed on {}: {}", agent_name, e),
                    });
                }
            }
        } else {
            overall_success = false;
            results.push(BulkActionResult {
                agent_name: agent_name.clone(),
                success: false,
                message: format!("Agent '{}' not found", agent_name),
            });
        }
    }

    Json(BulkActionResponse {
        success: overall_success,
        message: if overall_success {
            format!(
                "Successfully un-ignored item across {} agents",
                results.len()
            )
        } else {
            "Bulk un-ignore completed with errors".to_string()
        },
        results,
        media_result: None,
    })
    .into_response()
}

async fn bulk_delete_item(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<BulkDeleteRequest>,
) -> impl IntoResponse {
    if !auth_user.is_admin() {
        return (
            StatusCode::FORBIDDEN,
            Json(BulkActionResponse {
                success: false,
                message: "Access denied: Admin role required".to_string(),
                results: vec![],
                media_result: None,
            }),
        )
            .into_response();
    }
    let filtered_item_path: Vec<String> = payload
        .item_path
        .iter()
        .filter(|i| !i.is_empty())
        .cloned()
        .collect();

    if filtered_item_path.is_empty() {
        return Json(BulkActionResponse {
            success: false,
            message: "No valid path provided".to_string(),
            results: vec![],
            media_result: None,
        })
        .into_response();
    }

    let category_id = filtered_item_path[0].clone();
    let folder_path = if filtered_item_path.len() > 1 {
        filtered_item_path[1..].to_vec()
    } else {
        vec![]
    };

    let delete_request = AgentDeleteRequest {
        category_id: category_id.clone(),
        folder_path: folder_path.clone(),
    };

    let mut results = Vec::new();
    let mut overall_success = true;
    let mut successful_deletions = HashSet::new();

    // Check which agents currently hold this item before deletion
    let disabled_agents = state.disabled_agents.read().unwrap().clone();
    let item_path_refs: Vec<&str> = filtered_item_path.iter().map(AsRef::as_ref).collect();
    let holding_agent_names: HashSet<String> = match agents::item_info(
        &state.agent_client,
        state.config.agents.clone(),
        item_path_refs,
        &disabled_agents,
    )
    .await
    {
        Ok(info_resp) => info_resp
            .agent_items
            .iter()
            .filter(|(_, it)| !it.id.is_empty() && (it.size_kb > 0 || !it.items.is_empty()))
            .map(|(a, _)| a.name.clone())
            .collect(),
        Err(_) => payload.agent_names.iter().cloned().collect(),
    };

    for agent_name in &payload.agent_names {
        if let Some(agent) = state.config.agents.iter().find(|a| &a.name == agent_name) {
            match state.agent_client.delete_item(agent, &delete_request).await {
                Ok(_) => {
                    successful_deletions.insert(agent_name.clone());
                    results.push(BulkActionResult {
                        agent_name: agent_name.clone(),
                        success: true,
                        message: format!("Deleted item on {}", agent_name),
                    });
                }
                Err(e) if e.is_not_found() => {
                    successful_deletions.insert(agent_name.clone());
                    results.push(BulkActionResult {
                        agent_name: agent_name.clone(),
                        success: true,
                        message: format!("Item was already absent on {} (skipped)", agent_name),
                    });
                }
                Err(e) => {
                    overall_success = false;
                    results.push(BulkActionResult {
                        agent_name: agent_name.clone(),
                        success: false,
                        message: format!("Failed on {}: {}", agent_name, e),
                    });
                }
            }
        } else {
            overall_success = false;
            results.push(BulkActionResult {
                agent_name: agent_name.clone(),
                success: false,
                message: format!("Agent '{}' not found", agent_name),
            });
        }
    }

    // Check if all agents that held the item succeeded in deleting
    let all_copies_deleted = !holding_agent_names.is_empty()
        && holding_agent_names
            .iter()
            .all(|name| successful_deletions.contains(name));

    let should_delete_arr = (all_copies_deleted || payload.delete_from_arr == Some(true))
        && payload.delete_from_arr != Some(false);

    let mut media_result = None;
    let mut extra_message = String::new();

    if should_delete_arr
        && !successful_deletions.is_empty()
        && let Some(media_res) = state
            .integrations
            .execute_media_deletion(&category_id, &folder_path, payload.add_import_exclusion)
            .await
    {
        extra_message = format!(" {}", media_res.message);
        media_result = Some(media_res);
    }

    let base_message = if overall_success {
        format!("Successfully deleted item across {} agents.", results.len())
    } else {
        "Bulk delete completed with errors.".to_string()
    };

    Json(BulkActionResponse {
        success: overall_success,
        message: format!("{}{}", base_message, extra_message),
        results,
        media_result,
    })
    .into_response()
}

async fn toggle_agent(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<ToggleAgentRequest>,
) -> impl IntoResponse {
    if !auth_user.is_admin() {
        return (
            StatusCode::FORBIDDEN,
            Json(ToggleAgentResponse {
                success: false,
                message: "Access denied: Admin role required".to_string(),
                agent_name: payload.agent_name,
                enabled: payload.enabled,
            }),
        )
            .into_response();
    }
    let mut disabled = state.disabled_agents.write().unwrap();
    if payload.enabled {
        disabled.remove(&payload.agent_name);
    } else {
        disabled.insert(payload.agent_name.clone());
    }

    let status_str = if payload.enabled {
        "enabled"
    } else {
        "disabled"
    };
    Json(ToggleAgentResponse {
        success: true,
        message: format!("Agent '{}' is now {}", payload.agent_name, status_str),
        agent_name: payload.agent_name,
        enabled: payload.enabled,
    })
    .into_response()
}

async fn agents_table(State(state): State<AppState>, auth_user: AuthUser) -> impl IntoResponse {
    let mut context = state.context.clone();
    auth::inject_auth_context(&mut context, &auth_user);
    let agent_summaries = crate::pages::build_agent_summaries(&state).await;
    context.insert("agents", &agent_summaries);

    RenderHtml(
        Key("components/agents-table.html".to_string()),
        state.engine,
        context.into_json(),
    )
}

pub async fn agent_status_pill(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> impl IntoResponse {
    let mut context = state.context.clone();
    auth::inject_auth_context(&mut context, &auth_user);

    let disabled_agents = state.disabled_agents.read().unwrap().clone();
    let total = state.config.agents.len();

    if total == 0 {
        context.insert("status_level", "empty");
        context.insert("label", "0 Agents");
        context.insert("tooltip", "No agents configured • Click to view Agents");
        context.insert("total", &0);
        context.insert("enabled", &0);
        context.insert("online", &0);
        context.insert("offline", &0);
        context.insert("disabled", &0);

        return RenderHtml(
            Key("components/agent-status-pill.html".to_string()),
            state.engine,
            context.into_json(),
        );
    }

    let mut set = tokio::task::JoinSet::new();
    for agent in state.config.agents.clone() {
        let is_enabled = !disabled_agents.contains(&agent.name);
        let client = state.agent_client.clone();
        set.spawn(async move {
            if !is_enabled {
                (agent.name, false, true)
            } else {
                let is_online = client.get_categories(&agent).await.is_ok();
                (agent.name, is_online, false)
            }
        });
    }

    let mut online_count = 0;
    let mut disabled_count = 0;
    let mut offline_count = 0;

    while let Some(res) = set.join_next().await {
        if let Ok((_name, is_online, is_disabled)) = res {
            if is_disabled {
                disabled_count += 1;
            } else if is_online {
                online_count += 1;
            } else {
                offline_count += 1;
            }
        }
    }

    let enabled_count = total - disabled_count;

    let status_level = if enabled_count == 0 {
        "warning"
    } else if online_count == enabled_count && offline_count == 0 {
        if disabled_count > 0 {
            "warning"
        } else {
            "healthy"
        }
    } else if online_count > 0 {
        "warning"
    } else {
        "danger"
    };

    let label = if disabled_count > 0 {
        format!("{}/{} Online", online_count, enabled_count)
    } else {
        format!("{}/{} Online", online_count, total)
    };

    let mut tooltip_parts = Vec::new();
    if disabled_count > 0 {
        tooltip_parts.push(format!(
            "{}/{} active agents online",
            online_count, enabled_count
        ));
        tooltip_parts.push(format!("{} disabled", disabled_count));
    } else {
        tooltip_parts.push(format!("{}/{} agents online", online_count, total));
    }
    if offline_count > 0 {
        tooltip_parts.push(format!("{} unreachable", offline_count));
    }
    tooltip_parts.push("Click to view Agents".to_string());
    let tooltip = tooltip_parts.join(" • ");

    context.insert("status_level", status_level);
    context.insert("label", &label);
    context.insert("tooltip", &tooltip);
    context.insert("total", &total);
    context.insert("enabled", &enabled_count);
    context.insert("online", &online_count);
    context.insert("offline", &offline_count);
    context.insert("disabled", &disabled_count);

    RenderHtml(
        Key("components/agent-status-pill.html".to_string()),
        state.engine,
        context.into_json(),
    )
}

fn percent_decode_str(input: &str) -> String {
    let mut bytes = Vec::with_capacity(input.len());
    let mut chars = input.bytes();
    while let Some(b) = chars.next() {
        match b {
            b'+' => bytes.push(b' '),
            b'%' => {
                if let (Some(c1), Some(c2)) = (chars.next(), chars.next())
                    && let Ok(hex) = std::str::from_utf8(&[c1, c2])
                    && let Ok(byte) = u8::from_str_radix(hex, 16)
                {
                    bytes.push(byte);
                    continue;
                }
            }

            _ => bytes.push(b),
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

async fn dynamic_items(
    State(state): State<AppState>,
    auth_user: AuthUser,
    axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> impl IntoResponse {
    let mut parent_id: Option<String> = None;
    let mut parent_path: Option<String> = None;
    let mut level: Option<u8> = None;
    let mut sort: Option<String> = None;

    if let Some(query_str) = raw_query {
        for pair in query_str.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                let decoded_k = percent_decode_str(k);
                let decoded_v = percent_decode_str(v);
                match decoded_k.as_str() {
                    "parent_id" => parent_id = Some(decoded_v),
                    "parent_path" => parent_path = Some(decoded_v),
                    "level" => {
                        if let Ok(l) = decoded_v.parse::<u8>() {
                            level = Some(l);
                        }
                    }
                    "sort" => sort = Some(decoded_v),
                    _ => {}
                }
            }
        }
    }

    let (parent_id, parent_path, level) = match (parent_id, parent_path, level) {
        (Some(id), Some(path), Some(lvl)) if lvl == 2 || lvl == 3 => (id, path, lvl),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                "Missing or invalid required query parameters (parent_id, parent_path, level)",
            )
                .into_response();
        }
    };

    let mut context = state.context.clone();
    auth::inject_auth_context(&mut context, &auth_user);

    // Decode the parent_path from base64
    let decoded_parent_path = match unsanitize_id(&parent_path) {
        Ok(path) => path,
        Err(_) => {
            // If decoding fails, return empty items
            context.insert("items", &Vec::<ItemGroupWithFlags>::new());
            context.insert("parent_id", &parent_id);
            context.insert("parent_path", &parent_path);
            context.insert("level", &level);
            context.insert("minimum_copies", &state.config.manager.minimum_copies);
            return RenderHtml(
                Key("components/dynamic-items.html".to_string()),
                state.engine,
                context.into_json(),
            )
            .into_response();
        }
    };

    let disabled_agents = state.disabled_agents.read().unwrap().clone();
    let response =
        agents::list_categories(&state.agent_client, state.config.agents, &disabled_agents).await;

    let sort_order = SortOrder::from_query(sort.as_deref());

    let found_items = if level == 2 {
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
        let mut sorted_items = items.clone();
        sort_order.sort_items(&mut sorted_items);

        let items_with_flags: Vec<ItemGroupWithFlags> = sorted_items
            .iter()
            .map(|item| convert_item_with_flags(item, state.config.manager.minimum_copies))
            .collect();

        context.insert("items", &items_with_flags);
    } else {
        context.insert("items", &Vec::<ItemGroupWithFlags>::new());
    }

    context.insert("sort_order", sort_order.as_str());

    context.insert("parent_id", &parent_id);
    context.insert("parent_path", &parent_path);
    context.insert("parent_path_raw", &decoded_parent_path);
    context.insert("level", &level);
    context.insert("minimum_copies", &state.config.manager.minimum_copies);

    // For level 3, extract and decode category_id from parent_id (format: categoryId-level2Id)
    if level == 3 {
        let parts: Vec<&str> = parent_id.split('-').collect();
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
    .into_response()
}

#[derive(Deserialize, Debug)]
pub struct StignoreModalRequest {
    pub agent_name: String,
    pub category_id: String,
}

#[derive(Deserialize, Debug)]
pub struct SaveStignoreRequest {
    pub agent_name: String,
    pub category_id: String,
    pub content: String,
    #[serde(default)]
    pub expected_hash: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct SaveStignoreResponse {
    pub success: bool,
    pub message: String,
    pub new_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_created: Option<String>,
    pub conflict: bool,
    pub validation: stignore_lib::StignoreValidationReport,
}

#[derive(Deserialize, Debug)]
pub struct RestoreStignoreRequest {
    pub agent_name: String,
    pub category_id: String,
    pub backup_filename: String,
}

#[derive(Serialize, Debug)]
pub struct RestoreStignoreResponse {
    pub success: bool,
    pub message: String,
    pub restored_content: String,
    pub new_hash: String,
}

#[derive(Deserialize, Debug)]
pub struct ValidateStignoreRequest {
    pub content: String,
}

#[derive(Serialize, Debug)]
pub struct ValidateStignoreResponse {
    pub report: stignore_lib::StignoreValidationReport,
}

async fn stignore_modal(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<StignoreModalRequest>,
) -> impl IntoResponse {
    let mut context = state.context.clone();
    auth::inject_auth_context(&mut context, &auth_user);

    let clean_category_id = payload.category_id.trim();
    if clean_category_id.is_empty() {
        context.insert("error", "No Category ID specified");
        return RenderHtml(
            Key("components/stignore-modal.html".to_string()),
            state.engine,
            context.into_json(),
        );
    }

    let agent = match state
        .config
        .agents
        .iter()
        .find(|a| a.name == payload.agent_name)
    {
        Some(a) => a,
        None => {
            context.insert(
                "error",
                &format!("Agent '{}' not found", payload.agent_name),
            );
            return RenderHtml(
                Key("components/stignore-modal.html".to_string()),
                state.engine,
                context.into_json(),
            );
        }
    };

    let req = AgentGetStignoreRequest {
        category_id: clean_category_id.to_string(),
    };

    match state.agent_client.get_stignore(agent, &req).await {
        Ok(resp) => {
            let validation = validate_stignore_content(&resp.content);
            context.insert("agent_name", &agent.name);
            context.insert("category_id", &payload.category_id);
            context.insert("content", &resp.content);
            context.insert("hash", &resp.hash);
            context.insert("exists", &resp.exists);
            context.insert("backups", &resp.backups);
            context.insert("validation", &validation);
        }
        Err(err) => {
            context.insert(
                "error",
                &format!(
                    "Failed to retrieve .stignore from agent '{}': {}",
                    agent.name, err
                ),
            );
        }
    }

    RenderHtml(
        Key("components/stignore-modal.html".to_string()),
        state.engine,
        context.into_json(),
    )
}

async fn save_stignore(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<SaveStignoreRequest>,
) -> impl IntoResponse {
    if !auth_user.is_admin() {
        return (
            StatusCode::FORBIDDEN,
            Json(SaveStignoreResponse {
                success: false,
                message: "Access denied: Admin role required".to_string(),
                new_hash: String::new(),
                backup_created: None,
                conflict: false,
                validation: StignoreValidationReport::default(),
            }),
        )
            .into_response();
    }

    let agent = match state
        .config
        .agents
        .iter()
        .find(|a| a.name == payload.agent_name)
    {
        Some(a) => a,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(SaveStignoreResponse {
                    success: false,
                    message: format!("Agent '{}' not found", payload.agent_name),
                    new_hash: String::new(),
                    backup_created: None,
                    conflict: false,
                    validation: StignoreValidationReport::default(),
                }),
            )
                .into_response();
        }
    };

    let validation = validate_stignore_content(&payload.content);
    if !validation.is_valid {
        let first_err = validation
            .issues
            .iter()
            .find(|i| i.severity == StignoreIssueSeverity::Error)
            .map(|i| i.message.clone())
            .unwrap_or_else(|| "Syntax validation failed".to_string());

        return (
            StatusCode::BAD_REQUEST,
            Json(SaveStignoreResponse {
                success: false,
                message: format!("Validation error: {}", first_err),
                new_hash: String::new(),
                backup_created: None,
                conflict: false,
                validation,
            }),
        )
            .into_response();
    }

    let req = AgentSetStignoreRequest {
        category_id: payload.category_id,
        content: payload.content,
        expected_hash: payload.expected_hash,
    };

    match state.agent_client.set_stignore(agent, &req).await {
        Ok(resp) => Json(SaveStignoreResponse {
            success: true,
            message: resp.message,
            new_hash: resp.new_hash,
            backup_created: resp.backup_created,
            conflict: false,
            validation,
        })
        .into_response(),
        Err(err) if err.is_conflict() => (
            StatusCode::CONFLICT,
            Json(SaveStignoreResponse {
                success: false,
                message: err.to_string(),
                new_hash: String::new(),
                backup_created: None,
                conflict: true,
                validation,
            }),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SaveStignoreResponse {
                success: false,
                message: err.to_string(),
                new_hash: String::new(),
                backup_created: None,
                conflict: false,
                validation,
            }),
        )
            .into_response(),
    }
}

async fn restore_stignore(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<RestoreStignoreRequest>,
) -> impl IntoResponse {
    if !auth_user.is_admin() {
        return (
            StatusCode::FORBIDDEN,
            Json(RestoreStignoreResponse {
                success: false,
                message: "Access denied: Admin role required".to_string(),
                restored_content: String::new(),
                new_hash: String::new(),
            }),
        )
            .into_response();
    }

    let agent = match state
        .config
        .agents
        .iter()
        .find(|a| a.name == payload.agent_name)
    {
        Some(a) => a,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(RestoreStignoreResponse {
                    success: false,
                    message: format!("Agent '{}' not found", payload.agent_name),
                    restored_content: String::new(),
                    new_hash: String::new(),
                }),
            )
                .into_response();
        }
    };

    let req = AgentRestoreStignoreRequest {
        category_id: payload.category_id,
        backup_filename: payload.backup_filename,
    };

    match state.agent_client.restore_stignore(agent, &req).await {
        Ok(resp) => Json(RestoreStignoreResponse {
            success: true,
            message: resp.message,
            restored_content: resp.restored_content,
            new_hash: resp.new_hash,
        })
        .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(RestoreStignoreResponse {
                success: false,
                message: err.to_string(),
                restored_content: String::new(),
                new_hash: String::new(),
            }),
        )
            .into_response(),
    }
}

async fn validate_stignore(Json(payload): Json<ValidateStignoreRequest>) -> impl IntoResponse {
    let report = validate_stignore_content(&payload.content);
    Json(ValidateStignoreResponse { report })
}
