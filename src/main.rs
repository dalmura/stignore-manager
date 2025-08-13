mod agents;
mod components;
mod config;
mod pages;
mod types;

use axum_template::engine::Engine;
use tera::{Context, Tera};

use axum::extract::FromRef;
use axum::{Router, routing::get};
use tracing_subscriber::fmt;

use tower_http::services::{ServeDir, ServeFile};

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
        .route("/agents", get(pages::agents_overview))
        .route_service("/favicon.ico", ServeFile::new("assets/favicon.ico"))
        .nest_service("/assets", ServeDir::new("assets"))
        .nest("/components", components::router())
        .fallback(pages::not_found)
        .with_state(AppState {
            engine: Engine::from(tera),
            context,
            config: data.clone(),
        });

    /* bind to the port and listen */
    let addr = format!("127.0.0.1:{}", data.manager.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("listening on {}", &addr);

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
