use axum::{extract::State, http::StatusCode, response::IntoResponse};

use axum_template::{Key, RenderHtml};

use super::AppState;

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
