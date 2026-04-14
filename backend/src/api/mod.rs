mod models;
mod moderation;
mod servers;
mod players;
mod metrics;
mod modrinth;
mod settings;

use crate::AppState;
use axum::{Router, routing::get};
use std::sync::Arc;

pub fn router<S>(state: Arc<AppState>) -> Router<S> {
    let servers_routes = servers::routes()
        .merge(moderation::routes())
        .with_state(state.clone());

    Router::new()
        .nest("/servers",  servers_routes)
        .nest("/players",  players::router(state.clone()).with_state(state.clone()))
        .nest("/metrics",  metrics::router(state.clone()).with_state(state.clone()))
        .nest("/modrinth", modrinth::router(state.clone()).with_state(state.clone()))
        .nest("",          settings::router(state.clone()).with_state(state.clone()))
        .route("/health",  get(health))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
