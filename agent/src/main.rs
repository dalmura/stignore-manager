mod filesystem;
mod tasks;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware,
    response::Response,
    routing::get,
    routing::post,
    Router,
};
use tracing_subscriber::fmt;

use std::env;
use stignore_lib::{load_agent_config, AgentData};
use tokio::signal;

async fn auth_middleware(
    State(data): State<AgentData>,
    request: Request<Body>,
    next: middleware::Next,
) -> Result<Response, StatusCode> {
    // Skip auth for help endpoint
    if request.uri().path() == "/" {
        return Ok(next.run(request).await);
    }

    // Check for X-API-Key header
    let auth_header = request
        .headers()
        .get("X-API-Key")
        .and_then(|header| header.to_str().ok());

    match auth_header {
        Some(provided_key) if provided_key == data.agent.api_key => Ok(next.run(request).await),
        _ => {
            tracing::warn!("Unauthorized access attempt to {}", request.uri().path());
            Err(StatusCode::UNAUTHORIZED)
        }
    }
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
            tracing::info!("Received Ctrl+C, shutting down gracefully...");
        },
        _ = terminate => {
            tracing::info!("Received terminate signal, shutting down gracefully...");
        },
    }
}

#[tokio::main]
async fn main() {
    /* initialize tracing */
    fmt::init();

    /* load config */
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <config_file>", args[0]);
        std::process::exit(1);
    }
    let config_filename = &args[1];

    let data = match load_agent_config(config_filename) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load configuration: {}", err);
            std::process::exit(1);
        }
    };

    /* configure application routes */
    let app = Router::new()
        .route("/", get(tasks::help))
        .route("/api/v1/categories", get(tasks::category_list))
        .route("/api/v1/categories/{id}", get(tasks::category_info))
        .route("/api/v1/items", post(tasks::post_item_info))
        .route("/api/v1/ignore", post(tasks::post_ignore))
        .route("/api/v1/ignore-status", post(tasks::post_ignore_status))
        .route(
            "/api/v1/ignore-status-bulk",
            post(tasks::post_ignore_status_bulk),
        )
        .route("/api/v1/delete", post(tasks::post_delete))
        .layer(middleware::from_fn_with_state(
            data.clone(),
            auth_middleware,
        ))
        .with_state(data.clone());

    /* bind to the port and listen */
    let addr = format!("0.0.0.0:{}", data.agent.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("listening on {}", &addr);

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
