use crate::AppState;
use crate::auth::{Role, User};
use crate::config::AppConfig;
use axum::{routing::put, Extension, Json, Router, extract::State, http::StatusCode, response::IntoResponse};
use std::sync::Arc;

#[derive(serde::Serialize)]
struct SettingsResponse {
    #[serde(flatten)]
    config: AppConfig,
    device_hostname: String,
}

fn detect_device_hostname() -> String {
    for key in ["NOVABOX_DEVICE_HOSTNAME", "COMPUTERNAME", "HOSTNAME"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    if let Ok(output) = std::process::Command::new("hostname").output() {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                let trimmed = stdout.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }

    "localhost".to_string()
}

pub fn router<S>(state: Arc<AppState>) -> Router<S> {
    Router::new()
        .route("/settings", put(update_settings).get(get_settings))
        .with_state(state)
}

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    Extension(_user): Extension<User>,
) -> impl IntoResponse {
    let config = state.config.read().await;
    Json(SettingsResponse {
        config: config.clone(),
        device_hostname: detect_device_hostname(),
    }).into_response()
}

pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Json(req): Json<AppConfig>,
) -> impl IntoResponse {
    if user.role != Role::Admin {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Admin required" })),
        )
            .into_response();
    }
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
    Json(SettingsResponse {
        config: config.clone(),
        device_hostname: detect_device_hostname(),
    }).into_response()
}
