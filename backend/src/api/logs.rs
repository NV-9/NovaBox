use crate::AppState;
use crate::api::models::ErrorResponse;
use crate::auth::{User, PERM_SERVERS_CONSOLE};
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:id/logs", get(search_logs))
}

#[derive(Deserialize)]
struct LogQuery {
    q:     Option<String>,
    limit: Option<usize>,
}

#[derive(Serialize)]
struct LogLine {
    line: usize,
    text: String,
}

async fn search_logs(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Query(q): Query<LogQuery>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_CONSOLE) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.console"))).into_response();
    }
    let log_path = format!("/servers/{}/logs/latest.log", id);

    let content = match tokio::fs::read_to_string(&log_path).await {
        Ok(c)  => c,
        Err(e) => return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!("Log file not found: {e}"))),
        )
            .into_response(),
    };

    let limit = q.limit.unwrap_or(500).min(5000);
    let query = q.q.as_deref().unwrap_or("").to_lowercase();

    let mut results: Vec<LogLine> = content
        .lines()
        .enumerate()
        .filter(|(_, line)| query.is_empty() || line.to_lowercase().contains(&query))
        .map(|(i, line)| LogLine { line: i + 1, text: line.to_string() })
        .collect();

    if results.len() > limit {
        let skip = results.len() - limit;
        results.drain(..skip);
    }

    Json(results).into_response()
}
