use anyhow::Result;
use serde::{Deserialize, Serialize};

const CONFIG_PATH: &str = "/app/data/novabox.json";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub domain: String,
    pub velocity_enabled: bool,
    pub velocity_secret: String,
    pub velocity_container: String,
    pub traefik_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            domain:             "localhost".to_string(),
            velocity_enabled:   false,
            velocity_secret:    uuid::Uuid::new_v4().to_string().replace('-', ""),
            velocity_container: "novabox-velocity".to_string(),
            traefik_enabled:    false,
        }
    }
}

impl AppConfig {
    pub async fn load() -> Self {
        match tokio::fs::read_to_string(CONFIG_PATH).await {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
                tracing::warn!("Failed to parse novabox.json, using defaults: {e}");
                Self::default()
            }),
            Err(_) => {
                let cfg = Self::default();
                if let Err(e) = cfg.save().await {
                    tracing::warn!("Could not write initial config: {e}");
                } else {
                    tracing::info!("Created default config at {}", CONFIG_PATH);
                }
                cfg
            }
        }
    }

    pub async fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        tokio::fs::write(CONFIG_PATH, json).await?;
        Ok(())
    }
}
