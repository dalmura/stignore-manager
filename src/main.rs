mod agent_client;
mod agents;
mod components;
mod config;
mod pages;
mod types;

use axum_template::engine::Engine;
use tera::{Context, Result as TeraResult, Tera, Value};

use axum::extract::FromRef;
use axum::{Router, routing::get};
use tracing_subscriber::fmt;

use std::env;

use tokio::signal;
use tower_http::services::{ServeDir, ServeFile};

type AppEngine = Engine<Tera>;

#[derive(Clone, FromRef)]
pub struct AppState {
    engine: AppEngine,
    context: Context,
    config: config::Data,
    agent_client: agent_client::AgentClient,
}

fn humansize_filter(
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

#[tokio::main]
async fn main() {
    /* initialize tracing */
    fmt::init();

    /* load config */
    let args: Vec<String> = env::args().collect();
    let config_filename = &args[1];

    let data = config::load_config(config_filename);

    /* setup templates and context */
    let mut tera = Tera::new("html/**/*.html").unwrap();
    tera.register_filter("humansize", humansize_filter);
    let mut context = Context::new();
    context.insert("title", "stignore-manager");
    context.insert("copyright", "© 2024 Dalmura");

    let app = Router::new()
        .route("/", get(pages::root))
        .route("/agents", get(pages::agents_overview))
        .route_service("/favicon.ico", ServeFile::new("assets/favicon.ico"))
        .nest_service("/assets", ServeDir::new("assets"))
        .nest("/components", components::router())
        .fallback(pages::not_found)
        .with_state(AppState {
            engine: Engine::from(tera),
            context,
            config: data.clone(),
            agent_client: agent_client::AgentClient::with_timeout(
                data.manager.agent_timeout_seconds,
            ),
        });

    /* bind to the port and listen */
    let addr = format!("0.0.0.0:{}", data.manager.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("listening on {}", &addr);

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("received Ctrl+C, shutting down gracefully...");
        },
        _ = terminate => {
            tracing::info!("received SIGTERM, shutting down gracefully...");
        },
    }
}
