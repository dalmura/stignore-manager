use axum::{
    Router,
    routing::get,
    response::IntoResponse,
    extract::State
};

use axum_template::{Key, RenderHtml};
use serde::Serialize;
use crate::agents::list_categories;

use super::AppState;


#[derive(Debug, Serialize)]
pub struct RootContext {
    page_title: String,
    message: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/navbar.html", get(navbar))
        .route("/itemlist.html", get(itemlist))
}

async fn navbar(State(state): State<AppState>) -> impl IntoResponse {
    let mut context = state.context.clone();
    context.insert("page_title", "Index");
    context.insert("message", "Welcome to stignore-manager.");

    RenderHtml(Key("components/navbar.html".to_string()), state.engine, context.into_json())
}

async fn itemlist(State(state): State<AppState>) -> impl IntoResponse {
    let mut context = state.context.clone();

    let categories = list_categories(state.config.agents).categories;
    context.insert("categories", &categories);

    RenderHtml(Key("components/itemlist.html".to_string()), state.engine, context.into_json())
}