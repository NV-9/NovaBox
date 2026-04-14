use crate::AppState;
use bollard::container::{InspectContainerOptions, LogOutput, LogsOptions, StartContainerOptions, StatsOptions};
use bollard::models::ContainerStateStatusEnum;
use futures_util::StreamExt;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

const DOCKER_NETWORK: fn() -> String = || {
    std::env::var("DOCKER_NETWORK").unwrap_or_else(|_| "novabox-mc-net".to_string())
};

pub async fn auto_start_servers(state: Arc<AppState>) {
    let rows = sqlx::query!(
        "SELECT id, container_id, auto_start_delay FROM servers
         WHERE auto_start = 1 AND status = 'stopped' AND container_id IS NOT NULL"
    )
    .fetch_all(&state.db)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(e) => { tracing::warn!("auto_start: DB query failed: {e}"); return; }
    };

    for row in rows {
        let server_id    = match row.id           { Some(v) => v, None => continue };
        let container_id = match row.container_id { Some(v) => v, None => continue };
        let delay_secs   = row.auto_start_delay as u64;

        let state2 = state.clone();
        tokio::spawn(async move {
            if delay_secs > 0 {
                tracing::info!(server_id=%server_id, delay=delay_secs, "Auto-start: waiting before starting");
                tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;
            }
            tracing::info!(server_id=%server_id, "Auto-starting server");
            match state2.docker.start_container(&container_id, None::<StartContainerOptions<String>>).await {
                Ok(_) => {
                    let _ = sqlx::query!(
                        "UPDATE servers SET status='starting', updated_at=datetime('now') WHERE id=?",
                        server_id
                    )
                    .execute(&state2.db)
                    .await;
                }
                Err(e) => tracing::warn!(server_id=%server_id, "Auto-start failed: {e}"),
            }
        });
    }
}

