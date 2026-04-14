use crate::config::AppConfig;
use bollard::Docker;
use sqlx::SqlitePool;
use std::collections::{HashMap, VecDeque};
use tokio::sync::{broadcast, RwLock};

pub type LogSender = broadcast::Sender<String>;

const LOG_BUFFER_MAX: usize = 1000;

pub struct AppState {
    pub db: SqlitePool,
    pub docker: Docker,
    pub servers_host_path: String,
    pub config: RwLock<AppConfig>,
    pub log_channels: RwLock<HashMap<String, LogSender>>,
    pub log_buffers: RwLock<HashMap<String, VecDeque<String>>>,
}

impl AppState {
    pub fn new(db: SqlitePool, docker: Docker, servers_host_path: String, config: AppConfig) -> Self {
        Self {
            db,
            docker,
            servers_host_path,
            config: RwLock::new(config),
            log_channels: RwLock::new(HashMap::new()),
            log_buffers: RwLock::new(HashMap::new()),
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
