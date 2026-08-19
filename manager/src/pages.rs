use axum::{extract::State, http::StatusCode, response::IntoResponse};

use axum_template::{Key, RenderHtml};
use serde::Serialize;

use super::AppState;
use crate::agents;

#[derive(Serialize)]
pub struct CategoryInfo {
    pub name: String,
    pub size_kb: u64,
    pub item_count: usize,
    pub stversions_size_kb: u64,
    pub stfolder_present: bool,
    pub has_conflicts: bool,
    pub is_syncing: bool,
}

#[derive(Serialize)]
pub struct AgentSummary {
    pub name: String,
    pub url: String,
    pub total_size_kb: u64,
    pub categories: Vec<CategoryInfo>,
    pub status: String,
    pub status_message: Option<String>,
    pub enabled: bool,
    pub latency_ms: Option<u128>,
}

use crate::auth::{self, AuthUser};

pub async fn root(State(state): State<AppState>, auth_user: AuthUser) -> impl IntoResponse {
    let mut context = state.context.clone();
    auth::inject_auth_context(&mut context, &auth_user);
    context.insert("page_title", "Index");
    context.insert("current_page", "home");
    context.insert("message", "Welcome to stignore-manager.");

    RenderHtml(
        Key("pages/index.html".to_string()),
        state.engine,
        context.into_json(),
    )
}

pub async fn build_agent_summaries(state: &AppState) -> Vec<AgentSummary> {
    let mut agent_summaries = Vec::new();
    let disabled_agents = state.disabled_agents.read().unwrap().clone();

    // Test connectivity to each agent individually first
    for agent in &state.config.agents {
        let is_enabled = !disabled_agents.contains(&agent.name);
        let mut total_size_kb = 0u64;
        let mut category_infos = Vec::new();
        let mut status_message = None;
        let mut latency_ms = None;

        let agent_status = if !is_enabled {
            status_message = Some("Agent is manually disabled by user".to_string());
            "Disabled".to_string()
        } else {
            let start_time = std::time::Instant::now();
            match state.agent_client.get_categories(agent).await {
                Ok(categories_response) => {
                    latency_ms = Some(start_time.elapsed().as_millis());
                    // Agent is reachable, now get detailed info for each category
                    for category in &categories_response.items {
                        match agents::item_info(
                            &state.agent_client,
                            vec![agent.clone()],
                            vec![category.id.as_str()],
                            &disabled_agents,
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

                                    if size_kb > 0
                                        || item_count > 0
                                        || item_group.stversions_size_kb > 0
                                    {
                                        category_infos.push(CategoryInfo {
                                            name: category.name.clone(),
                                            size_kb,
                                            item_count,
                                            stversions_size_kb: item_group.stversions_size_kb,
                                            stfolder_present: item_group.stfolder_present,
                                            has_conflicts: item_group.has_conflicts,
                                            is_syncing: item_group.is_syncing,
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
                    latency_ms = Some(start_time.elapsed().as_millis());
                    // Agent is not reachable, determine the type of error
                    let status = match e {
                        crate::agent_client::AgentError::Timeout(_) => "Timeout".to_string(),
                        crate::agent_client::AgentError::RequestFailed(_) => {
                            "Unreachable".to_string()
                        }
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
            }
        };

        let summary = AgentSummary {
            name: agent.name.clone(),
            url: agent.hostname.clone(),
            total_size_kb,
            categories: category_infos,
            status: agent_status,
            status_message,
            enabled: is_enabled,
            latency_ms,
        };
        agent_summaries.push(summary);
    }

    // Sort by name
    agent_summaries.sort_by(|a, b| a.name.cmp(&b.name));
    agent_summaries
}

pub async fn agents_overview(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> impl IntoResponse {
    let mut context = state.context.clone();
    auth::inject_auth_context(&mut context, &auth_user);
    context.insert("page_title", "Agents Overview");
    context.insert("current_page", "agents");

    let agent_summaries = build_agent_summaries(&state).await;
    context.insert("agents", &agent_summaries);

    RenderHtml(
        Key("pages/agents_overview.html".to_string()),
        state.engine,
        context.into_json(),
    )
}

pub async fn not_found(State(state): State<AppState>, auth_user: AuthUser) -> impl IntoResponse {
    let mut context = state.context.clone();
    auth::inject_auth_context(&mut context, &auth_user);
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
