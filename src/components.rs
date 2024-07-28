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

#[derive(Deserialize)]
struct InfoPanelRequest {
    hierarchy_names: Vec<String>,
    item_path: Vec<String>,
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

    tracing::info!("Using item path: {:?}", item_path);

    match agents::item_info(state.config.agents, item_path).await {
        Ok(response) => {
            context.insert("item", &response.item);
            context.insert("agent_items", &response.agent_items);
            context.insert("parent_names", &payload.hierarchy_names[1..]);
        }
        Err(t) => {
            tracing::error!("Error in agents::item_info");
            tracing::debug!("{}", t);
        }
    }

    tracing::info!("Context: {:?}", context.clone().into_json());

    RenderHtml(
        Key("components/infopanel.html".to_string()),
        state.engine,
        context.into_json(),
    )
}
