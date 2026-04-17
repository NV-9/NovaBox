use crate::AppState;
use crate::api::models::*;
use crate::auth::{Role, User, PERM_SERVERS_CONSOLE, PERM_SERVERS_CREATE, PERM_SERVERS_DELETE, PERM_SERVERS_MODRINTH, PERM_SERVERS_POWER, PERM_SERVERS_SETTINGS, PERM_SERVERS_VIEW};
use crate::docker::container_name;
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Extension, Json,
};
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::{HostConfig, PortBinding};
use bollard::network::CreateNetworkOptions;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_servers).post(create_server))
        .route("/:id", get(get_server).put(update_server).delete(delete_server))
        .route("/:id/world-info", get(world_info))
        .route("/:id/world-settings", get(world_settings_get).put(world_settings_set))
        .route("/:id/modrinth-projects", get(get_modrinth_projects).put(set_modrinth_projects))
        .route("/:id/start", post(start_server))
        .route("/:id/stop", post(stop_server))
        .route("/:id/kill", post(kill_server))
        .route("/:id/restart", post(restart_server))
        .route("/:id/storage", get(storage_usage))
        .route("/:id/runtime", get(get_runtime_options).put(set_runtime_options))
        .route("/:id/stdin", post(send_stdin_command))
        .route("/:id/command", post(run_command))
        .route("/:id/apply-map", post(apply_map_switch))
        .route("/:id/map-config", get(get_map_config))
        .route("/:id/members", get(list_members).post(add_member))
        .route("/:id/members/:uid", delete(remove_member))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RuntimeOptions {
    #[serde(default)]
    min_memory_mb: Option<i64>,
    #[serde(default)]
    jvm_flags: Option<String>,
    #[serde(default)]
    pause_when_empty_seconds: Option<i64>,
}

#[derive(Debug, Serialize)]
struct StorageUsage {
    bytes: i64,
    mb: f64,
    gb: f64,
}

#[derive(Debug, Serialize)]
struct WorldInfo {
    difficulty: Option<String>,
    gamemode: Option<String>,
    simulation_distance: Option<i64>,
    view_distance: Option<i64>,
    white_list: Option<bool>,
    online_mode: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WorldSettings {
    #[serde(default)]
    difficulty: Option<String>,
    #[serde(default)]
    gamemode: Option<String>,
    #[serde(default)]
    simulation_distance: Option<i64>,
    #[serde(default)]
    view_distance: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ModrinthProjects {
    #[serde(default)]
    projects: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AddMemberRequest {
    username: String,
}

async fn check_membership(db: &sqlx::SqlitePool, server_id: &str, user: &User) -> bool {
    if user.role == Role::Admin {
        return true;
    }
    sqlx::query!(
        "SELECT 1 as v FROM server_members WHERE server_id = ? AND user_id = ?",
        server_id,
        user.id
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .is_some()
}

fn runtime_options_path(server_id: &str) -> String {
    format!("/servers/{}/novabox.runtime.json", server_id)
}

fn world_settings_path(server_id: &str) -> String {
    format!("/servers/{}/novabox.world.json", server_id)
}

fn server_properties_path(server_id: &str) -> String {
    format!("/servers/{}/server.properties", server_id)
}

fn modrinth_projects_path(server_id: &str) -> String {
    format!("/servers/{}/novabox.modrinth.json", server_id)
}

fn normalize_modrinth_projects(projects: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for raw in projects {
        let value = raw.trim().to_lowercase();
        if value.is_empty() {
            continue;
        }
        if seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

async fn read_modrinth_projects(server_id: &str) -> ModrinthProjects {
    let text = match tokio::fs::read_to_string(modrinth_projects_path(server_id)).await {
        Ok(t) => t,
        Err(_) => {
            return ModrinthProjects { projects: vec![] };
        }
    };

    let parsed = serde_json::from_str::<ModrinthProjects>(&text)
        .unwrap_or(ModrinthProjects { projects: vec![] });

    ModrinthProjects {
        projects: normalize_modrinth_projects(&parsed.projects),
    }
}

async fn write_modrinth_projects(server_id: &str, payload: ModrinthProjects) -> Result<(), String> {
    let cleaned = ModrinthProjects {
        projects: normalize_modrinth_projects(&payload.projects),
    };
    let body = serde_json::to_string_pretty(&cleaned).map_err(|e| e.to_string())?;
    tokio::fs::write(modrinth_projects_path(server_id), body)
        .await
        .map_err(|e| e.to_string())
}

fn parse_server_properties(text: &str) -> WorldInfo {
    let mut info = WorldInfo {
        difficulty: None,
        gamemode: None,
        simulation_distance: None,
        view_distance: None,
        white_list: None,
        online_mode: None,
    };

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else { continue };
        let key = key.trim();
        let value = value.trim();

        match key {
            "difficulty" => info.difficulty = Some(value.to_string()),
            "gamemode" => info.gamemode = Some(value.to_string()),
            "simulation-distance" => info.simulation_distance = value.parse::<i64>().ok(),
            "view-distance" => info.view_distance = value.parse::<i64>().ok(),
            "white-list" => info.white_list = Some(value.eq_ignore_ascii_case("true")),
            "online-mode" => info.online_mode = Some(value.eq_ignore_ascii_case("true")),
            _ => {}
        }
    }

    info
}

async fn read_runtime_options(server_id: &str) -> RuntimeOptions {
    let path = runtime_options_path(server_id);
    let text = match tokio::fs::read_to_string(path).await {
        Ok(t) => t,
        Err(_) => {
            return RuntimeOptions {
                min_memory_mb: None,
                jvm_flags: None,
                pause_when_empty_seconds: None,
            };
        }
    };

    serde_json::from_str(&text).unwrap_or(RuntimeOptions {
        min_memory_mb: None,
        jvm_flags: None,
        pause_when_empty_seconds: None,
    })
}

async fn read_world_settings(server_id: &str) -> WorldSettings {
    let path = world_settings_path(server_id);
    let text = match tokio::fs::read_to_string(path).await {
        Ok(t) => t,
        Err(_) => {
            return WorldSettings {
                difficulty: None,
                gamemode: None,
                simulation_distance: None,
                view_distance: None,
            };
        }
    };

    serde_json::from_str(&text).unwrap_or(WorldSettings {
        difficulty: None,
        gamemode: None,
        simulation_distance: None,
        view_distance: None,
    })
}

async fn write_world_settings(server_id: &str, mut settings: WorldSettings) -> Result<(), String> {
    if let Some(diff) = &settings.difficulty {
        let trimmed = diff.trim().to_lowercase();
        settings.difficulty = if trimmed.is_empty() { None } else { Some(trimmed) };
    }
    if let Some(mode) = &settings.gamemode {
        let trimmed = mode.trim().to_lowercase();
        settings.gamemode = if trimmed.is_empty() { None } else { Some(trimmed) };
    }
    if let Some(sim) = settings.simulation_distance {
        settings.simulation_distance = Some(sim.clamp(2, 32));
    }
    if let Some(view) = settings.view_distance {
        settings.view_distance = Some(view.clamp(2, 32));
    }

    let body = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    tokio::fs::write(world_settings_path(server_id), body).await.map_err(|e| e.to_string())
}

async fn ensure_server_properties(server_id: &str) -> Result<(), String> {
    let settings = read_world_settings(server_id).await;
    let path = server_properties_path(server_id);
    let existing = tokio::fs::read_to_string(&path).await.unwrap_or_default();
    let mut lines: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    fn push_kv(lines: &mut Vec<String>, seen: &mut std::collections::HashSet<String>, key: &str, value: String) {
        lines.push(format!("{}={}", key, value));
        seen.insert(key.to_string());
    }

    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            lines.push(line.to_string());
            continue;
        }
        let Some((key, _)) = trimmed.split_once('=') else {
            lines.push(line.to_string());
            continue;
        };
        let key = key.trim();
        if ["difficulty", "gamemode", "simulation-distance", "view-distance", "white-list", "online-mode"].contains(&key) {
            continue;
        }
        lines.push(line.to_string());
    }

    if let Some(difficulty) = settings.difficulty {
        push_kv(&mut lines, &mut seen, "difficulty", difficulty);
    }
    if let Some(gamemode) = settings.gamemode {
        push_kv(&mut lines, &mut seen, "gamemode", gamemode);
    }
    if let Some(sim) = settings.simulation_distance {
        push_kv(&mut lines, &mut seen, "simulation-distance", sim.to_string());
    }
    if let Some(view) = settings.view_distance {
        push_kv(&mut lines, &mut seen, "view-distance", view.to_string());
    }
    if !seen.contains("white-list") {
        push_kv(&mut lines, &mut seen, "white-list", "false".to_string());
    }
    if !seen.contains("online-mode") {
        push_kv(&mut lines, &mut seen, "online-mode", "true".to_string());
    }

    let mut body = lines.join("\n");
    body.push('\n');
    tokio::fs::write(&path, body).await.map_err(|e| e.to_string())
}

async fn world_info(
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_VIEW) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.view"))).into_response();
    }
    let path = server_properties_path(&id);
    let info = match tokio::fs::read_to_string(path).await {
        Ok(text) => parse_server_properties(&text),
        Err(_) => WorldInfo {
            difficulty: None,
            gamemode: None,
            simulation_distance: None,
            view_distance: None,
            white_list: None,
            online_mode: None,
        },
    };

    Json(info).into_response()
}

async fn world_settings_get(
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_SETTINGS) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.settings"))).into_response();
    }
    Json(read_world_settings(&id).await).into_response()
}

