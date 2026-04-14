use crate::AppState;
use crate::api::models::*;
use crate::docker::container_name;
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json,
};
use bollard::container::{
    Config, CreateContainerOptions, InspectContainerOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::{HostConfig, PortBinding};
use bollard::network::CreateNetworkOptions;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_servers).post(create_server))
        .route("/:id", get(get_server).put(update_server).delete(delete_server))
        .route("/:id/start", post(start_server))
        .route("/:id/stop", post(stop_server))
        .route("/:id/kill", post(kill_server))
        .route("/:id/restart", post(restart_server))
        .route("/:id/storage", get(storage_usage))
        .route("/:id/runtime", get(get_runtime_options).put(set_runtime_options))
        .route("/:id/command", post(run_command))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RuntimeOptions {
    #[serde(default)]
    min_memory_mb: Option<i64>,
    #[serde(default)]
    jvm_flags: Option<String>,
}

#[derive(Debug, Serialize)]
struct StorageUsage {
    bytes: i64,
    mb: f64,
    gb: f64,
}

fn runtime_options_path(server_id: &str) -> String {
    format!("/servers/{}/novabox.runtime.json", server_id)
}

async fn read_runtime_options(server_id: &str) -> RuntimeOptions {
    let path = runtime_options_path(server_id);
    let text = match tokio::fs::read_to_string(path).await {
        Ok(t) => t,
        Err(_) => {
            return RuntimeOptions {
                min_memory_mb: None,
                jvm_flags: None,
            };
        }
    };

    serde_json::from_str(&text).unwrap_or(RuntimeOptions {
        min_memory_mb: None,
        jvm_flags: None,
    })
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

async fn list_servers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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
            let servers: Vec<Server> = rows
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
            Json(servers).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response(),
    }
}

async fn get_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
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
    Json(req): Json<CreateServerRequest>,
) -> impl IntoResponse {
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
    Path(id): Path<String>,
    Json(req): Json<CreateServerRequest>,
) -> impl IntoResponse {
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
        Ok(_)  => get_server(State(state), Path(id)).await.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response(),
    }
}

async fn delete_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
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

    crate::velocity::unregister_server(&state, &id).await;

    let data_path = format!("/servers/{}", id);
    if let Err(e) = tokio::fs::remove_dir_all(&data_path).await {
        tracing::warn!(server_id=%id, path=%data_path, "Could not delete server data dir: {e}");
    } else {
        tracing::info!(server_id=%id, path=%data_path, "Deleted server data dir");
    }

    StatusCode::NO_CONTENT
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
    Path(id): Path<String>,
) -> impl IntoResponse {
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

        if cfg.velocity_enabled {
            env.push("ENABLE_VELOCITY=TRUE".to_string());
            env.push(format!("VELOCITY_SECRET={}", cfg.velocity_secret));
        }

        let mut modrinth: Vec<&str> = vec![];
        match loader.to_uppercase().as_str() {
            "FABRIC" => modrinth.push("fabric-api"),
            "QUILT"  => modrinth.push("qsl"),
            _ => {}
        }
        if cfg.velocity_enabled {
            match loader.to_uppercase().as_str() {
                "FABRIC" => modrinth.push("fabricproxy-lite"),
                _ => {}
            }
        }
        if let Some(ref mm) = server.map_mod {
            modrinth.push(match mm.to_uppercase().as_str() {
                "DYNMAP" => "dynmap",
                _        => "bluemap",
            });
        }
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
    Path(id): Path<String>,
) -> impl IntoResponse {
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

    crate::velocity::regenerate(&state).await;

    Json(serde_json::json!({"status": "stopped"})).into_response()
}

async fn restart_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let row = sqlx::query!("SELECT container_id, loader FROM servers WHERE id = ?", id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

    if let Some(row) = row {
        if let Some(container_id) = row.container_id {
            let _ = state.docker.restart_container(&container_id, None).await;

            let cfg = state.config.read().await.clone();
            if cfg.velocity_enabled && row.loader.eq_ignore_ascii_case("FABRIC") {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                if let Err(e) = ensure_fabricproxy_config_in_container(&state.docker, &container_id, &cfg.velocity_secret).await {
                    tracing::warn!(server_id=%id, container=%container_id, error=%e, "Failed to write FabricProxy config to container (non-fatal)");
                }
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

    Json(serde_json::json!({"status": "restarting"})).into_response()
}

async fn kill_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
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

    crate::velocity::regenerate(&state).await;

    Json(serde_json::json!({"status": "killed"})).into_response()
}

async fn storage_usage(
    Path(id): Path<String>,
) -> impl IntoResponse {
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
    Path(id): Path<String>,
) -> impl IntoResponse {
    Json(read_runtime_options(&id).await).into_response()
}

async fn set_runtime_options(
    Path(id): Path<String>,
    Json(opts): Json<RuntimeOptions>,
) -> impl IntoResponse {
    if let Err(e) = write_runtime_options(&id, opts.clone()).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response();
    }

    Json(read_runtime_options(&id).await).into_response()
}

async fn run_command(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<RconCommandRequest>,
) -> impl IntoResponse {
    let row = sqlx::query!(
        "SELECT rcon_port, rcon_password, container_id FROM servers WHERE id = ?",
        id
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let server = match row {
        Some(r) => r,
        None    => return (StatusCode::NOT_FOUND, Json(ErrorResponse::new("Server not found"))).into_response(),
    };

    let container_id = match server.container_id {
        Some(cid) => cid,
        None      => return (StatusCode::SERVICE_UNAVAILABLE, Json(ErrorResponse::new("Server is not running"))).into_response(),
    };

    let network   = std::env::var("DOCKER_NETWORK").unwrap_or_else(|_| "novabox-mc-net".to_string());
    let rcon_host = match container_ip(&state.docker, &container_id, &network).await {
        Some(ip) => ip,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse::new("Server network address unavailable")),
            )
                .into_response();
        }
    };

    let mut rcon = match timeout(
        Duration::from_secs(4),
        crate::rcon::RconClient::connect(&rcon_host, server.rcon_port as u16, &server.rcon_password),
    )
    .await
    {
        Ok(Ok(client)) => client,
        Ok(Err(e)) => {
            return (StatusCode::BAD_GATEWAY, Json(ErrorResponse::new(e))).into_response();
        }
        Err(_) => {
            return (
                StatusCode::GATEWAY_TIMEOUT,
                Json(ErrorResponse::new("RCON connect timed out")),
            )
                .into_response();
        }
    };

    let cmd = req.command.trim().to_string();
    state
        .append_log_line(&id, format!("> {}", cmd))
        .await;

    match timeout(Duration::from_secs(6), rcon.command(&cmd)).await {
        Ok(Ok(output)) => {
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
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response(),
        Err(_) => (
            StatusCode::GATEWAY_TIMEOUT,
            Json(ErrorResponse::new("RCON command timed out")),
        )
            .into_response(),
    }
}

async fn container_ip(docker: &bollard::Docker, container_id: &str, network_name: &str) -> Option<String> {
    let info = docker
        .inspect_container(container_id, None::<InspectContainerOptions>)
        .await
        .ok()?;

    let networks = info.network_settings?.networks?;
    networks
        .get(network_name)
        .and_then(|n| n.ip_address.clone())
        .filter(|ip| !ip.is_empty())
}

async fn allocate_rcon_port(state: &Arc<AppState>) -> i64 {
    for _ in 0..32 {
        let bytes = *Uuid::new_v4().as_bytes();
        let rnd = u16::from_be_bytes([bytes[0], bytes[1]]) as i64;
        let candidate = 20000 + (rnd % 40000); // 20000..59999

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