pub async fn run(state: Arc<AppState>) {
    tracing::info!("Container monitor started");

    let tailing: Arc<Mutex<HashSet<String>>>               = Arc::new(Mutex::new(HashSet::new()));
    let bluemap_reload_pending: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let bluemap_rendered_containers: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    loop {
        sync_statuses(state.clone()).await;

        bluemap_check(
            state.clone(),
            bluemap_reload_pending.clone(),
            bluemap_rendered_containers.clone(),
        ).await;

        let rows = sqlx::query!(
            "SELECT id, container_id, port FROM servers WHERE status = 'running' AND container_id IS NOT NULL"
        )
        .fetch_all(&state.db)
        .await;

        if let Ok(rows) = rows {
            for row in rows {
                let server_id    = match row.id           { Some(v) => v, None => continue };
                let container_id = match row.container_id { Some(v) => v, None => continue };
                let port         = row.port as u16;

                {
                    let mut set = tailing.lock().await;
                    if !set.contains(&server_id) {
                        set.insert(server_id.clone());
                        let state2   = state.clone();
                        let sid      = server_id.clone();
                        let cid      = container_id.clone();
                        let tailing2 = tailing.clone();
                        tokio::spawn(async move {
                            tail_logs(state2, &sid, &cid).await;
                            tailing2.lock().await.remove(&sid);
                        });
                    }
                }

                let state3 = state.clone();
                let sid2   = server_id.clone();
                let cid2   = container_id.clone();
                tokio::spawn(async move { poll_stats(state3, &sid2, &cid2, port).await });
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn sync_statuses(state: Arc<AppState>) {
    let rows = sqlx::query!(
        "SELECT id, container_id, status FROM servers WHERE container_id IS NOT NULL"
    )
    .fetch_all(&state.db)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(e) => { tracing::warn!("Status sync DB query failed: {e}"); return; }
    };

    for row in rows {
        let server_id    = match row.id            { Some(v) => v, None => continue };
        let container_id = match row.container_id  { Some(v) => v, None => continue };
        let db_status    = row.status;

        match state.docker.inspect_container(&container_id, None::<InspectContainerOptions>).await {
            Ok(info) => {
                let docker_status = info
                    .state
                    .and_then(|s| s.status)
                    .unwrap_or(ContainerStateStatusEnum::EMPTY);

                let new_status = match docker_status {
                    ContainerStateStatusEnum::RUNNING                => "running",
                    ContainerStateStatusEnum::EXITED
                    | ContainerStateStatusEnum::DEAD                 => "stopped",
                    ContainerStateStatusEnum::RESTARTING             => "starting",
                    ContainerStateStatusEnum::PAUSED                 => "stopped",
                    _                                                => continue,
                };

                if db_status != new_status {
                    tracing::info!(
                        server_id=%server_id,
                        from=%db_status,
                        to=%new_status,
                        "Updating server status"
                    );
                    let _ = sqlx::query!(
                        "UPDATE servers SET status=?, updated_at=datetime('now') WHERE id=?",
                        new_status, server_id
                    )
                    .execute(&state.db)
                    .await;
                }
            }
            Err(_) => {
                if db_status != "stopped" {
                    tracing::warn!(server_id=%server_id, "Container gone, marking stopped");
                    let _ = sqlx::query!(
                        "UPDATE servers SET status='stopped', container_id=NULL, updated_at=datetime('now') WHERE id=?",
                        server_id
                    )
                    .execute(&state.db)
                    .await;
                }
            }
        }
    }
}

async fn tail_logs(state: Arc<AppState>, server_id: &str, container_id: &str) {
    state.get_or_create_log_channel(server_id).await;

    let mut stream = state.docker.logs(
        container_id,
        Some(LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            tail: "1000".to_string(),
            ..Default::default()
        }),
    );

    while let Some(Ok(msg)) = stream.next().await {
        let line = match msg {
            LogOutput::StdOut { message } | LogOutput::StdErr { message } => {
                String::from_utf8_lossy(&message).trim().to_string()
            }
            _ => continue,
        };
        if line.is_empty() { continue; }
        state.append_log_line(server_id, line).await;
    }
}

async fn bluemap_check(
    state: Arc<AppState>,
    pending: Arc<Mutex<HashSet<String>>>,
    rendered_containers: Arc<Mutex<HashSet<String>>>,
) {
    let rows = sqlx::query!(
           "SELECT id, container_id, rcon_port, rcon_password \
            FROM servers WHERE status = 'running' AND UPPER(COALESCE(map_mod, '')) = 'BLUEMAP' AND container_id IS NOT NULL"
    )
    .fetch_all(&state.db)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(e) => { tracing::warn!("bluemap_check: DB query failed: {e}"); return; }
    };

    let network = DOCKER_NETWORK();
    let mut running_container_ids: HashSet<String> = HashSet::new();

    for row in rows {
        let server_id    = match row.id           { Some(v) => v, None => continue };
        let container_id = match row.container_id { Some(v) => v, None => continue };
        running_container_ids.insert(container_id.clone());
        let rcon_port = row.rcon_port;
        let rcon_pass = row.rcon_password;

        let conf_path = format!("/servers/{}/config/bluemap/core.conf", server_id);

        let mut should_queue_render = !rendered_containers.lock().await.contains(&container_id);
        if let Ok(content) = tokio::fs::read_to_string(&conf_path).await {
            let needs_patch = content.lines().any(|l| {
                let t = l.trim_start();
                t.starts_with("accept-download") && t.contains("false")
            });

            if needs_patch {
                tracing::info!(server_id=%server_id, path=%conf_path,
                    "bluemap_check: accept-download=false found, patching");

                let patched: String = content
                    .lines()
                    .map(|l| if l.trim_start().starts_with("accept-download") {
                        "accept-download: true".to_string()
                    } else {
                        l.to_string()
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let patched = if content.ends_with('\n') {
                    format!("{}\n", patched)
                } else {
                    patched
                };

                match tokio::fs::write(&conf_path, &patched).await {
                    Ok(_) => {
                        tracing::info!(server_id=%server_id, "bluemap_check: file patched, queuing reload");
                        should_queue_render = true;
                    }
                    Err(e) => tracing::warn!(server_id=%server_id, "bluemap_check: write failed: {e}"),
                }
            }
        }

        if should_queue_render {
            pending.lock().await.insert(server_id.clone());
        }

        if !pending.lock().await.contains(&server_id) { continue; }

        let rcon_ip = state.docker
            .inspect_container(&container_id, None::<InspectContainerOptions>)
            .await
            .ok()
            .and_then(|i| i.network_settings)
            .and_then(|n| n.networks)
            .and_then(|nets| nets.get(&network).and_then(|e| e.ip_address.clone()))
            .filter(|ip| !ip.is_empty());

        let ip = match rcon_ip {
            Some(ip) => ip,
            None => {
                tracing::debug!(server_id=%server_id, "bluemap_check: container IP not found, will retry next cycle");
                continue;
            }
        };

        match crate::rcon::RconClient::connect(&ip, rcon_port as u16, &rcon_pass).await {
            Ok(mut rcon) => {
                tracing::info!(server_id=%server_id, "bluemap_check: RCON ready, sending /bluemap reload then /bluemap render");
                let _ = rcon.command("bluemap reload").await;
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                let _ = rcon.command("bluemap render").await;
                pending.lock().await.remove(&server_id);
                rendered_containers.lock().await.insert(container_id.clone());
                tracing::info!(server_id=%server_id, "bluemap_check: reload + render triggered, done");
            }
            Err(e) => {
                tracing::debug!(server_id=%server_id, "bluemap_check: RCON not ready yet ({e}), will retry next cycle");
            }
        }
    }

    rendered_containers
        .lock()
        .await
        .retain(|cid| running_container_ids.contains(cid));
}

async fn poll_stats(state: Arc<AppState>, server_id: &str, container_id: &str, game_port: u16) {
    let mut stream = state.docker.stats(
        container_id,
        Some(StatsOptions { stream: false, one_shot: true }),
    );

    let (cpu_pct, mem_mb) = if let Some(Ok(stats)) = stream.next().await {
        let cpu_delta   = stats.cpu_stats.cpu_usage.total_usage as f64
            - stats.precpu_stats.cpu_usage.total_usage as f64;
        let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64
            - stats.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;
        let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as f64;
        let cpu = if system_delta > 0.0 {
            (cpu_delta / system_delta) * num_cpus * 100.0
        } else {
            0.0
        };
        let mem = stats.memory_stats.usage.unwrap_or(0) as f64 / 1024.0 / 1024.0;
        (cpu, mem)
    } else {
        return;
    };

    let network      = DOCKER_NETWORK();
    let container_ip = state.docker
        .inspect_container(container_id, None::<InspectContainerOptions>)
        .await
        .ok()
        .and_then(|i| i.network_settings)
        .and_then(|n| n.networks)
        .and_then(|nets| nets.get(&network).and_then(|e| e.ip_address.clone()))
        .filter(|ip| !ip.is_empty());

    let online_players = if let Some(ip) = container_ip {
        crate::mc_ping::ping(&ip, game_port).await.online_players
    } else {
        0
    };

    let metric_id = Uuid::new_v4().to_string();
    let _ = sqlx::query!(
        "INSERT INTO server_metrics (id, server_id, cpu_percent, memory_mb, online_players) VALUES (?, ?, ?, ?, ?)",
        metric_id, server_id, cpu_pct, mem_mb, online_players,
    )
    .execute(&state.db)
    .await;
}