async fn world_settings_set(
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(settings): Json<WorldSettings>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_SETTINGS) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.settings"))).into_response();
    }
    if let Err(e) = write_world_settings(&id, settings.clone()).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response();
    }
    if let Err(e) = ensure_server_properties(&id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response();
    }
    Json(settings).into_response()
}

async fn get_modrinth_projects(
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_MODRINTH) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.modrinth"))).into_response();
    }
    Json(read_modrinth_projects(&id).await).into_response()
}

async fn set_modrinth_projects(
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(payload): Json<ModrinthProjects>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_MODRINTH) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.modrinth"))).into_response();
    }
    if let Err(e) = write_modrinth_projects(&id, payload).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response();
    }
    Json(read_modrinth_projects(&id).await).into_response()
}

async fn write_runtime_options(server_id: &str, mut opts: RuntimeOptions) -> Result<(), String> {
    let path = runtime_options_path(server_id);

    if let Some(v) = opts.min_memory_mb {
        opts.min_memory_mb = Some(v.max(128));
    }
    if let Some(flags) = &opts.jvm_flags {
        let trimmed = flags.trim();
        opts.jvm_flags = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        };
    }

    let body = serde_json::to_string_pretty(&opts).map_err(|e| e.to_string())?;
    tokio::fs::write(path, body).await.map_err(|e| e.to_string())
}

fn dir_size_bytes_sync(path: &PathBuf) -> u64 {
    let mut total: u64 = 0;
    let mut stack: Vec<PathBuf> = vec![path.clone()];

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(v) => v,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let p = entry.path();
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_file() {
                total = total.saturating_add(meta.len());
            } else if meta.is_dir() {
                stack.push(p);
            }
        }
    }

    total
}

