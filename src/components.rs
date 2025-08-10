use axum::{
    extract::State,
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
        Err(t) => {
            tracing::debug!("{}", t);
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

#[derive(Serialize, Debug)]
struct AgentItemWithStatus {
    agent: crate::config::Agent,
    item: ItemGroup,
    sync_status: String,
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
            format!("Partial ({}/{})", agent_item_ids.len(), all_item_ids.len())
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
    tracing::info!(
        "DEBUG: InfoPanel request received with payload: {:?}",
        payload
    );

    let mut context = state.context.clone();

    let item_path: Vec<&str> = payload
        .item_path
        .iter()
        .filter(|i| !i.is_empty())
        .map(AsRef::as_ref)
        .collect();

    tracing::info!("DEBUG: Filtered item path: {:?}", item_path);
    tracing::info!(
        "DEBUG: Original hierarchy_names: {:?}",
        payload.hierarchy_names
    );
    tracing::info!(
        "DEBUG: Parent names (excluding first): {:?}",
        &payload.hierarchy_names[1..]
    );

    match agents::item_info(state.config.agents, item_path).await {
        Ok(response) => {
            tracing::info!("DEBUG: agents::item_info succeeded");
            tracing::info!("DEBUG: Response item: {:?}", response.item);
            tracing::info!(
                "DEBUG: Response agent_items count: {}",
                response.agent_items.len()
            );
            tracing::info!("DEBUG: Response agent_items: {:?}", response.agent_items);

            let agent_items_with_status = calculate_sync_status(&response.agent_items);

            context.insert("item", &response.item);
            context.insert("agent_items", &agent_items_with_status);
            context.insert("parent_names", &payload.hierarchy_names[1..]);
        }
        Err(t) => {
            tracing::error!("DEBUG: Error in agents::item_info: {}", t);
            tracing::debug!("DEBUG: Full error details: {}", t);

            // Insert empty defaults to prevent template errors
            context.insert("agent_items", &Vec::<()>::new());
            context.insert("parent_names", &Vec::<String>::new());
        }
    }

    tracing::info!(
        "DEBUG: Final template context: {:?}",
        context.clone().into_json()
    );

    let result = RenderHtml(
        Key("components/infopanel.html".to_string()),
        state.engine,
        context.into_json(),
    );

    tracing::info!("DEBUG: Template rendering initiated");
    result
}
