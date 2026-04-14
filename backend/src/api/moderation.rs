use crate::AppState;
use crate::api::models::ErrorResponse;
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json,
};
use bollard::container::InspectContainerOptions;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{Duration, timeout};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:id/whitelist",      get(whitelist_list).post(whitelist_add))
        .route("/:id/whitelist/:name", axum::routing::delete(whitelist_remove))
        .route("/:id/whitelist/state", get(whitelist_state_get).put(whitelist_state_set))
        .route("/:id/bans",            get(bans_list).post(bans_add))
        .route("/:id/bans/:name",      axum::routing::delete(bans_remove))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WhitelistEntry {
    pub uuid:    String,
    pub name:    String,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub expires: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BanEntry {
    pub uuid:    String,
    pub name:    String,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub source:  String,
    #[serde(default = "default_expires")]
    pub expires: String,
    #[serde(default = "default_reason")]
    pub reason:  String,
}

fn default_expires() -> String { "forever".to_string() }
fn default_reason()  -> String { "Banned by an operator.".to_string() }

#[derive(Deserialize)]
struct AddWhitelistRequest {
    name: String,
}

#[derive(Deserialize)]
struct SetWhitelistStateRequest {
    enabled: bool,
}

#[derive(Serialize)]
struct WhitelistStateResponse {
    enabled: bool,
}

#[derive(Deserialize)]
struct AddBanRequest {
    name:   String,
    #[serde(default)]
    reason: String,
}

fn whitelist_path(server_id: &str) -> String {
    format!("/servers/{}/whitelist.json", server_id)
}

fn bans_path(server_id: &str) -> String {
    format!("/servers/{}/banned-players.json", server_id)
}

fn server_properties_path(server_id: &str) -> String {
    format!("/servers/{}/server.properties", server_id)
}

fn parse_whitelist_enabled(properties: &str) -> bool {
    for line in properties.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("white-list=") {
            return value.trim().eq_ignore_ascii_case("true");
        }
    }
    false
}

async fn whitelist_enabled_from_properties(server_id: &str) -> bool {
    let path = server_properties_path(server_id);
    match tokio::fs::read_to_string(path).await {
        Ok(content) => parse_whitelist_enabled(&content),
        Err(_) => false,
    }
}

async fn set_whitelist_in_properties(server_id: &str, enabled: bool) -> Result<(), String> {
    let path = server_properties_path(server_id);
    let existing = tokio::fs::read_to_string(&path).await.unwrap_or_default();
    let mut out_lines: Vec<String> = Vec::new();
    let mut replaced = false;

    if !existing.is_empty() {
        for line in existing.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with('#') && trimmed.starts_with("white-list=") {
                out_lines.push(format!("white-list={}", if enabled { "true" } else { "false" }));
                replaced = true;
            } else {
                out_lines.push(line.to_string());
            }
        }
    }

    if !replaced {
        out_lines.push(format!("white-list={}", if enabled { "true" } else { "false" }));
    }

    let mut body = out_lines.join("\n");
    body.push('\n');
    tokio::fs::write(path, body).await.map_err(|e| e.to_string())
}

async fn read_json_list<T: for<'de> Deserialize<'de>>(path: &str) -> Vec<T> {
    match tokio::fs::read_to_string(path).await {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => vec![],
    }
}

async fn write_json_list<T: Serialize>(path: &str, list: &[T]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(list)
        .map_err(|e| e.to_string())?;
    tokio::fs::write(path, json)
        .await
        .map_err(|e| e.to_string())
}

async fn server_rcon(
    state: &Arc<AppState>,
    server_id: &str,
) -> Option<crate::rcon::RconClient> {
    let row = sqlx::query!(
        "SELECT rcon_port, rcon_password, container_id, status FROM servers WHERE id = ?",
        server_id
    )
    .fetch_optional(&state.db)
    .await
    .ok()??;

    if row.status != "running" {
        return None;
    }
    let container_id = row.container_id?;
    let network = std::env::var("DOCKER_NETWORK")
        .unwrap_or_else(|_| "novabox-mc-net".to_string());

    let ip = state
        .docker
        .inspect_container(&container_id, None::<InspectContainerOptions>)
        .await
        .ok()
        .and_then(|i| i.network_settings)
        .and_then(|n| n.networks)
        .and_then(|nets| nets.get(&network).and_then(|e| e.ip_address.clone()))
        .filter(|ip| !ip.is_empty())?;

    let password = row.rcon_password.clone();
    timeout(
        Duration::from_secs(3),
        crate::rcon::RconClient::connect(&ip, row.rcon_port as u16, &password),
    )
    .await
    .ok()
    .and_then(|r| r.ok())
}