async fn list_servers(State(state): State<Arc<AppState>>, Extension(user): Extension<User>) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_VIEW) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.view"))).into_response();
    }
    let rows = sqlx::query!(
        r#"SELECT id, name, description, container_id, status, loader, mc_version, port, rcon_port,
         max_players, memory_mb, map_mod, online_mode, auto_start, auto_start_delay,
         crash_detection, shutdown_timeout, show_on_status_page, data_dir, created_at, updated_at,
         (SELECT COUNT(*) FROM player_sessions WHERE server_id = servers.id AND left_at IS NULL) as "online_players!: i64"
         FROM servers ORDER BY created_at DESC"#
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => {
            let mut servers: Vec<Server> = rows
                .into_iter()
                .map(|r| {
                    let status = s(r.status.clone()).parse().unwrap_or(ServerStatus::Stopped);
                    Server {
                        id:                  s(r.id),
                        name:                s(r.name),
                        description:         s(r.description),
                        container_id:        r.container_id,
                        status,
                        loader:              s(r.loader).parse().unwrap_or(ServerLoader::Vanilla),
                        mc_version:          s(r.mc_version),
                        port:                r.port,
                        rcon_port:           r.rcon_port,
                        max_players:         r.max_players,
                        memory_mb:           r.memory_mb,
                        map_mod:             r.map_mod,
                        online_mode:         r.online_mode != 0,
                        auto_start:          r.auto_start != 0,
                        auto_start_delay:    r.auto_start_delay,
                        crash_detection:     r.crash_detection != 0,
                        shutdown_timeout:    r.shutdown_timeout,
                        show_on_status_page: r.show_on_status_page != 0,
                        online_players:      r.online_players,
                        data_dir:            s(r.data_dir),
                        created_at:          s(r.created_at),
                        updated_at:          s(r.updated_at),
                    }
                })
                .collect();

            if user.role != Role::Admin {
                let member_ids: std::collections::HashSet<String> = sqlx::query!(
                    "SELECT server_id FROM server_members WHERE user_id = ?",
                    user.id
                )
                .fetch_all(&state.db)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|r| r.server_id)
                .collect();
                servers.retain(|srv| member_ids.contains(&srv.id));
            }

            Json(servers).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response(),
    }
}

async fn get_server(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_VIEW) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.view"))).into_response();
    }
    if !check_membership(&state.db, &id, &user).await {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Access denied"))).into_response();
    }
    let row = sqlx::query!(
        r#"SELECT id, name, description, container_id, status, loader, mc_version, port, rcon_port,
         max_players, memory_mb, map_mod, online_mode, auto_start, auto_start_delay,
         crash_detection, shutdown_timeout, show_on_status_page, data_dir, created_at, updated_at,
         (SELECT COUNT(*) FROM player_sessions WHERE server_id = servers.id AND left_at IS NULL) as "online_players!: i64"
         FROM servers WHERE id = ?"#,
        id
    )
    .fetch_optional(&state.db)
    .await;

    match row {
        Ok(Some(r)) => {
            let status = s(r.status.clone()).parse().unwrap_or(ServerStatus::Stopped);
            Json(Server {
                id:                  s(r.id),
                name:                s(r.name),
                description:         s(r.description),
                container_id:        r.container_id,
                status,
                loader:              s(r.loader).parse().unwrap_or(ServerLoader::Vanilla),
                mc_version:          s(r.mc_version),
                port:                r.port,
                rcon_port:           r.rcon_port,
                max_players:         r.max_players,
                memory_mb:           r.memory_mb,
                map_mod:             r.map_mod,
                online_mode:         r.online_mode != 0,
                auto_start:          r.auto_start != 0,
                auto_start_delay:    r.auto_start_delay,
                crash_detection:     r.crash_detection != 0,
                shutdown_timeout:    r.shutdown_timeout,
                show_on_status_page: r.show_on_status_page != 0,
                online_players:      r.online_players,
                data_dir:            s(r.data_dir),
                created_at:          s(r.created_at),
                updated_at:          s(r.updated_at),
            })
            .into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(ErrorResponse::new("Server not found"))).into_response(),
        Err(e)   => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response(),
    }
}

async fn create_server(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Json(req): Json<CreateServerRequest>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_CREATE) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.create"))).into_response();
    }
    let id            = Uuid::new_v4().to_string();
    let rcon_password = Uuid::new_v4().to_string().replace('-', "");
    let rcon_port     = allocate_rcon_port(&state).await;
    let data_dir      = format!("/servers/{}", id);

    let online_mode_int:         i64 = if req.online_mode         { 1 } else { 0 };
    let auto_start_int:          i64 = if req.auto_start          { 1 } else { 0 };
    let crash_detection_int:     i64 = if req.crash_detection     { 1 } else { 0 };
    let show_on_status_page_int: i64 = if req.show_on_status_page { 1 } else { 0 };
    let result = sqlx::query!(
        "INSERT INTO servers (id, name, description, loader, mc_version, port, rcon_port, rcon_password,
         max_players, memory_mb, map_mod, online_mode, auto_start, auto_start_delay,
         crash_detection, shutdown_timeout, show_on_status_page, data_dir)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        id, req.name, req.description, req.loader, req.mc_version,
        req.port, rcon_port, rcon_password, req.max_players, req.memory_mb, req.map_mod,
        online_mode_int, auto_start_int, req.auto_start_delay, crash_detection_int,
        req.shutdown_timeout, show_on_status_page_int, data_dir,
    )
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => {
            let world_settings = WorldSettings {
                difficulty: req.difficulty.clone(),
                gamemode: req.gamemode.clone(),
                simulation_distance: req.simulation_distance,
                view_distance: req.view_distance,
            };
            let runtime_options = RuntimeOptions {
                min_memory_mb: None,
                jvm_flags: None,
                pause_when_empty_seconds: req.pause_when_empty_seconds,
            };
            let _ = write_world_settings(&id, world_settings).await;
            let _ = write_runtime_options(&id, runtime_options).await;
            let _ = ensure_server_properties(&id).await;

            let r = sqlx::query!(
                "SELECT id, name, description, container_id, status, loader, mc_version, port, rcon_port,
                 max_players, memory_mb, map_mod, online_mode, auto_start, auto_start_delay,
                 crash_detection, shutdown_timeout, show_on_status_page,
                 data_dir, created_at, updated_at FROM servers WHERE id = ?",
                id
            )
            .fetch_one(&state.db)
            .await
            .unwrap();

            (StatusCode::CREATED, Json(Server {
                id:                  s(r.id),
                name:                s(r.name),
                description:         s(r.description),
                container_id:        r.container_id,
                status:              ServerStatus::Stopped,
                loader:              s(r.loader).parse().unwrap_or(ServerLoader::Vanilla),
                mc_version:          s(r.mc_version),
                port:                r.port,
                rcon_port:           r.rcon_port,
                max_players:         r.max_players,
                memory_mb:           r.memory_mb,
                map_mod:             r.map_mod,
                online_players:      0,
                online_mode:         r.online_mode != 0,
                auto_start:          r.auto_start != 0,
                auto_start_delay:    r.auto_start_delay,
                crash_detection:     r.crash_detection != 0,
                shutdown_timeout:    r.shutdown_timeout,
                show_on_status_page: r.show_on_status_page != 0,
                data_dir:            s(r.data_dir),
                created_at:          s(r.created_at),
                updated_at:          s(r.updated_at),
            })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response(),
    }
}

