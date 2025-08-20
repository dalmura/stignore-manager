pub mod agent_client;
pub mod agents;
pub mod components;
pub mod config;
pub mod pages;
pub mod types;

use axum_template::engine::Engine;
use tera::{Context, Result as TeraResult, Tera, Value};

use axum::extract::FromRef;
use axum::{Router, routing::get};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::compression::CompressionLayer;

type AppEngine = Engine<Tera>;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub engine: AppEngine,
    pub context: Context,
    pub config: config::Data,
    pub agent_client: agent_client::AgentClient,
}

pub fn humansize_filter(
    value: &Value,
    _args: &std::collections::HashMap<String, Value>,
) -> TeraResult<Value> {
    let kb = value
        .as_f64()
        .ok_or_else(|| tera::Error::msg("Value must be a number"))?;
    let bytes = kb * 1024.0;

    let formatted = if bytes < 1024.0 {
        format!("{:.0} B", bytes)
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.1} KB", bytes / 1024.0)
    } else if bytes < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB", bytes / (1024.0 * 1024.0))
    } else if bytes < 1024.0 * 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} GB", bytes / (1024.0 * 1024.0 * 1024.0))
    } else {
        format!("{:.1} TB", bytes / (1024.0 * 1024.0 * 1024.0 * 1024.0))
    };

    Ok(Value::String(formatted))
}

pub fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/", get(pages::root))
        .route("/agents", get(pages::agents_overview))
        .route_service("/favicon.ico", ServeFile::new("assets/favicon.ico"))
        .nest_service("/assets", ServeDir::new("assets"))
        .nest("/components", components::router())
        .fallback(pages::not_found)
        .layer(CompressionLayer::new())
        .with_state(state)
}
