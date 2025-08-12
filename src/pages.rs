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
}

pub async fn root(State(state): State<AppState>) -> impl IntoResponse {
    let mut context = state.context.clone();
    context.insert("page_title", "Index");
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

    // Get categories first, then collect per-agent totals
    match agents::list_categories(state.config.agents.clone()).await {
        Ok(category_response) => {
            let mut agent_summaries = Vec::new();

            // Get all categories with both ID and name
            let categories: Vec<(String, String)> = category_response
                .items
                .iter()
                .map(|item| (item.id.clone(), item.name.clone()))
                .collect();

            // For each agent, collect category-specific data
            for agent in &state.config.agents {
                let mut total_size_kb = 0u64;
                let mut category_infos = Vec::new();

                // Query each category for this agent
                for (category_id, category_name) in &categories {
                    match agents::item_info(state.config.agents.clone(), vec![category_id.as_str()])
                        .await
                    {
                        Ok(item_response) => {
                            // Find this agent's data in the response
                            if let Some((_, item_group)) = item_response
                                .agent_items
                                .iter()
                                .find(|(a, _)| a.name == agent.name)
                            {
                                let size_kb = item_group.size_kb;
                                let item_count = item_group.items.len();

                                if size_kb > 0 || item_count > 0 {
                                    category_infos.push(CategoryInfo {
                                        name: category_name.clone(),
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

                let summary = AgentSummary {
                    name: agent.name.clone(),
                    url: agent.hostname.clone(),
                    total_size_mb: (total_size_kb as f64 / 1024.0).round(),
                    categories: category_infos,
                    status: if total_size_kb > 0 {
                        "Active".to_string()
                    } else {
                        "Empty".to_string()
                    },
                };
                agent_summaries.push(summary);
            }

            // Sort by name
            agent_summaries.sort_by(|a, b| a.name.cmp(&b.name));

            context.insert("agents", &agent_summaries);
        }
        Err(_) => {
            context.insert("agents", &Vec::<AgentSummary>::new());
        }
    }

    RenderHtml(
        Key("pages/agents_overview.html".to_string()),
        state.engine,
        context.into_json(),
    )
}

pub async fn not_found(State(state): State<AppState>) -> impl IntoResponse {
    let mut context = state.context.clone();
    context.insert("page_title", "Not Found");
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