async fn update_server(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(req): Json<CreateServerRequest>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_SETTINGS) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.settings"))).into_response();
    }
    if !check_membership(&state.db, &id, &user).await {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Access denied"))).into_response();
    }
    let online_mode_int:         i64 = if req.online_mode         { 1 } else { 0 };
    let auto_start_int:          i64 = if req.auto_start          { 1 } else { 0 };
    let crash_detection_int:     i64 = if req.crash_detection     { 1 } else { 0 };
    let show_on_status_page_int: i64 = if req.show_on_status_page { 1 } else { 0 };
    let result = sqlx::query!(
        "UPDATE servers SET name=?, description=?, max_players=?, memory_mb=?, map_mod=?,
         online_mode=?, auto_start=?, auto_start_delay=?, crash_detection=?,
         shutdown_timeout=?, show_on_status_page=?, updated_at=datetime('now') WHERE id=?",
        req.name, req.description, req.max_players, req.memory_mb, req.map_mod,
        online_mode_int, auto_start_int, req.auto_start_delay, crash_detection_int,
        req.shutdown_timeout, show_on_status_page_int, id,
    )
    .execute(&state.db)
    .await;

    match result {
        Ok(_)  => get_server(State(state), Extension(user), Path(id)).await.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response(),
    }
}

async fn delete_server(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_DELETE) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.delete"))).into_response();
    }
    if !check_membership(&state.db, &id, &user).await {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Access denied"))).into_response();
    }
    let row = sqlx::query!("SELECT container_id FROM servers WHERE id = ?", id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

    if let Some(row) = row {
        if let Some(container_id) = row.container_id {
            let _ = state
                .docker
                .stop_container(&container_id, Some(StopContainerOptions { t: 10 }))
                .await;
            let _ = state
                .docker
                .remove_container(&container_id, Some(RemoveContainerOptions { force: true, ..Default::default() }))
                .await;
        }
    }

    let _ = sqlx::query!("DELETE FROM servers WHERE id = ?", id)
        .execute(&state.db)
        .await;

    state.invalidate_rcon(&id).await;

    crate::velocity::unregister_server(&state, &id).await;

    let data_path = format!("/servers/{}", id);
    if let Err(e) = tokio::fs::remove_dir_all(&data_path).await {
        tracing::warn!(server_id=%id, path=%data_path, "Could not delete server data dir: {e}");
    } else {
        tracing::info!(server_id=%id, path=%data_path, "Deleted server data dir");
    }

    StatusCode::NO_CONTENT.into_response()
}

