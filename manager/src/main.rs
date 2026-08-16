use stignore_manager::{
    AppState, Context, TeraEngine, agent_client, config, create_app, humansize_filter,
};

use std::env;
use tera::Tera;

use tokio::signal;

#[tokio::main]
async fn main() {
    /* initialize tracing */
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    /* load config */
    let args: Vec<String> = env::args().collect();
    let explicit_config = args.get(1).map(|s| s.as_str());

    let data = config::load_config(explicit_config);

    /* setup templates and context */
    let mut tera = Tera::default();
    tera.register_filter("humansize", humansize_filter);
    tera.load_from_glob("html/**/*.html").unwrap();
    let mut context = Context::new();
    context.insert("title", "stignore-manager");
    context.insert("copyright", "© 2024 Dalmura");

    let app_state = AppState {
        engine: TeraEngine(tera),
        context,
        config: data.clone(),
        agent_client: agent_client::AgentClient::with_timeout(data.manager.agent_timeout_seconds),
        disabled_agents: std::sync::Arc::new(std::sync::RwLock::new(
            std::collections::HashSet::new(),
        )),
    };

    let app = create_app(app_state);

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
