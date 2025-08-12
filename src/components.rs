use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use crate::agents;
use crate::types::ItemGroup;
use axum_template::{Key, RenderHtml};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/navbar.html", get(navbar))
        .route("/itemlist.html", get(itemlist))
        .route("/infopanel.html", post(infopanel))
        .route("/agent-modal.html", get(agent_modal))
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

async fn itemlist(State(state): State<AppState>) -> impl IntoResponse {
    let mut context = state.context.clone();

    match agents::list_categories(state.config.agents).await {
        Ok(response) => {
            let mut sorted_items = response.items;
            sorted_items.sort_by(|a, b| a.name.cmp(&b.name));
            context.insert("items", &sorted_items);
        }
        Err(_) => {
            let items: Vec<ItemGroup> = vec![];
            context.insert("items", &items);
        }
    }

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

#[derive(Serialize, Debug)]
struct AgentItemWithStatus {
    agent: crate::config::Agent,
    item: ItemGroup,
    sync_status: String,
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

    // Add this item's ID if it has one and is not empty
    if !item_group.id.is_empty() {
        ids.insert(item_group.id.clone());
    }

    // Recursively collect IDs from all sub-items
    for item in &item_group.items {
        ids.extend(collect_all_item_ids(item));
    }

    ids
}

fn calculate_sync_status(
    agent_items: &[(crate::config::Agent, ItemGroup)],
) -> Vec<AgentItemWithStatus> {
    // Collect all unique item IDs from all agents (including nested items)
    let mut all_item_ids: HashSet<String> = HashSet::new();
    for (_, item_group) in agent_items {
        all_item_ids.extend(collect_all_item_ids(item_group));
    }

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

        result.push(AgentItemWithStatus {
            agent: agent.clone(),
            item: item_group.clone(),
            sync_status,
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

            let agent_items_with_status = calculate_sync_status(&response.agent_items);

            context.insert("item", &response.item);
            context.insert("agent_items", &agent_items_with_status);
            context.insert("parent_names", &payload.hierarchy_names[1..]);
            context.insert("item_path", &payload.item_path);
        }
        Err(_) => {

            // Insert empty defaults to prevent template errors
            context.insert("agent_items", &Vec::<()>::new());
            context.insert("parent_names", &Vec::<String>::new());
            context.insert("item_path", &Vec::<String>::new());
        }
    }


    let result = RenderHtml(
        Key("components/infopanel.html".to_string()),
        state.engine,
        context.into_json(),
    );

    result
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
            let agent_items_with_status = calculate_sync_status(&response.agent_items);

            // Find the specific agent's data
            if let Some(agent_item_with_status) = agent_items_with_status
                .iter()
                .find(|a| a.agent.name == query.agent_name)
            {

                // Collect all unique item names and find max sizes
                let mut all_item_names = std::collections::HashSet::new();
                let mut max_sizes: std::collections::HashMap<String, u64> =
                    std::collections::HashMap::new();

                for agent_with_status in &agent_items_with_status {
                    for item in &agent_with_status.item.items {
                        all_item_names.insert(item.name.clone());
                        max_sizes
                            .entry(item.name.clone())
                            .and_modify(|max| *max = (*max).max(item.size_kb))
                            .or_insert(item.size_kb);
                    }
                }

                // Create merged items showing availability across agents
                let mut merged_items = Vec::new();
                for item_name in all_item_names {
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


    let result = RenderHtml(
        Key("components/agent-modal.html".to_string()),
        state.engine,
        context.into_json(),
    );

    result
}