async fn ensure_fabricproxy_config_in_container(
    docker: &bollard::Docker,
    container_id: &str,
    velocity_secret: &str,
) -> Result<(), String> {
    use bollard::exec::{CreateExecOptions, StartExecOptions};

    let config_content = format!(
        "hackOnlineMode = true\nhackEarlySend = false\nhackMessageChain = false\ndisconnectMessage = \"This server requires you to connect with Velocity.\"\nsecret = \"{}\"",
        velocity_secret
    );

    let cmd = format!(
        "printf '%s' '{}' > /data/config/FabricProxy-Lite.toml",
        config_content.replace("'", "'\\''")
    );

    let exec = docker
        .create_exec(
            container_id,
            CreateExecOptions::<String> {
                cmd: Some(vec!["sh".to_string(), "-c".to_string(), cmd]),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| format!("Failed to create exec: {e}"))?;

    docker
        .start_exec(
            &exec.id,
            Some(StartExecOptions {
                detach: false,
                tty: false,
                ..Default::default()
            }),
        )
        .await
        .map_err(|e| format!("Failed to execute: {e}"))?;

    Ok(())

}

async fn ensure_fabricproxy_config(
    server_id: &str,
    loader: &str,
    velocity_enabled: bool,
    velocity_secret: &str,
) -> Result<(), String> {
    if !velocity_enabled || !loader.eq_ignore_ascii_case("FABRIC") {
        return Ok(());
    }

    let server_data_path = format!("/servers/{}", server_id);
    let cfg_dir = format!("{}/config", server_data_path);
    
    tokio::fs::create_dir_all(&cfg_dir)
        .await
        .map_err(|e| format!("Could not create config dir: {e}"))?;

    let proxy_cfg_path = format!("{}/FabricProxy-Lite.toml", cfg_dir);
    let proxy_cfg = format!(
        "hackOnlineMode = true\nhackEarlySend = false\nhackMessageChain = false\ndisconnectMessage = \"This server requires you to connect with Velocity.\"\nsecret = \"{}\"\n",
        velocity_secret
    );

    tokio::fs::write(&proxy_cfg_path, proxy_cfg)
        .await
        .map_err(|e| format!("Could not write FabricProxy config: {e}"))?;

    tracing::info!(server_id=%server_id, path=%proxy_cfg_path, "Wrote FabricProxy-Lite config");
    Ok(())
}

async fn start_server(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_POWER) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.power"))).into_response();
    }
    if !check_membership(&state.db, &id, &user).await {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Access denied"))).into_response();
    }
    let row = sqlx::query!(
        "SELECT id, name, loader, mc_version, port, rcon_port, rcon_password, max_players, memory_mb,
         map_mod, online_mode, crash_detection, container_id
         FROM servers WHERE id = ?",
        id
    )
    .fetch_optional(&state.db)
    .await;

    let server = match row {
        Ok(Some(r)) => r,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(ErrorResponse::new("Server not found"))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response(),
    };

    let cfg = state.config.read().await.clone();

    if let Err(e) = ensure_server_properties(&id).await {
        tracing::warn!(server_id=%id, error=%e, "World settings ensure failed (non-fatal)");
    }

    if let Err(e) = ensure_fabricproxy_config(
        &id,
        &server.loader,
        cfg.velocity_enabled,
        &cfg.velocity_secret,
    )
    .await
    {
        tracing::warn!(server_id=%id, error=%e, "FabricProxy config ensure failed (non-fatal)");
    }

    if let Some(ref cid) = server.container_id {
        tracing::info!(server_id=%id, container=%cid, "Restarting existing container");
        if let Err(e) = state.docker.start_container(cid, None::<StartContainerOptions<String>>).await {
            tracing::error!(server_id=%id, container=%cid, "Failed to restart container: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(format!("Container restart failed: {e}")))).into_response();
        }
        if cfg.velocity_enabled && server.loader.eq_ignore_ascii_case("FABRIC") {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            if let Err(e) = ensure_fabricproxy_config_in_container(&state.docker, cid, &cfg.velocity_secret).await {
                tracing::warn!(server_id=%id, container=%cid, error=%e, "Failed to write FabricProxy config to container (non-fatal)");
            }
        }
    } else {
        let mem_bytes        = server.memory_mb * 1024 * 1024;
        let name             = container_name(&id);
        let network          = std::env::var("DOCKER_NETWORK").unwrap_or_else(|_| "novabox-mc-net".to_string());
        let loader           = server.loader;
        let mc_version       = server.mc_version;
        let rcon_password    = server.rcon_password;
        let short_id         = &id[..8];
        let server_data_path = format!("{}/{}", state.servers_host_path, id);

        let mut port_bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();

        if !cfg.velocity_enabled {
            port_bindings.insert(
                format!("{}/tcp", server.port),
                Some(vec![PortBinding {
                    host_ip:   Some("0.0.0.0".to_string()),
                    host_port: Some(server.port.to_string()),
                }]),
            );
        }
        if let Some(ref mm) = server.map_mod {
            if !cfg.traefik_enabled {
                let map_port: u32 = match mm.to_uppercase().as_str() {
                    "DYNMAP" => 8123,
                    _        => 8100,
                };
                port_bindings.insert(
                    format!("{}/tcp", map_port),
                    Some(vec![PortBinding {
                        host_ip:   Some("0.0.0.0".to_string()),
                        host_port: Some(map_port.to_string()),
                    }]),
                );
            }
        }

        let effective_online_mode = if cfg.velocity_enabled {
            "FALSE"
        } else if server.online_mode != 0 {
            "TRUE"
        } else {
            "FALSE"
        };

        let mut env = vec![
            "EULA=TRUE".to_string(),
            format!("TYPE={}", loader),
            format!("VERSION={}", mc_version),
            format!("MAX_PLAYERS={}", server.max_players),
            format!("MEMORY={}M", server.memory_mb),
            "ENABLE_RCON=true".to_string(),
            format!("RCON_PASSWORD={}", rcon_password),
            format!("RCON_PORT={}", server.rcon_port),
            format!("ONLINE_MODE={}", effective_online_mode),
            "ALLOW_NETHER=true".to_string(),
            "GENERATE_STRUCTURES=true".to_string(),
            "VIEW_DISTANCE=10".to_string(),
        ];

        let world_settings = read_world_settings(&id).await;
        if let Some(sim) = world_settings.simulation_distance {
            env.push(format!("SIMULATION_DISTANCE={}", sim));
        }
        if let Some(view) = world_settings.view_distance {
            env.push(format!("VIEW_DISTANCE={}", view));
        }

        let runtime_opts = read_runtime_options(&id).await;
        if let Some(min_mb) = runtime_opts.min_memory_mb {
            let min_mb = min_mb.min(server.memory_mb).max(128);
            env.push(format!("INIT_MEMORY={}M", min_mb));
        }
        if let Some(flags) = runtime_opts.jvm_flags {
            let trimmed = flags.trim();
            if !trimmed.is_empty() {
                env.push(format!("JVM_OPTS={}", trimmed));
            }
        }
        if let Some(pause) = runtime_opts.pause_when_empty_seconds {
            if pause > 0 {
                env.push(format!("PAUSE_WHEN_EMPTY_SECONDS={}", pause));
            }
        }

        if cfg.velocity_enabled {
            env.push("ENABLE_VELOCITY=TRUE".to_string());
            env.push(format!("VELOCITY_SECRET={}", cfg.velocity_secret));
        }

        let mut modrinth: Vec<String> = vec![];
        match loader.to_uppercase().as_str() {
            "FABRIC" => modrinth.push("fabric-api".to_string()),
            "QUILT"  => modrinth.push("qsl".to_string()),
            _ => {}
        }
        if cfg.velocity_enabled {
            match loader.to_uppercase().as_str() {
                "FABRIC" => modrinth.push("fabricproxy-lite".to_string()),
                _ => {}
            }
        }
        if let Some(ref mm) = server.map_mod {
            modrinth.push(match mm.to_uppercase().as_str() {
                "DYNMAP" => "dynmap".to_string(),
                _        => "bluemap".to_string(),
            });
        }

        let custom_projects = read_modrinth_projects(&id).await;
        modrinth.extend(custom_projects.projects);
        let modrinth = normalize_modrinth_projects(&modrinth);

        if !modrinth.is_empty() {
            env.push(format!("MODRINTH_PROJECTS={}", modrinth.join(",")));
        }

        let mut labels: HashMap<String, String> = HashMap::new();
        let stack_name = std::env::var("COMPOSE_STACK_NAME")
            .unwrap_or_else(|_| "novabox-local".to_string());
        labels.insert("com.docker.compose.project".to_string(), stack_name);
        labels.insert("com.docker.compose.service".to_string(), "minecraft".to_string());
        labels.insert("com.docker.compose.oneoff".to_string(), "False".to_string());

        if cfg.traefik_enabled {
            if let Some(ref mm) = server.map_mod {
                let map_port = match mm.to_uppercase().as_str() {
                    "DYNMAP" => 8123u16,
                    _        => 8100,
                };
                let router  = format!("map-{}", short_id);
                let service = format!("map-{}-svc", short_id);
                labels.insert("traefik.enable".into(), "true".into());
                labels.insert(
                    format!("traefik.http.routers.{}.rule", router),
                    format!("Host(`map.{}.{}`)", short_id, cfg.domain),
                );
                labels.insert(
                    format!("traefik.http.routers.{}.entrypoints", router),
                    "web".into(),
                );
                labels.insert(
                    format!("traefik.http.routers.{}.service", router),
                    service.clone(),
                );
                labels.insert(
                    format!("traefik.http.services.{}.loadbalancer.server.port", service),
                    map_port.to_string(),
                );
                labels.insert(
                    "traefik.docker.network".to_string(),
                    network.clone(),
                );
            }
        }

        let config = Config {
            image:  Some("itzg/minecraft-server:latest".to_string()),
            env:    Some(env),
            labels: if labels.is_empty() { None } else { Some(labels) },
            host_config: Some(HostConfig {
                memory:          Some(mem_bytes),
                port_bindings:   Some(port_bindings),
                network_mode:    Some(network.clone()),
                binds:           Some(vec![format!("{}:/data", server_data_path)]),
                restart_policy:  Some(bollard::models::RestartPolicy {
                    name: Some(if server.crash_detection != 0 {
                        bollard::models::RestartPolicyNameEnum::ON_FAILURE
                    } else {
                        bollard::models::RestartPolicyNameEnum::NO
                    }),
                    maximum_retry_count: Some(3),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        ensure_network(&state.docker, &network).await;

        tracing::info!(server_id=%id, "Pulling itzg/minecraft-server:latest");
        let mut pull = state.docker.create_image(
            Some(CreateImageOptions {
                from_image: "itzg/minecraft-server",
                tag: "latest",
                ..Default::default()
            }),
            None,
            None,
        );
        while let Some(result) = pull.next().await {
            if let Err(e) = result {
                tracing::warn!(server_id=%id, "Image pull event error (non-fatal): {e}");
            }
        }

        tracing::info!(server_id=%id, container_name=%name, network=%network, bind=%server_data_path, "Creating Minecraft container");
        match state.docker.create_container(Some(CreateContainerOptions { name: &name, platform: None }), config).await {
            Ok(resp) => {
                let cid = resp.id;
                if let Err(e) = state.docker.start_container(&cid, None::<StartContainerOptions<String>>).await {
                    tracing::error!(server_id=%id, container=%cid, "Failed to start container: {e}");
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(format!("Container start failed: {e}")))).into_response();
                }
                let _ = sqlx::query!(
                    "UPDATE servers SET container_id=?, status='starting', updated_at=datetime('now') WHERE id=?",
                    cid, id
                )
                .execute(&state.db)
                .await;
                if cfg.velocity_enabled && loader.eq_ignore_ascii_case("FABRIC") {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    if let Err(e) = ensure_fabricproxy_config_in_container(&state.docker, &cid, &cfg.velocity_secret).await {
                        tracing::warn!(server_id=%id, container=%cid, error=%e, "Failed to write FabricProxy config to container (non-fatal)");
                    }
                }
            }
            Err(e) => {
                tracing::error!(server_id=%id, "Failed to create container: {e}");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(format!("Container create failed: {e}")))).into_response();
            }
        }
    }

    let _ = sqlx::query!(
        "UPDATE servers SET status='starting', updated_at=datetime('now') WHERE id=?",
        id
    )
    .execute(&state.db)
    .await;

    crate::velocity::regenerate(&state).await;

    Json(serde_json::json!({"status": "starting"})).into_response()
}

async fn stop_server(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_POWER) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.power"))).into_response();
    }
    if !check_membership(&state.db, &id, &user).await {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Access denied"))).into_response();
    }
    let row = sqlx::query!("SELECT container_id, shutdown_timeout FROM servers WHERE id = ?", id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

    if let Some(row) = row {
        if let Some(container_id) = row.container_id {
            let timeout = row.shutdown_timeout as i64;
            let _ = state.docker.stop_container(&container_id, Some(StopContainerOptions { t: timeout })).await;
        }
    }

    let _ = sqlx::query!(
        "UPDATE servers SET status='stopped', updated_at=datetime('now') WHERE id=?",
        id
    )
    .execute(&state.db)
    .await;

    state.invalidate_rcon(&id).await;

    crate::velocity::regenerate(&state).await;

    Json(serde_json::json!({"status": "stopped"})).into_response()
}

