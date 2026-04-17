use crate::auth::AuthStore;
use crate::config::AppConfig;
use crate::rcon::RconClient;
use bollard::container::InspectContainerOptions;
use bollard::Docker;
use sqlx::SqlitePool;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, RwLock};
use tokio::time::{Duration, Instant, timeout};

pub type LogSender = broadcast::Sender<String>;

const LOG_BUFFER_MAX: usize = 1000;

struct RconPoolEntry {
    client: Option<RconClient>,
    container_id: String,
    host: String,
    port: u16,
    password: String,
    last_used: Instant,
}

impl Default for RconPoolEntry {
    fn default() -> Self {
        Self {
            client: None,
            container_id: String::new(),
            host: String::new(),
            port: 0,
            password: String::new(),
            last_used: Instant::now(),
        }
    }
}

struct RconTarget {
    container_id: String,
    host: String,
    port: u16,
    password: String,
}

pub struct AppState {
    pub db: SqlitePool,
    pub docker: Docker,
    pub servers_host_path: String,
    pub config: RwLock<AppConfig>,
    pub auth: AuthStore,
    pub log_channels: RwLock<HashMap<String, LogSender>>,
    pub log_buffers: RwLock<HashMap<String, VecDeque<String>>>,
    rcon_pool: RwLock<HashMap<String, Arc<Mutex<RconPoolEntry>>>>,
}

impl AppState {
    pub fn new(db: SqlitePool, docker: Docker, servers_host_path: String, config: AppConfig, data_dir: &str) -> Self {
        Self {
            db,
            docker,
            servers_host_path,
            config: RwLock::new(config),
            auth: AuthStore::new(data_dir),
            log_channels: RwLock::new(HashMap::new()),
            log_buffers: RwLock::new(HashMap::new()),
            rcon_pool: RwLock::new(HashMap::new()),
        }
    }

    async fn fetch_rcon_target(&self, server_id: &str) -> Result<RconTarget, String> {
        let row = sqlx::query!(
            "SELECT container_id, status, rcon_port, rcon_password FROM servers WHERE id = ?",
            server_id
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| format!("DB query failed: {e}"))?
        .ok_or_else(|| "Server not found".to_string())?;

        if row.status != "running" {
            return Err("Server is not running".to_string());
        }

        let container_id = row
            .container_id
            .ok_or_else(|| "Server container unavailable".to_string())?;

        let network = std::env::var("DOCKER_NETWORK").unwrap_or_else(|_| "novabox-mc-net".to_string());

        let host = self
            .docker
            .inspect_container(&container_id, None::<InspectContainerOptions>)
            .await
            .map_err(|e| format!("Container inspect failed: {e}"))?
            .network_settings
            .and_then(|n| n.networks)
            .and_then(|nets| nets.get(&network).and_then(|e| e.ip_address.clone()))
            .filter(|ip| !ip.is_empty())
            .ok_or_else(|| "Container IP unavailable".to_string())?;

        Ok(RconTarget {
            container_id,
            host,
            port: row.rcon_port as u16,
            password: row.rcon_password,
        })
    }

