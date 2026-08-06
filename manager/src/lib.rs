pub mod agent_client;
pub mod agents;
pub mod auth;
pub mod components;
pub mod config;
pub mod pages;

use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Router, routing::get};
use axum_template::TemplateEngine;
use tera::Value;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};

#[derive(Debug, Clone)]
pub struct TeraEngine(pub tera::Tera);

#[derive(Debug)]
pub struct TeraError(pub tera::Error);

impl std::fmt::Display for TeraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TeraError {}

impl From<tera::Error> for TeraError {
    fn from(err: tera::Error) -> Self {
        TeraError(err)
    }
}

impl IntoResponse for TeraError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

impl TemplateEngine for TeraEngine {
    type Error = TeraError;

    fn render<D: serde::Serialize>(&self, key: &str, data: D) -> Result<String, Self::Error> {
        let context = tera::Context::from_serialize(&data)?;
        let rendered = self.0.render(key, &context)?;
        Ok(rendered)
    }
}

pub type AppEngine = TeraEngine;

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Context(pub serde_json::Map<String, serde_json::Value>);

impl Context {
    pub fn new() -> Self {
        Self(serde_json::Map::new())
    }

    pub fn insert<S: Into<String>, T: serde::Serialize + ?Sized>(&mut self, key: S, val: &T) {
        if let Ok(v) = serde_json::to_value(val) {
            self.0.insert(key.into(), v);
        }
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.0.get(key)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    pub fn into_json(self) -> serde_json::Value {
        serde_json::Value::Object(self.0)
    }
}

#[derive(Clone, FromRef)]
pub struct AppState {
    pub engine: AppEngine,
    pub context: Context,
    pub config: stignore_lib::ManagerData,
    pub agent_client: agent_client::AgentClient,
    pub disabled_agents: std::sync::Arc<std::sync::RwLock<std::collections::HashSet<String>>>,
}

pub fn humansize_filter(
    kb: f64,
    _kwargs: tera::Kwargs,
    _state: &tera::State,
) -> Result<Value, tera::Error> {
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

    Ok(Value::from(formatted))
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
