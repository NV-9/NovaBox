mod api;
mod config;
mod db;
mod docker;
mod mc_ping;
mod rcon;
mod state;
mod velocity;
mod ws;

use anyhow::Result;
use axum::Router;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "novabox=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:///app/data/novabox.db".to_string());

    let pool              = db::init(&db_url).await?;
    let docker            = docker::init().await?;
    let servers_host_path = docker::resolve_servers_host_path(&docker).await;
    let config            = config::AppConfig::load().await;
    tracing::info!(domain=%config.domain, velocity=%config.velocity_enabled, "Loaded AppConfig");
    let state = Arc::new(AppState::new(pool, docker, servers_host_path, config));

    velocity::regenerate(&state).await;

    docker::monitor::auto_start_servers(state.clone()).await;

    let state_clone = state.clone();
    tokio::spawn(async move {
        docker::monitor::run(state_clone).await;
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest("/api", api::router(state.clone()))
        .nest("/ws", ws::router(state.clone()))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    let addr = format!("{}:{}", host, port);
    tracing::info!("NovaBox master node listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