    async fn get_or_create_rcon_entry(&self, server_id: &str) -> Arc<Mutex<RconPoolEntry>> {
        {
            let pool = self.rcon_pool.read().await;
            if let Some(entry) = pool.get(server_id) {
                return entry.clone();
            }
        }

        let mut pool = self.rcon_pool.write().await;
        pool.entry(server_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(RconPoolEntry::default())))
            .clone()
    }

    pub async fn invalidate_rcon(&self, server_id: &str) {
        let mut pool = self.rcon_pool.write().await;
        pool.remove(server_id);
    }

    pub async fn prune_idle_rcon(&self, idle_timeout: Duration) {
        let snapshot: Vec<(String, Arc<Mutex<RconPoolEntry>>)> = {
            let pool = self.rcon_pool.read().await;
            pool.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        let mut remove_ids: Vec<String> = Vec::new();
        for (server_id, entry) in snapshot {
            if let Ok(guard) = entry.try_lock() {
                if guard.last_used.elapsed() > idle_timeout {
                    remove_ids.push(server_id);
                }
            }
        }

        if remove_ids.is_empty() {
            return;
        }

        let mut pool = self.rcon_pool.write().await;
        for server_id in remove_ids {
            pool.remove(&server_id);
        }
    }

    pub async fn rcon_command(&self, server_id: &str, command: &str) -> Result<String, String> {
        let target = self.fetch_rcon_target(server_id).await?;
        let entry = self.get_or_create_rcon_entry(server_id).await;
        let mut guard = entry.lock().await;

        let changed = guard.container_id != target.container_id
            || guard.host != target.host
            || guard.port != target.port
            || guard.password != target.password;

        if changed {
            guard.client = None;
            guard.container_id = target.container_id.clone();
            guard.host = target.host.clone();
            guard.port = target.port;
            guard.password = target.password.clone();
        }

        if guard.client.is_none() {
            let client = timeout(
                Duration::from_secs(4),
                RconClient::connect(&target.host, target.port, &target.password),
            )
            .await
            .map_err(|_| "RCON connect timed out".to_string())?
            .map_err(|e| format!("RCON connect failed: {e}"))?;
            guard.client = Some(client);
        }

        guard.last_used = Instant::now();

        let first_try = match guard.client.as_mut() {
            Some(client) => timeout(Duration::from_secs(6), client.command(command)).await,
            None => return Err("RCON client unavailable".to_string()),
        };

        match first_try {
            Ok(Ok(out)) => {
                guard.last_used = Instant::now();
                Ok(out)
            }
            Ok(Err(e)) => {
                guard.client = None;
                tracing::debug!(server_id=%server_id, error=%e, "RCON pooled command failed, reconnecting once");

                let client = timeout(
                    Duration::from_secs(4),
                    RconClient::connect(&target.host, target.port, &target.password),
                )
                .await
                .map_err(|_| "RCON reconnect timed out".to_string())?
                .map_err(|re| format!("RCON reconnect failed: {re}"))?;
                guard.client = Some(client);

                let retry = match guard.client.as_mut() {
                    Some(client) => timeout(Duration::from_secs(6), client.command(command)).await,
                    None => return Err("RCON client unavailable after reconnect".to_string()),
                };

                match retry {
                    Ok(Ok(out)) => {
                        guard.last_used = Instant::now();
                        Ok(out)
                    }
                    Ok(Err(retry_err)) => {
                        guard.client = None;
                        Err(format!("RCON command failed after reconnect: {retry_err}"))
                    }
                    Err(_) => {
                        guard.client = None;
                        Err("RCON command timed out after reconnect".to_string())
                    }
                }
            }
            Err(_) => {
                guard.client = None;
                Err("RCON command timed out".to_string())
            }
        }
    }

    pub async fn get_or_create_log_channel(&self, server_id: &str) -> LogSender {
        {
            let channels = self.log_channels.read().await;
            if let Some(tx) = channels.get(server_id) {
                return tx.clone();
            }
        }
        let (tx, _) = broadcast::channel(1024);
        let mut channels = self.log_channels.write().await;
        channels.insert(server_id.to_string(), tx.clone());
        tx
    }

    pub async fn append_log_line(&self, server_id: &str, line: String) {
        {
            let mut buffers = self.log_buffers.write().await;
            let buf = buffers.entry(server_id.to_string()).or_insert_with(VecDeque::new);
            buf.push_back(line.clone());
            if buf.len() > LOG_BUFFER_MAX {
                buf.pop_front();
            }
        }
        let channels = self.log_channels.read().await;
        if let Some(tx) = channels.get(server_id) {
            let _ = tx.send(line);
        }
    }

    pub async fn recent_log_lines(&self, server_id: &str, n: usize) -> Vec<String> {
        let buffers = self.log_buffers.read().await;
        buffers
            .get(server_id)
            .map(|buf| {
                let skip = buf.len().saturating_sub(n);
                buf.iter().skip(skip).cloned().collect()
            })
            .unwrap_or_default()
    }
}
