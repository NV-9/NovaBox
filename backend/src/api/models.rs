use serde::{Deserialize, Serialize};

pub trait IntoStr {
    fn into_str(self) -> String;
}
impl IntoStr for String         { fn into_str(self) -> String { self } }
impl IntoStr for Option<String> { fn into_str(self) -> String { self.unwrap_or_default() } }

pub fn s<T: IntoStr>(v: T) -> String { v.into_str() }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub description: String,
    pub container_id: Option<String>,
    pub status: ServerStatus,
    pub loader: ServerLoader,
    pub mc_version: String,
    pub port: i64,
    pub rcon_port: i64,
    pub max_players: i64,
    pub memory_mb: i64,
    pub map_mod: Option<String>,
    pub online_mode: bool,
    pub auto_start: bool,
    pub auto_start_delay: i64,
    pub crash_detection: bool,
    pub shutdown_timeout: i64,
    pub show_on_status_page: bool,
    pub online_players: i64,
    pub data_dir: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

impl std::fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped  => write!(f, "stopped"),
            Self::Starting => write!(f, "starting"),
            Self::Running  => write!(f, "running"),
            Self::Stopping => write!(f, "stopping"),
            Self::Error    => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for ServerStatus {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "stopped"  => Self::Stopped,
            "starting" => Self::Starting,
            "running"  => Self::Running,
            "stopping" => Self::Stopping,
            "error"    => Self::Error,
            _          => Self::Stopped,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ServerLoader {
    Vanilla,
    Paper,
    Fabric,
    Forge,
    NeoForge,
    Quilt,
}

impl std::fmt::Display for ServerLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vanilla  => write!(f, "VANILLA"),
            Self::Paper    => write!(f, "PAPER"),
            Self::Fabric   => write!(f, "FABRIC"),
            Self::Forge    => write!(f, "FORGE"),
            Self::NeoForge => write!(f, "NEOFORGE"),
            Self::Quilt    => write!(f, "QUILT"),
        }
    }
}

impl std::str::FromStr for ServerLoader {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "PAPER"    => Self::Paper,
            "FABRIC"   => Self::Fabric,
            "FORGE"    => Self::Forge,
            "NEOFORGE" => Self::NeoForge,
            "QUILT"    => Self::Quilt,
            _          => Self::Vanilla,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_loader")]
    pub loader: String,
    #[serde(default = "default_version")]
    pub mc_version: String,
    #[serde(default = "default_port")]
    pub port: i64,
    #[serde(default = "default_max_players")]
    pub max_players: i64,
    #[serde(default = "default_memory")]
    pub memory_mb: i64,
    pub map_mod: Option<String>,
    #[serde(default = "default_online_mode")]
    pub online_mode: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub auto_start_delay: i64,
    #[serde(default = "default_crash_detection")]
    pub crash_detection: bool,
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout: i64,
    #[serde(default)]
    pub show_on_status_page: bool,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub gamemode: Option<String>,
    #[serde(default)]
    pub simulation_distance: Option<i64>,
    #[serde(default)]
    pub view_distance: Option<i64>,
    #[serde(default)]
    pub pause_when_empty_seconds: Option<i64>,
}

fn default_loader()           -> String { "VANILLA".to_string() }
fn default_online_mode()      -> bool   { true }
fn default_crash_detection()  -> bool   { true }
fn default_shutdown_timeout() -> i64    { 30 }
fn default_version()          -> String { "LATEST".to_string() }
fn default_port()             -> i64    { 25565 }
fn default_max_players()      -> i64    { 20 }
fn default_memory()           -> i64    { 2048 }

#[derive(Debug, Deserialize)]
pub struct RconCommandRequest {
    pub command: String,
}

#[derive(Debug, Serialize)]
pub struct PlayerSession {
    pub id: String,
    pub server_id: String,
    pub player_uuid: String,
    pub player_name: String,
    pub joined_at: String,
    pub left_at: Option<String>,
    pub duration_seconds: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct MetricPoint {
    pub timestamp: String,
    pub online_players: i64,
    pub cpu_percent: f64,
    pub memory_mb: f64,
    pub tps: f64,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl ErrorResponse {
    pub fn new(msg: impl ToString) -> Self {
        Self { error: msg.to_string() }
    }
}