async fn restart_server(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_POWER) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.power"))).into_response();
    }
    if !check_membership(&state.db, &id, &user).await {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Access denied"))).into_response();
    }
    let row = sqlx::query!("SELECT container_id, loader FROM servers WHERE id = ?", id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

    if let Some(row) = row {
        if let Some(container_id) = row.container_id {
            let _ = state.docker.stop_container(&container_id, Some(StopContainerOptions { t: 10 })).await;
            let _ = state
                .docker
                .remove_container(&container_id, Some(RemoveContainerOptions { force: true, ..Default::default() }))
                .await;
            let _ = sqlx::query!(
                "UPDATE servers SET container_id=NULL, status='stopped', updated_at=datetime('now') WHERE id=?",
                id
            )
            .execute(&state.db)
            .await;
        }
    }

    state.invalidate_rcon(&id).await;

    let response = start_server(State(state), Extension(user), Path(id)).await;
    response.into_response()
}

async fn kill_server(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_POWER) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.power"))).into_response();
    }
    if !check_membership(&state.db, &id, &user).await {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Access denied"))).into_response();
    }
    let row = sqlx::query!("SELECT container_id FROM servers WHERE id = ?", id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

    if let Some(row) = row {
        if let Some(container_id) = row.container_id {
            let _ = state.docker.kill_container::<String>(&container_id, None).await;
        }
    }

    let _ = sqlx::query!(
        "UPDATE servers SET status='stopped', updated_at=datetime('now') WHERE id=?",
        id
    )
    .execute(&state.db)
    .await;

    state.invalidate_rcon(&id).await;

    crate::velocity::regenerate(&state).await;

    Json(serde_json::json!({"status": "killed"})).into_response()
}

