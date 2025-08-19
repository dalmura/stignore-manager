use axum::{extract::State, http::StatusCode, response::IntoResponse};

use axum_template::{Key, RenderHtml};
use serde::Serialize;

use super::AppState;
use crate::agents;

#[derive(Serialize)]
struct CategoryInfo {
    name: String,
    size_mb: f64,
    item_count: usize,
}

#[derive(Serialize)]
struct AgentSummary {
    name: String,
    url: String,
    total_size_mb: f64,
    categories: Vec<CategoryInfo>,
    status: String,
    status_message: Option<String>,
}

pub async fn root(State(state): State<AppState>) -> impl IntoResponse {
    let mut context = state.context.clone();
    context.insert("page_title", "Index");
    context.insert("current_page", "home");
    context.insert("message", "Welcome to stignore-manager.");

    RenderHtml(
        Key("pages/index.html".to_string()),
        state.engine,
        context.into_json(),
    )
}

pub async fn agents_overview(State(state): State<AppState>) -> impl IntoResponse {
    let mut context = state.context.clone();
    context.insert("page_title", "Agents Overview");
    context.insert("current_page", "agents");

    let mut agent_summaries = Vec::new();

    // Test connectivity to each agent individually first
    for agent in &state.config.agents {
        let mut total_size_kb = 0u64;
        let mut category_infos = Vec::new();
        let mut status_message = None;

        // Test basic connectivity by trying to get categories
        let agent_status = match state.agent_client.get_categories(agent).await {
            Ok(categories_response) => {
                // Agent is reachable, now get detailed info for each category
                for category in &categories_response.items {
                    match agents::item_info(
                        &state.agent_client,
                        vec![agent.clone()],
                        vec![category.id.as_str()],
                    )
                    .await
                    {
                        Ok(item_response) => {
                            if let Some((_, item_group)) = item_response
                                .agent_items
                                .iter()
                                .find(|(a, _)| a.name == agent.name)
                            {
                                let size_kb = item_group.size_kb;
                                let item_count = item_group.items.len();

                                if size_kb > 0 || item_count > 0 {
                                    category_infos.push(CategoryInfo {
                                        name: category.name.clone(),
                                        size_mb: (size_kb as f64 / 1024.0).round(),
                                        item_count,
                                    });
                                }

                                total_size_kb += size_kb;
                            }
                        }
                        Err(_) => {
                            // Skip this category if there's an error
                            continue;
                        }
                    }
                }

                // Sort categories by name
                category_infos.sort_by(|a, b| a.name.cmp(&b.name));

                if total_size_kb > 0 {
                    "Active".to_string()
                } else {
                    "Empty".to_string()
                }
            }
            Err(e) => {
                // Agent is not reachable, determine the type of error
                let status = match e {
                    crate::agent_client::AgentError::Timeout(_) => "Timeout".to_string(),
                    crate::agent_client::AgentError::RequestFailed(_) => "Unreachable".to_string(),
                    crate::agent_client::AgentError::InvalidResponse(_) => "Error".to_string(),
                    crate::agent_client::AgentError::OperationFailed(_) => "Error".to_string(),
                };

                status_message = Some(match e {
                    crate::agent_client::AgentError::Timeout(_) => {
                        format!(
                            "Request timed out after {} seconds",
                            state.config.manager.agent_timeout_seconds
                        )
                    }
                    crate::agent_client::AgentError::RequestFailed(_) => {
                        "Could not connect to agent".to_string()
                    }
                    crate::agent_client::AgentError::InvalidResponse(msg) => {
                        format!("Invalid response: {}", msg)
                    }
                    crate::agent_client::AgentError::OperationFailed(msg) => {
                        format!("Operation failed: {}", msg)
                    }
                });

                status
            }
        };

        let summary = AgentSummary {
            name: agent.name.clone(),
            url: agent.hostname.clone(),
            total_size_mb: (total_size_kb as f64 / 1024.0).round(),
            categories: category_infos,
            status: agent_status,
            status_message,
        };
        agent_summaries.push(summary);
    }

    // Sort by name
    agent_summaries.sort_by(|a, b| a.name.cmp(&b.name));

    context.insert("agents", &agent_summaries);

    RenderHtml(
        Key("pages/agents_overview.html".to_string()),
        state.engine,
        context.into_json(),
    )
}

pub async fn not_found(State(state): State<AppState>) -> impl IntoResponse {
    let mut context = state.context.clone();
    context.insert("page_title", "Not Found");
    context.insert("current_page", ""); // No active nav for 404 pages
    context.insert("message", "Whatever you are looking for isn't here!");

    (
        StatusCode::NOT_FOUND,
        RenderHtml(
            Key("pages/not_found.html".to_string()),
            state.engine,
            context.into_json(),
        ),
    )
}