async fn rcon_cmd(state: &Arc<AppState>, server_id: &str, cmd: &str) {
    if let Some(mut rcon) = server_rcon(state, server_id).await {
        match timeout(Duration::from_secs(4), rcon.command(cmd)).await {
            Ok(Ok(out)) => tracing::debug!(server_id=%server_id, cmd=%cmd, out=%out, "RCON ok"),
            Ok(Err(e))  => tracing::warn!(server_id=%server_id, "RCON cmd failed: {e}"),
            Err(_)      => tracing::warn!(server_id=%server_id, "RCON cmd timed out"),
        }
    }
}

async fn whitelist_list(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let list: Vec<WhitelistEntry> = read_json_list(&whitelist_path(&id)).await;
    Json(list)
}

async fn whitelist_state_get(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let enabled = whitelist_enabled_from_properties(&id).await;
    Json(WhitelistStateResponse { enabled })
}

async fn whitelist_state_set(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<SetWhitelistStateRequest>,
) -> impl IntoResponse {
    if let Err(e) = set_whitelist_in_properties(&id, req.enabled).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response();
    }

    if req.enabled {
        rcon_cmd(&state, &id, "whitelist on").await;
        rcon_cmd(&state, &id, "whitelist reload").await;
    } else {
        rcon_cmd(&state, &id, "whitelist off").await;
    }

    Json(WhitelistStateResponse {
        enabled: req.enabled,
    })
    .into_response()
}

async fn whitelist_add(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<AddWhitelistRequest>,
) -> impl IntoResponse {
    let path = whitelist_path(&id);
    let mut list: Vec<WhitelistEntry> = read_json_list(&path).await;

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Name required"))).into_response();
    }
    if list.iter().any(|e| e.name.eq_ignore_ascii_case(&name)) {
        return (StatusCode::CONFLICT, Json(ErrorResponse::new("Player already whitelisted"))).into_response();
    }

    list.push(WhitelistEntry {
        uuid:    String::new(),
        name:    name.clone(),
        created: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S %z").to_string(),
        expires: String::new(),
    });

    if let Err(e) = write_json_list(&path, &list).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response();
    }

    rcon_cmd(&state, &id, &format!("whitelist add {}", name)).await;
    rcon_cmd(&state, &id, "whitelist reload").await;

    Json(list).into_response()
}

async fn whitelist_remove(
    State(state): State<Arc<AppState>>,
    Path((id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let path = whitelist_path(&id);
    let mut list: Vec<WhitelistEntry> = read_json_list(&path).await;
    let before = list.len();
    list.retain(|e| !e.name.eq_ignore_ascii_case(&name));

    if list.len() == before {
        return (StatusCode::NOT_FOUND, Json(ErrorResponse::new("Player not in whitelist"))).into_response();
    }

    if let Err(e) = write_json_list(&path, &list).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response();
    }

    rcon_cmd(&state, &id, &format!("whitelist remove {}", name)).await;
    rcon_cmd(&state, &id, "whitelist reload").await;

    Json(list).into_response()
}

async fn bans_list(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let list: Vec<BanEntry> = read_json_list(&bans_path(&id)).await;
    Json(list)
}

async fn bans_add(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<AddBanRequest>,
) -> impl IntoResponse {
    let path = bans_path(&id);
    let mut list: Vec<BanEntry> = read_json_list(&path).await;

    let name   = req.name.trim().to_string();
    let reason = if req.reason.trim().is_empty() {
        "Banned by an operator.".to_string()
    } else {
        req.reason.trim().to_string()
    };

    if name.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Name required"))).into_response();
    }
    if list.iter().any(|e| e.name.eq_ignore_ascii_case(&name)) {
        return (StatusCode::CONFLICT, Json(ErrorResponse::new("Player already banned"))).into_response();
    }

    list.push(BanEntry {
        uuid:    String::new(),
        name:    name.clone(),
        created: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S %z").to_string(),
        source:  "NovaBox".to_string(),
        expires: "forever".to_string(),
        reason:  reason.clone(),
    });

    if let Err(e) = write_json_list(&path, &list).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response();
    }

    rcon_cmd(&state, &id, &format!("ban {} {}", name, reason)).await;

    Json(list).into_response()
}

async fn bans_remove(
    State(state): State<Arc<AppState>>,
    Path((id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let path = bans_path(&id);
    let mut list: Vec<BanEntry> = read_json_list(&path).await;
    let before = list.len();
    list.retain(|e| !e.name.eq_ignore_ascii_case(&name));

    if list.len() == before {
        return (StatusCode::NOT_FOUND, Json(ErrorResponse::new("Player not banned"))).into_response();
    }

    if let Err(e) = write_json_list(&path, &list).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response();
    }

    rcon_cmd(&state, &id, &format!("pardon {}", name)).await;

    Json(list).into_response()
}