async fn storage_usage(
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_VIEW) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.view"))).into_response();
    }
    let path = PathBuf::from(format!("/servers/{}", id));
    let bytes = tokio::task::spawn_blocking(move || dir_size_bytes_sync(&path))
        .await
        .unwrap_or(0);

    let mb = bytes as f64 / 1024.0 / 1024.0;
    let gb = mb / 1024.0;

    Json(StorageUsage {
        bytes: bytes as i64,
        mb,
        gb,
    })
    .into_response()
}

async fn get_runtime_options(
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_SETTINGS) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.settings"))).into_response();
    }
    Json(read_runtime_options(&id).await).into_response()
}

async fn set_runtime_options(
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(opts): Json<RuntimeOptions>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_SETTINGS) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.settings"))).into_response();
    }
    if let Err(e) = write_runtime_options(&id, opts.clone()).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response();
    }

    Json(read_runtime_options(&id).await).into_response()
}

async fn run_command(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(req): Json<RconCommandRequest>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_CONSOLE) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.console"))).into_response();
    }
    let cmd = req.command.trim().to_string();
    state
        .append_log_line(&id, format!("> {}", cmd))
        .await;

    match state.rcon_command(&id, &cmd).await {
        Ok(output) => {
            let trimmed = output.trim();
            if trimmed.is_empty() {
                state
                    .append_log_line(&id, "(RCON: no output)".to_string())
                    .await;
            } else {
                for line in trimmed.lines() {
                    state.append_log_line(&id, line.to_string()).await;
                }
            }
            Json(serde_json::json!({"output": output})).into_response()
        }
        Err(e) => (StatusCode::BAD_GATEWAY, Json(ErrorResponse::new(e))).into_response(),
    }
}

async fn send_stdin_command(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(req): Json<RconCommandRequest>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_CONSOLE) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.console"))).into_response();
    }

    use bollard::exec::{CreateExecOptions, StartExecOptions};

    let command = req.command.trim();
    if command.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Command cannot be empty"))).into_response();
    }

    let row = sqlx::query!(
        "SELECT container_id FROM servers WHERE id = ?",
        id
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let container_id = match row.and_then(|r| r.container_id) {
        Some(cid) => cid,
        None => return (StatusCode::SERVICE_UNAVAILABLE, Json(ErrorResponse::new("Server is not running"))).into_response(),
    };

    let escaped = command.replace('\\', "\\\\").replace('\'', "'\\''");
    let sh = format!("printf '%s\\n' '{}' > /proc/1/fd/0", escaped);

    let exec = match state
        .docker
        .create_exec(
            &container_id,
            CreateExecOptions::<String> {
                attach_stdout: Some(false),
                attach_stderr: Some(false),
                cmd: Some(vec!["sh".to_string(), "-c".to_string(), sh]),
                ..Default::default()
            },
        )
        .await
    {
        Ok(exec) => exec,
        Err(e) => {
            return (StatusCode::BAD_GATEWAY, Json(ErrorResponse::new(format!("Failed to create stdin exec: {e}")))).into_response();
        }
    };

    if let Err(e) = state
        .docker
        .start_exec(
            &exec.id,
            Some(StartExecOptions {
                detach: false,
                tty: false,
                ..Default::default()
            }),
        )
        .await
    {
        return (StatusCode::BAD_GATEWAY, Json(ErrorResponse::new(format!("Failed to write command to stdin: {e}")))).into_response();
    }

    state.append_log_line(&id, format!("> {}", command)).await;
    Json(serde_json::json!({"status": "queued"})).into_response()
}

