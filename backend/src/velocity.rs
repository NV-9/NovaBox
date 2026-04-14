use crate::AppState;
use std::sync::Arc;

const TOML_PATH:   &str = "/app/data/velocity.toml";
const SECRET_PATH: &str = "/app/data/forwarding.secret";

fn api_url() -> Option<String> {
    std::env::var("VELOCITY_API_URL").ok().filter(|s| !s.is_empty())
}

fn api_secret() -> String {
    std::env::var("VELOCITY_API_SECRET").unwrap_or_default()
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default()
}

pub async fn register_server(state: &Arc<AppState>, server_id: &str, container_host: &str) {
    let cfg = state.config.read().await.clone();
    if !cfg.velocity_enabled {
        return;
    }
    let Some(base_url) = api_url() else {
        tracing::debug!("velocity: VELOCITY_API_URL not set, skipping register");
        return;
    };

    let short_id = &server_id[..8];
    let body = serde_json::json!({
        "name": short_id,
        "host": container_host,
        "port": 25565,
    });

    let secret = api_secret();
    let mut req = http_client().post(format!("{}/servers", base_url)).json(&body);
    if !secret.is_empty() {
        req = req.header("x-novabox-secret", &secret);
    }

    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!(server_id=%server_id, short=%short_id, "velocity: registered server via API");
        }
        Ok(resp) => {
            tracing::warn!(server_id=%server_id, status=%resp.status(), "velocity: API register returned non-2xx");
        }
        Err(e) => {
            tracing::warn!(server_id=%server_id, "velocity: API register failed (proxy may not be ready yet): {e}");
        }
    }
}

pub async fn unregister_server(state: &Arc<AppState>, server_id: &str) {
    let cfg = state.config.read().await.clone();
    if !cfg.velocity_enabled {
        return;
    }
    let Some(base_url) = api_url() else {
        return;
    };

    let short_id = &server_id[..8];
    let secret = api_secret();
    let mut req = http_client().delete(format!("{}/servers/{}", base_url, short_id));
    if !secret.is_empty() {
        req = req.header("x-novabox-secret", &secret);
    }

    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!(server_id=%server_id, short=%short_id, "velocity: unregistered server via API");
        }
        Ok(resp) if resp.status() == 404 => {
            tracing::debug!(server_id=%server_id, "velocity: server was not registered, nothing to unregister");
        }
        Ok(resp) => {
            tracing::warn!(server_id=%server_id, status=%resp.status(), "velocity: API unregister returned non-2xx");
        }
        Err(e) => {
            tracing::warn!(server_id=%server_id, "velocity: API unregister failed: {e}");
        }
    }
}

pub async fn regenerate(state: &Arc<AppState>) {
    let cfg = state.config.read().await.clone();
    if !cfg.velocity_enabled {
        return;
    }

    let rows = sqlx::query!(
        "SELECT id, container_id FROM servers WHERE container_id IS NOT NULL"
    )
    .fetch_all(&state.db)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(e) => { tracing::warn!("velocity: DB query failed: {e}"); return; }
    };

    let mut servers_lines    = String::new();
    let mut try_list:          Vec<String> = Vec::new();
    let mut forced_host_lines  = String::new();

    for row in &rows {
        let server_id = match &row.id           { Some(v) => v.clone(), None => continue };
        let _cid      = match &row.container_id { Some(v) => v.clone(), None => continue };

        let short     = &server_id[..8];
        let container = crate::docker::container_name(&server_id);

        servers_lines.push_str(&format!("  \"{}\" = \"{}:25565\"\n", short, container));
        try_list.push(format!("\"{}\"", short));
        forced_host_lines.push_str(&format!("  \"{}.{}\" = [\"{}\"]\n", short, cfg.domain, short));
    }

    let try_array = format!("[{}]", try_list.join(", "));

    if let Err(e) = tokio::fs::write(SECRET_PATH, &cfg.velocity_secret).await {
        tracing::warn!("velocity: could not write forwarding.secret: {e}");
        return;
    }

    let toml = format!(
r#"config-version = "2.7"
bind = "0.0.0.0:25565"
motd = "&#8b5cf6NovaBox"
show-max-players = 500
online-mode = true
force-key-authentication = false
player-info-forwarding-mode = "modern"
announce-forge = false
forwarding-secret-file = "forwarding.secret"

[servers]
{servers_lines}  try = {try_array}

[forced-hosts]
{forced_host_lines}

[advanced]
  compression-threshold = 256
  compression-level = -1
  login-ratelimit = 3000
  connection-timeout = 5000
  read-timeout = 30000
  haproxy-protocol = false
  tcp-fast-open = false
  bungee-plugin-message-channel = true
  show-ping-requests = false
  failover-on-unexpected-server-disconnect = true
  announce-proxy-commands = true
  log-command-executions = false
  log-player-connections = true
  accepts-transfers = false

[query]
  enabled = false
  port = 25577
  map = "Velocity"
  show-plugins = false
"#,
        servers_lines     = servers_lines,
        try_array         = try_array,
        forced_host_lines = forced_host_lines,
    );

    match tokio::fs::write(TOML_PATH, &toml).await {
        Ok(_)  => tracing::info!("velocity: wrote {}", TOML_PATH),
        Err(e) => { tracing::warn!("velocity: failed to write {}: {e}", TOML_PATH); return; }
    }

    reload_velocity(&state.docker, &cfg.velocity_container).await;

    for row in &rows {
        let server_id = match &row.id { Some(v) => v.clone(), None => continue };
        let container = crate::docker::container_name(&server_id);
        register_server(state, &server_id, &container).await;
    }
}

async fn reload_velocity(docker: &bollard::Docker, container: &str) {
    use bollard::exec::{CreateExecOptions, StartExecOptions};

    let exec = docker
        .create_exec(
            container,
            CreateExecOptions::<String> {
                cmd: Some(vec![
                    "send-command".to_string(),
                    "velocity reload".to_string(),
                ]),
                ..Default::default()
            },
        )
        .await;

    match exec {
        Ok(resp) => {
            let _ = docker
                .start_exec(
                    &resp.id,
                    Some(StartExecOptions {
                        detach: true,
                        tty: false,
                        ..Default::default()
                    }),
                )
                .await;
            tracing::info!("velocity: sent 'velocity reload' to '{}'", container);
        }
        Err(e) => {
            tracing::debug!(
                "velocity: could not exec into '{}' (may not be running yet): {e}",
                container
            );
        }
    }
}
