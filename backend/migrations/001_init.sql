CREATE TABLE IF NOT EXISTS servers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    container_id TEXT,
    status TEXT NOT NULL DEFAULT 'stopped',
    loader TEXT NOT NULL DEFAULT 'VANILLA',
    mc_version TEXT NOT NULL DEFAULT 'LATEST',
    port INTEGER NOT NULL DEFAULT 25565,
    rcon_port INTEGER NOT NULL DEFAULT 25575,
    rcon_password TEXT NOT NULL,
    max_players INTEGER NOT NULL DEFAULT 20,
    memory_mb INTEGER NOT NULL DEFAULT 2048,
    map_mod TEXT,
    online_mode INTEGER NOT NULL DEFAULT 1,
    auto_start INTEGER NOT NULL DEFAULT 0,
    auto_start_delay INTEGER NOT NULL DEFAULT 0,
    crash_detection INTEGER NOT NULL DEFAULT 1,
    shutdown_timeout INTEGER NOT NULL DEFAULT 30,
    show_on_status_page INTEGER NOT NULL DEFAULT 0,
    data_dir TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS server_mods (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    mod_id TEXT NOT NULL,
    mod_name TEXT NOT NULL,
    version_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    mod_type TEXT NOT NULL DEFAULT 'mod',
    installed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS player_sessions (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    player_uuid TEXT NOT NULL,
    player_name TEXT NOT NULL,
    joined_at TEXT NOT NULL DEFAULT (datetime('now')),
    left_at TEXT,
    duration_seconds INTEGER
);

CREATE TABLE IF NOT EXISTS player_events (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    player_uuid TEXT,
    player_name TEXT,
    event_type TEXT NOT NULL,
    data TEXT NOT NULL DEFAULT '{}',
    occurred_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS server_metrics (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    online_players INTEGER NOT NULL DEFAULT 0,
    cpu_percent REAL NOT NULL DEFAULT 0,
    memory_mb REAL NOT NULL DEFAULT 0,
    tps REAL NOT NULL DEFAULT 20
);

CREATE TABLE IF NOT EXISTS backups (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_sessions_server ON player_sessions(server_id);
CREATE INDEX IF NOT EXISTS idx_sessions_player ON player_sessions(player_uuid);
CREATE INDEX IF NOT EXISTS idx_events_server ON player_events(server_id, occurred_at);
CREATE INDEX IF NOT EXISTS idx_metrics_server ON server_metrics(server_id, timestamp);
