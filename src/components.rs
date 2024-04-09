use axum::{
    Router,
    routing::get,
    response::IntoResponse,
    extract::State
};

use axum_template::{Key, RenderHtml};
use serde::Serialize;

use super::AppState;


#[derive(Debug, Serialize)]
pub struct RootContext {
    page_title: String,
    message: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/navbar.html", get(navbar))
}

async fn navbar(State(state): State<AppState>) -> impl IntoResponse {
    let mut context = state.context.clone();
    context.insert("page_title", "Index");
    context.insert("message", "Welcome to stignore-manager.");

    RenderHtml(Key("components/navbar.html".to_string()), state.engine, context.into_json())
}