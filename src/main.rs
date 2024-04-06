mod config;
mod pages;

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
}

#[tokio::main]
async fn main() {
    /* initialize tracing */
    fmt::init();

    let tera = Tera::new("html/**/*").unwrap();
    let data = config::load_config("./config.toml");
    let mut context = Context::new();
    context.insert("title", "stignore-manager");
    context.insert("copyright", "© 2024");

    /* configure application routes */
    let app = Router::new()
        .route("/", get(pages::root))
        .route("/login", get(pages::login))
        .route("/about", get(pages::about))
        .route_service("/favicon.ico", ServeFile::new("assets/favicon.ico"))
        .nest_service("/assets", ServeDir::new("assets"))
        .fallback(pages::not_found)
        .with_state(AppState {
            engine: Engine::from(tera),
            context,
        });

    /* bind to the port and listen */
    let addr = format!("127.0.0.1:{}", data.manager.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::debug!("listening on {}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
