use crate::AppState;
use crate::api::models::ErrorResponse;
use crate::auth::{User, PERM_SERVERS_MODERATION};
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:id/whitelist",      get(whitelist_list).post(whitelist_add))
        .route("/:id/whitelist/:name", axum::routing::delete(whitelist_remove))
        .route("/:id/whitelist/state", get(whitelist_state_get).put(whitelist_state_set))
        .route("/:id/bans",            get(bans_list).post(bans_add))
        .route("/:id/bans/:name",      axum::routing::delete(bans_remove))
        .route("/:id/ops",             get(ops_list).post(ops_add))
        .route("/:id/ops/:name",       axum::routing::delete(ops_remove))
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

async fn rcon_cmd(state: &Arc<AppState>, server_id: &str, cmd: &str) {
    match state.rcon_command(server_id, cmd).await {
        Ok(out) => tracing::debug!(server_id=%server_id, cmd=%cmd, out=%out, "RCON ok"),
        Err(e) => tracing::warn!(server_id=%server_id, cmd=%cmd, "RCON cmd failed: {e}"),
    }
}

async fn sync_ops_from_live(
    state: &Arc<AppState>,
    server_id: &str,
    path: &str,
    current: Vec<OpEntry>,
) -> Vec<OpEntry> {
    let live = match state.rcon_command(server_id, "ops").await {
        Ok(out) => parse_live_ops_names(&out),
        Err(e) => {
            tracing::warn!(server_id=%server_id, error=%e, "Could not read live ops list over RCON");
            return current;
        }
    };

    let reconciled = reconcile_ops_list(current.clone(), live);
    if let Err(e) = write_json_list(path, &reconciled).await {
        tracing::warn!(server_id=%server_id, error=%e, "Failed to persist reconciled ops list");
        return current;
    }
    reconciled
}

async fn whitelist_list(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_MODERATION) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.moderation"))).into_response();
    }
    let list: Vec<WhitelistEntry> = read_json_list(&whitelist_path(&id)).await;
    Json(list).into_response()
}

async fn whitelist_state_get(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_MODERATION) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.moderation"))).into_response();
    }
    let enabled = whitelist_enabled_from_properties(&id).await;
    Json(WhitelistStateResponse { enabled }).into_response()
}

async fn whitelist_state_set(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(req): Json<SetWhitelistStateRequest>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_MODERATION) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.moderation"))).into_response();
    }
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
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(req): Json<AddWhitelistRequest>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_MODERATION) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.moderation"))).into_response();
    }
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
    Extension(user): Extension<User>,
    Path((id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_MODERATION) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.moderation"))).into_response();
    }
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
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_MODERATION) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.moderation"))).into_response();
    }
    let list: Vec<BanEntry> = read_json_list(&bans_path(&id)).await;
    Json(list).into_response()
}

async fn bans_add(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(req): Json<AddBanRequest>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_MODERATION) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.moderation"))).into_response();
    }
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
    Extension(user): Extension<User>,
    Path((id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_MODERATION) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.moderation"))).into_response();
    }
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpEntry {
    pub uuid:            String,
    pub name:            String,
    #[serde(default)]
    pub level:           i64,
    #[serde(
        default = "default_bypass_limit",
        rename = "bypassesPlayerLimit",
        alias = "bypasses_player_limit"
    )]
    pub bypasses_player_limit: bool,
}

fn default_bypass_limit() -> bool { false }

#[derive(Deserialize)]
struct AddOpRequest {
    name: String,
}

fn ops_path(server_id: &str) -> String {
    format!("/servers/{}/ops.json", server_id)
}

fn parse_live_ops_names(output: &str) -> Vec<String> {
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
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .collect()
}

fn reconcile_ops_list(mut file_list: Vec<OpEntry>, live_names: Vec<String>) -> Vec<OpEntry> {
    let live_lower: HashSet<String> = live_names.iter().map(|v| v.to_lowercase()).collect();

    if !live_lower.is_empty() {
        file_list.retain(|e| live_lower.contains(&e.name.to_lowercase()));
    }

    let existing_lower: HashSet<String> = file_list.iter().map(|e| e.name.to_lowercase()).collect();
    for name in live_names {
        if existing_lower.contains(&name.to_lowercase()) {
            continue;
        }
        file_list.push(OpEntry {
            uuid: String::new(),
            name,
            level: 4,
            bypasses_player_limit: false,
        });
    }

    file_list
}

async fn ops_list(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_MODERATION) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.moderation"))).into_response();
    }
    let path = ops_path(&id);
    let mut list: Vec<OpEntry> = read_json_list(&path).await;

    if let Ok(out) = state.rcon_command(&id, "ops").await {
        let live_names = parse_live_ops_names(&out);
        let reconciled = reconcile_ops_list(list.clone(), live_names);
        if reconciled.len() != list.len() || reconciled.iter().zip(list.iter()).any(|(a, b)| a.name != b.name || a.level != b.level || a.uuid != b.uuid || a.bypasses_player_limit != b.bypasses_player_limit) {
            if let Err(e) = write_json_list(&path, &reconciled).await {
                tracing::warn!(server_id=%id, error=%e, "Failed to persist reconciled ops list");
            }
            list = reconciled;
        }
    }

    Json(list).into_response()
}

async fn ops_add(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(req): Json<AddOpRequest>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_MODERATION) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.moderation"))).into_response();
    }
    let path = ops_path(&id);
    let mut list: Vec<OpEntry> = read_json_list(&path).await;

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Name required"))).into_response();
    }
    if list.iter().any(|e| e.name.eq_ignore_ascii_case(&name)) {
        return (StatusCode::CONFLICT, Json(ErrorResponse::new("Player is already an op"))).into_response();
    }

    let is_selector = name.starts_with('@');

    if !is_selector {
        list.push(OpEntry {
            uuid:                 String::new(),
            name:                 name.clone(),
            level:                4,
            bypasses_player_limit: false,
        });

        if let Err(e) = write_json_list(&path, &list).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response();
        }
    }

    rcon_cmd(&state, &id, &format!("op {}", name)).await;
    let synced = sync_ops_from_live(&state, &id, &path, list).await;

    Json(synced).into_response()
}

async fn ops_remove(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path((id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_MODERATION) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.moderation"))).into_response();
    }
    let path = ops_path(&id);
    let mut list: Vec<OpEntry> = read_json_list(&path).await;
    let before = list.len();
    list.retain(|e| !e.name.eq_ignore_ascii_case(&name));

    if list.len() == before {
        return (StatusCode::NOT_FOUND, Json(ErrorResponse::new("Player is not an op"))).into_response();
    }

    if let Err(e) = write_json_list(&path, &list).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response();
    }

    rcon_cmd(&state, &id, &format!("deop {}", name)).await;
    let synced = sync_ops_from_live(&state, &id, &path, list).await;

    Json(synced).into_response()
}
