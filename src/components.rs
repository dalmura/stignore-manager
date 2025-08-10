use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use crate::agents;
use crate::types::ItemGroup;
use axum_template::{Key, RenderHtml};
use serde::Deserialize;

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
            context.insert("items", &response.items);
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

            context.insert("item", &response.item);
            context.insert("agent_items", &response.agent_items);
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
