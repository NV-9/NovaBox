mod api;
mod auth;
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
use axum::http::HeaderValue;
use tower_http::cors::{AllowOrigin, CorsLayer, Any};
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

    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "/app/data".to_string());

    let pool              = db::init(&db_url).await?;
    let docker            = docker::init().await?;
    let servers_host_path = docker::resolve_servers_host_path(&docker).await;
    let config            = config::AppConfig::load().await;
    tracing::info!(domain=%config.domain, velocity=%config.velocity_enabled, "Loaded AppConfig");
    let state = Arc::new(AppState::new(pool, docker, servers_host_path, config, &data_dir));

    velocity::regenerate(&state).await;

    docker::monitor::auto_start_servers(state.clone()).await;

    let state_clone = state.clone();
    tokio::spawn(async move {
        docker::monitor::run(state_clone).await;
    });

    let idle_secs = std::env::var("RCON_IDLE_TIMEOUT_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(120);
    let state_clone = state.clone();
    tokio::spawn(async move {
        let idle = std::time::Duration::from_secs(idle_secs.max(30));
        loop {
            state_clone.prune_idle_rcon(idle).await;
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });

    let cors = match std::env::var("ALLOWED_ORIGINS").as_deref() {
        Ok("*") => {
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        }
        Ok(origins_str) => {
            let root_domains: Vec<String> = origins_str
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();

            let mut allowed_origins = vec![];
            for domain in &root_domains {
                allowed_origins.push(format!("http://{}", domain));
                allowed_origins.push(format!("https://{}", domain));
            }

            let list: Vec<HeaderValue> = allowed_origins
                .iter()
                .filter_map(|o| o.parse().ok())
                .collect();

            if list.is_empty() {
                tracing::warn!("Failed to parse ALLOWED_ORIGINS, allowing any origin");
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any)
            } else {
                tracing::info!("CORS allowing origins: {:?}", root_domains);
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any)
            }
        }
        Err(_) => {
            let defaults = [
                "http://localhost:5173",
                "http://localhost:8080",
                "http://127.0.0.1:8080",
            ];
            let list: Vec<HeaderValue> = defaults.iter()
                .filter_map(|o| o.parse().ok())
                .collect();
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(list))
                .allow_methods(Any)
                .allow_headers(Any)
        }
    };

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
