mod auth;
mod backups;
mod files;
mod logs;
mod metrics;
mod models;
mod moderation;
mod modrinth;
mod players;
mod servers;
mod settings;
mod users;

use crate::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
use std::sync::Arc;

pub fn router<S>(state: Arc<AppState>) -> Router<S> {
    let servers_routes = servers::routes()
        .merge(moderation::routes())
        .merge(files::routes())
        .merge(backups::routes())
        .merge(logs::routes())
        .with_state(state.clone());

    let protected = Router::new()
        .nest("/servers",  servers_routes)
        .nest("/players",  players::router(state.clone()).with_state(state.clone()))
        .nest("/metrics",  metrics::router(state.clone()).with_state(state.clone()))
        .nest("/modrinth", modrinth::router(state.clone()).with_state(state.clone()))
        .nest("",          settings::router(state.clone()).with_state(state.clone()))
        .nest("/users",    users::routes().with_state(state.clone()))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let public = Router::new()
        .nest("/auth", auth::routes().with_state(state.clone()))
        .route("/health", get(health));

    Router::new()
        .merge(protected)
        .merge(public)
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    let Some(token) = token else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let Some(user_id) = state.auth.resolve_token(&token).await else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let Some(user) = state.auth.find_by_id(&user_id).await else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}

