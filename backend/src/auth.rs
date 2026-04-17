use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

pub const PERM_SERVERS_VIEW:       &str = "servers.view";
pub const PERM_SERVERS_CREATE:     &str = "servers.create";
pub const PERM_SERVERS_DELETE:     &str = "servers.delete";
pub const PERM_SERVERS_POWER:      &str = "servers.power";
pub const PERM_SERVERS_CONSOLE:    &str = "servers.console";
pub const PERM_SERVERS_FILES:      &str = "servers.files";
pub const PERM_SERVERS_SETTINGS:   &str = "servers.settings";
pub const PERM_SERVERS_PLAYERS:    &str = "servers.players";
pub const PERM_SERVERS_MODERATION: &str = "servers.moderation";
pub const PERM_SERVERS_MODRINTH:   &str = "servers.modrinth";
pub const PERM_ANALYTICS_VIEW:     &str = "analytics.view";
pub const PERM_MODS_BROWSE:        &str = "mods.browse";

pub const ALL_PERMISSIONS: &[&str] = &[
    PERM_SERVERS_VIEW,
    PERM_SERVERS_CREATE,
    PERM_SERVERS_DELETE,
    PERM_SERVERS_POWER,
    PERM_SERVERS_CONSOLE,
    PERM_SERVERS_FILES,
    PERM_SERVERS_SETTINGS,
    PERM_SERVERS_PLAYERS,
    PERM_SERVERS_MODERATION,
    PERM_SERVERS_MODRINTH,
    PERM_ANALYTICS_VIEW,
    PERM_MODS_BROWSE,
];

pub const DEFAULT_USER_PERMISSIONS: &[&str] = &[
    PERM_SERVERS_VIEW,
    PERM_SERVERS_POWER,
    PERM_SERVERS_CONSOLE,
    PERM_SERVERS_PLAYERS,
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id:            String,
    pub username:      String,
    pub password_hash: String,
    pub role:          Role,
    pub permissions:   Vec<String>,
    pub settings:      serde_json::Value,
    pub created_at:    String,
}

impl User {
    pub fn has_permission(&self, perm: &str) -> bool {
        self.role == Role::Admin || self.permissions.iter().any(|p| p == perm)
    }
}

#[derive(Debug, Clone)]
pub struct AuthStore {
    users_path:    std::path::PathBuf,
    sessions_path: std::path::PathBuf,
    sessions:      Arc<RwLock<HashMap<String, String>>>,
}

impl AuthStore {
    pub fn new(data_dir: &str) -> Self {
        let users_path = std::path::PathBuf::from(data_dir).join("users.json");
        let sessions_path = std::path::PathBuf::from(data_dir).join("sessions.json");

        let initial_sessions: HashMap<String, String> = std::fs::read_to_string(&sessions_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Self {
            users_path,
            sessions_path,
            sessions: Arc::new(RwLock::new(initial_sessions)),
        }
    }

    async fn save_sessions(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.sessions_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let snapshot = self.sessions.read().await.clone();
        let json = serde_json::to_string_pretty(&snapshot)?;
        tokio::fs::write(&self.sessions_path, json).await?;
        Ok(())
    }

    pub async fn load_users(&self) -> Vec<User> {
        match tokio::fs::read_to_string(&self.users_path).await {
            Ok(s)  => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => vec![],
        }
    }

    pub async fn save_users(&self, users: &[User]) -> anyhow::Result<()> {
        if let Some(parent) = self.users_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let json = serde_json::to_string_pretty(users)?;
        tokio::fs::write(&self.users_path, json).await?;
        Ok(())
    }

    pub async fn needs_setup(&self) -> bool {
        self.load_users().await.is_empty()
    }

    pub async fn find_by_id(&self, id: &str) -> Option<User> {
        self.load_users().await.into_iter().find(|u| u.id == id)
    }

    pub async fn find_by_username(&self, username: &str) -> Option<User> {
        let lower = username.to_lowercase();
        self.load_users().await.into_iter().find(|u| u.username.to_lowercase() == lower)
    }

    pub async fn create_session(&self, user_id: String) -> String {
        let token = Uuid::new_v4().to_string();
        self.sessions.write().await.insert(token.clone(), user_id);
        if let Err(e) = self.save_sessions().await {
            tracing::warn!(error=%e, "Could not persist sessions file after login");
        }
        token
    }

    pub async fn resolve_token(&self, token: &str) -> Option<String> {
        self.sessions.read().await.get(token).cloned()
    }

    pub async fn revoke_token(&self, token: &str) {
        self.sessions.write().await.remove(token);
        if let Err(e) = self.save_sessions().await {
            tracing::warn!(error=%e, "Could not persist sessions file after logout");
        }
    }
}

pub fn hash_password(password: &str) -> String {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("argon2 hashing should not fail")
        .to_string()
}

pub fn verify_password(password: &str, stored: &str) -> bool {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };
    let Ok(parsed) = PasswordHash::new(stored) else { return false };
    Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
}