async fn apply_map_switch(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_SETTINGS) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.settings"))).into_response();
    }
    if !check_membership(&state.db, &id, &user).await {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Access denied"))).into_response();
    }

    let row = sqlx::query!(
        "SELECT container_id, shutdown_timeout FROM servers WHERE id = ?",
        id
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    if let Some(row) = row {
        if let Some(container_id) = row.container_id {
            let timeout = row.shutdown_timeout as i64;
            let _ = state.docker.stop_container(&container_id, Some(StopContainerOptions { t: timeout })).await;
            let _ = state.docker.remove_container(&container_id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await;
        }
    }

    let _ = sqlx::query!(
        "UPDATE servers SET container_id=NULL, status='stopped', updated_at=datetime('now') WHERE id=?",
        id
    )
    .execute(&state.db)
    .await;

    state.invalidate_rcon(&id).await;

    let plugins_dir = format!("/servers/{}/plugins", id);
    for map_dir in &["BlueMap", "bluemap", "dynmap", "Dynmap"] {
        let p = format!("{}/{}", plugins_dir, map_dir);
        let _ = tokio::fs::remove_dir_all(&p).await;
    }
    if let Ok(mut entries) = tokio::fs::read_dir(&plugins_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if (name.starts_with("bluemap") || name.starts_with("dynmap")) && name.ends_with(".jar") {
                let _ = tokio::fs::remove_file(entry.path()).await;
            }
        }
    }

    crate::velocity::regenerate(&state).await;

    tracing::info!(server_id=%id, "Map switch applied: container removed, plugin data wiped");
    Json(serde_json::json!({"ok": true})).into_response()
}

async fn get_map_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let server = match sqlx::query!(
        "SELECT map_mod, status FROM servers WHERE id = ?",
        id
    )
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return (StatusCode::NOT_FOUND, "Server not found".to_string()).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    if server.status != "running" {
        return (StatusCode::SERVICE_UNAVAILABLE, "Server is not running".to_string()).into_response();
    }

    let map_mod = match server.map_mod {
        Some(m) => m,
        None => return (StatusCode::NOT_FOUND, "Server has no map".to_string()).into_response(),
    };

    let is_dynmap = map_mod.to_uppercase() == "DYNMAP";
    let map_port = if is_dynmap { 8123 } else { 8100 };
    
    let container = container_name(&id);
    let map_url = format!("http://{}:{}", container, map_port);

    let endpoint = if is_dynmap {
        format!("{}/up/configuration", map_url)
    } else {
        format!("{}/settings.json", map_url)
    };

    match reqwest::Client::new()
        .get(&endpoint)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(res) => {
            let status_code = res.status().as_u16();
            let status = if status_code >= 400 {
                StatusCode::from_u16(status_code).unwrap_or(StatusCode::BAD_GATEWAY)
            } else {
                StatusCode::OK
            };
            match res.text().await {
                Ok(body) => (status, body).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            }
        }
        Err(e) => {
            tracing::warn!(server_id=%id, error=%e, "Failed to proxy map config");
            (StatusCode::BAD_GATEWAY, format!("Failed to fetch map config: {}", e)).into_response()
        }
    }
}

async fn list_members(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if user.role != Role::Admin {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Admin only"))).into_response();
    }
    let rows = sqlx::query!(
        "SELECT user_id, added_at FROM server_members WHERE server_id = ? ORDER BY added_at ASC",
        id
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => {
            let all_users = state.auth.load_users().await;
            let members: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|r| {
                    let username = all_users
                        .iter()
                        .find(|u| u.id == r.user_id)
                        .map(|u| u.username.clone())
                        .unwrap_or_else(|| r.user_id.clone());
                    serde_json::json!({
                        "user_id":  r.user_id,
                        "username": username,
                        "added_at": r.added_at,
                    })
                })
                .collect();
            Json(members).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response(),
    }
}

async fn add_member(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Json(req): Json<AddMemberRequest>,
) -> impl IntoResponse {
    if user.role != Role::Admin {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Admin only"))).into_response();
    }

    let target_user = match state.auth.find_by_username(&req.username).await {
        Some(u) => u,
        None => return (StatusCode::UNPROCESSABLE_ENTITY, Json(ErrorResponse::new("User not found"))).into_response(),
    };

    let result = sqlx::query!(
        "INSERT OR IGNORE INTO server_members (server_id, user_id) VALUES (?, ?)",
        id,
        target_user.id
    )
    .execute(&state.db)
    .await;

    match result {
        Ok(_)  => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response(),
    }
}

async fn remove_member(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path((id, uid)): Path<(String, String)>,
) -> impl IntoResponse {
    if user.role != Role::Admin {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Admin only"))).into_response();
    }
    let _ = sqlx::query!(
        "DELETE FROM server_members WHERE server_id = ? AND user_id = ?",
        id,
        uid
    )
    .execute(&state.db)
    .await;

    StatusCode::NO_CONTENT.into_response()
}

async fn allocate_rcon_port(state: &Arc<AppState>) -> i64 {
    for _ in 0..32 {
        let bytes = *Uuid::new_v4().as_bytes();
        let rnd = u16::from_be_bytes([bytes[0], bytes[1]]) as i64;
        let candidate = 20000 + (rnd % 40000);

        match sqlx::query!("SELECT id FROM servers WHERE rcon_port = ? LIMIT 1", candidate)
            .fetch_optional(&state.db)
            .await
        {
            Ok(None) => return candidate,
            Ok(Some(_)) => continue,
            Err(e) => {
                tracing::warn!("Failed to check RCON port uniqueness: {e}");
                return candidate;
            }
        }
    }

    25575
}

async fn ensure_network(docker: &bollard::Docker, name: &str) {
    match docker.inspect_network(name, None::<bollard::network::InspectNetworkOptions<String>>).await {
        Ok(_) => {
            tracing::debug!(network=%name, "Network already exists");
        }
        Err(_) => {
            tracing::info!(network=%name, "Creating Docker network");
            let _ = docker
                .create_network(CreateNetworkOptions {
                    name,
                    driver: "bridge",
                    ..Default::default()
                })
                .await;
        }
    }
}
