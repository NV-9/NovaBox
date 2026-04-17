use crate::AppState;
use crate::api::models::*;
use crate::auth::{User, PERM_SERVERS_PLAYERS};
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Extension, Json,
};
use sha2::{Digest, Sha256};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub fn router<S>(state: Arc<AppState>) -> Router<S> {
    Router::new()
        .route("/:server_id/sessions", get(list_sessions))
        .route("/:server_id/online", get(online_players))
        .with_state(state)
}

#[derive(Deserialize)]
struct Pagination {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}
fn default_limit() -> i64 { 50 }

async fn list_sessions(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(server_id): Path<String>,
    Query(page): Query<Pagination>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_PLAYERS) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.players"))).into_response();
    }
    let rows = sqlx::query!(
        "SELECT id, server_id, player_uuid, player_name, joined_at, left_at, duration_seconds
         FROM player_sessions WHERE server_id = ? ORDER BY joined_at DESC LIMIT ? OFFSET ?",
        server_id, page.limit, page.offset
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => {
            let sessions: Vec<PlayerSession> = rows.into_iter().map(|r| PlayerSession {
                id:               s(r.id),
                server_id:        s(r.server_id),
                player_uuid:      s(r.player_uuid),
                player_name:      s(r.player_name),
                joined_at:        s(r.joined_at),
                left_at:          r.left_at,
                duration_seconds: r.duration_seconds,
            }).collect();
            Json(sessions).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response(),
    }
}

async fn online_players(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(server_id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_PLAYERS) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.players"))).into_response();
    }
    let live_names = match fetch_live_online_names(&state, &server_id).await {
        Ok(names) => names,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new(format!("Live player query failed: {e}"))),
            )
                .into_response();
        }
    };

    if let Err(e) = reconcile_online_sessions(&state, &server_id, &live_names).await {
        tracing::warn!(server_id=%server_id, error=%e, "Failed to reconcile player sessions");
    }

    let rows = sqlx::query!(
        "SELECT id, server_id, player_uuid, player_name, joined_at, left_at, duration_seconds
         FROM player_sessions
         WHERE server_id = ? AND left_at IS NULL
         ORDER BY joined_at",
        server_id
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => {
            let sessions: Vec<PlayerSession> = rows
                .into_iter()
                .map(|r| PlayerSession {
                    id: s(r.id),
                    server_id: s(r.server_id),
                    player_uuid: s(r.player_uuid),
                    player_name: s(r.player_name),
                    joined_at: s(r.joined_at),
                    left_at: r.left_at,
                    duration_seconds: r.duration_seconds,
                })
                .collect();
            Json(sessions).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response(),
    }
}

async fn fetch_live_online_names(state: &Arc<AppState>, server_id: &str) -> Result<Vec<String>, String> {
    let row = sqlx::query!(
        "SELECT status FROM servers WHERE id = ?",
        server_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("DB query failed: {e}"))?
    .ok_or_else(|| "Server not found".to_string())?;

    if row.status != "running" {
        return Ok(vec![]);
    }

    let out = state
        .rcon_command(server_id, "list")
        .await
        .map_err(|e| format!("RCON list failed: {e}"))?;

    Ok(parse_online_names(&out))
}

fn parse_online_names(output: &str) -> Vec<String> {
    let normalized = output.replace('\r', "").replace('\n', " ");
    let names = normalized
        .split_once(':')
        .map(|(_, right)| right.trim())
        .unwrap_or("");

    if names.is_empty() {
        return vec![];
    }

    names
        .split(',')
        .map(|n| n.trim())
        .filter(|n| !n.is_empty())
        .map(|n| n.to_string())
        .collect()
}

async fn reconcile_online_sessions(
    state: &Arc<AppState>,
    server_id: &str,
    live_names: &[String],
) -> Result<(), String> {
    let live_lower: HashSet<String> = live_names.iter().map(|n| n.to_lowercase()).collect();

    let open_rows = sqlx::query!(
        "SELECT id, player_name FROM player_sessions
         WHERE server_id = ? AND left_at IS NULL
         ORDER BY joined_at DESC",
        server_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Open sessions query failed: {e}"))?;

    let mut seen_live = HashSet::<String>::new();
    for row in open_rows {
        let id = row.id;
        let name = row.player_name;
        let lower = name.to_lowercase();

        let should_close = !live_lower.contains(&lower) || seen_live.contains(&lower);
        if should_close {
            let _ = sqlx::query!(
                "UPDATE player_sessions
                 SET left_at = datetime('now'),
                     duration_seconds = CAST((julianday('now') - julianday(joined_at)) * 86400 AS INTEGER)
                 WHERE id = ?",
                id
            )
            .execute(&state.db)
            .await;
        } else {
            seen_live.insert(lower);
        }
    }

    let current_open = sqlx::query!(
        "SELECT player_name FROM player_sessions WHERE server_id = ? AND left_at IS NULL",
        server_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Current open sessions query failed: {e}"))?;

    let open_now: HashSet<String> = current_open
        .into_iter()
        .map(|r| r.player_name)
        .map(|n| n.to_lowercase())
        .collect();

    let uuid_by_name = load_uuid_map(state, server_id).await;

    for name in live_names {
        let lower = name.to_lowercase();
        if open_now.contains(&lower) {
            continue;
        }

        let player_uuid = uuid_by_name
            .get(&lower)
            .cloned()
            .unwrap_or_else(|| pseudo_uuid_from_name(name));

        let id = uuid::Uuid::new_v4().to_string();
        let _ = sqlx::query!(
            "INSERT INTO player_sessions (id, server_id, player_uuid, player_name)
             VALUES (?, ?, ?, ?)",
            id,
            server_id,
            player_uuid,
            name
        )
        .execute(&state.db)
        .await;
    }

    Ok(())
}

async fn load_uuid_map(_state: &Arc<AppState>, server_id: &str) -> HashMap<String, String> {
    let path = format!("/servers/{}/usercache.json", server_id);
    let text = match tokio::fs::read_to_string(path).await {
        Ok(t) => t,
        Err(_) => return HashMap::new(),
    };

    let parsed: Vec<serde_json::Value> = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };

    let mut out = HashMap::new();
    for item in parsed {
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("").trim();
        let uuid = item.get("uuid").and_then(|v| v.as_str()).unwrap_or("").trim();
        if name.is_empty() || uuid.is_empty() {
            continue;
        }
        out.insert(name.to_lowercase(), normalize_uuid(uuid));
    }
    out
}

fn normalize_uuid(raw: &str) -> String {
    let compact = raw.replace('-', "").to_lowercase();
    if compact.len() != 32 {
        return raw.to_string();
    }
    format!(
        "{}-{}-{}-{}-{}",
        &compact[0..8],
        &compact[8..12],
        &compact[12..16],
        &compact[16..20],
        &compact[20..32]
    )
}

fn pseudo_uuid_from_name(name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let hex = hex::encode(hasher.finalize());
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}
