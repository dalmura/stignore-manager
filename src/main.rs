mod config;
mod pages;
mod components;
mod agents;
mod types;

use axum_template::{engine::Engine};
use tera::{Context, Tera};

use axum::{routing::get, Router};
use tracing_subscriber::fmt;
use axum::extract::FromRef;

use tower_http::services::{ServeFile, ServeDir};

type AppEngine = Engine<Tera>;

#[derive(Clone, FromRef)]
pub struct AppState {
    engine: AppEngine,
    context: Context,
    config: config::Data,
}

#[tokio::main]
async fn main() {
    /* initialize tracing */
    fmt::init();

    let tera = Tera::new("html/**/*").unwrap();
    let data = config::load_config("./config.toml");
    let mut context = Context::new();
    context.insert("title", "stignore-manager");
    context.insert("copyright", "© 2024 Dalmura");

    let app = Router::new()
        .route("/", get(pages::root))
        .route_service("/favicon.ico", ServeFile::new("assets/favicon.ico"))
        .nest_service("/assets", ServeDir::new("assets"))
        .nest("/components", components::router())
        .fallback(pages::not_found)
        .with_state(AppState { engine: Engine::from(tera), context, config: data.clone() });

    /* bind to the port and listen */
    let addr = format!("127.0.0.1:{}", data.manager.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::debug!("listening on {}", &addr);

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
