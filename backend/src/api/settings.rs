use crate::AppState;
use crate::config::AppConfig;
use axum::{routing::put, Json, Router, extract::State, http::StatusCode, response::IntoResponse};
use std::sync::Arc;

pub fn router<S>(state: Arc<AppState>) -> Router<S> {
    Router::new()
        .route("/settings", put(update_settings).get(get_settings))
        .with_state(state)
}

pub async fn get_settings(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let config = state.config.read().await;
    Json(config.clone())
}

pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AppConfig>,
) -> impl IntoResponse {
    {
        let mut config = state.config.write().await;
        *config = req;
        if let Err(e) = config.save().await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    }

    crate::velocity::regenerate(&state).await;

    let config = state.config.read().await;
    Json(config.clone()).into_response()
}
