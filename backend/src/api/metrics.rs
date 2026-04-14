use crate::AppState;
use crate::api::models::*;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

pub fn router<S>(state: Arc<AppState>) -> Router<S> {
    Router::new()
        .route("/:server_id", get(get_metrics))
        .route("/:server_id/summary", get(summary))
        .with_state(state)
}

#[derive(Deserialize)]
struct TimeRange {
    #[serde(default = "default_hours")]
    hours: i64,
}
fn default_hours() -> i64 { 24 }

async fn get_metrics(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(range): Query<TimeRange>,
) -> impl IntoResponse {
    let since = format!("-{} hours", range.hours);
    let rows = sqlx::query!(
        "SELECT timestamp, online_players, cpu_percent, memory_mb, tps FROM server_metrics
         WHERE server_id = ? AND timestamp >= datetime('now', ?)
         ORDER BY timestamp ASC",
        server_id,
        since,
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => {
            let points: Vec<MetricPoint> = rows.into_iter().map(|r| MetricPoint {
                timestamp:      s(r.timestamp),
                online_players: r.online_players,
                cpu_percent:    r.cpu_percent,
                memory_mb:      r.memory_mb,
                tps:            r.tps,
            }).collect();
            Json(points).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response(),
    }
}

async fn summary(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
) -> impl IntoResponse {
    let total_sessions = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM player_sessions WHERE server_id = ?",
        server_id
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let unique_players = sqlx::query_scalar!(
        "SELECT COUNT(DISTINCT player_uuid) FROM player_sessions WHERE server_id = ?",
        server_id
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let peak_players = sqlx::query_scalar!(
        "SELECT MAX(online_players) FROM server_metrics WHERE server_id = ?",
        server_id
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(None)
    .unwrap_or(0);

    Json(serde_json::json!({
        "total_sessions": total_sessions,
        "unique_players": unique_players,
        "peak_players":   peak_players,
    })).into_response()
}
