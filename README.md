# NovaBox

NovaBox turns any server, NAS, or PC into a full Minecraft hosting platform. It handles container orchestration, mod compatibility, live maps, RCON, backups, and a real-time web panel — all from a single Docker Compose file.

---

## Requirements

TBC

---

## Quick Start

```bash
git clone https://github.com/your-org/novabox
cd novabox/deploy/local

cp .env.example .env
# Edit .env — set DATA_PATH, SERVERS_HOST_PATH, CARGO_CACHE_PATH

docker network create novabox-mc-net-dev
docker compose up -d
```

Open **http://lvh.me** in your browser.

> **Windows / Docker Desktop:** Use real host paths in `.env` (e.g. `C:/Users/you/novabox/data`).  
> The backend mounts `SERVERS_HOST_PATH` directly into child containers via the Docker socket, so the path must resolve on the Docker host.

---

## Configuration

### `deploy/local/.env`

| Variable | Description | Default |
|---|---|---|
| `DATA_PATH` | SQLite DB, velocity.toml, and app data | `../../data` |
| `SERVERS_HOST_PATH` | Minecraft server data directories | `../../servers` |
| `CARGO_CACHE_PATH` | Cargo registry cache (speeds up rebuilds) | `../../cargo-cache` |
| `DOMAIN` | Base domain for per-server subdomains | `lvh.me` |
| `VELOCITY_API_SECRET` | Shared secret for the Velocity HTTP API plugin | *(empty)* |

---

## Architecture

```
Browser
  └── Traefik (port 80)
        ├── lvh.me            →  Frontend (React + Vite, port 3000)
        └── map.{id}.lvh.me  →  Minecraft map mod (Dynmap / BlueMap)

Frontend  →  NovaBox Backend (Rust/Axum, port 8080)
                ├── Docker socket  →  spawns/monitors Minecraft containers
                ├── SQLite  →  server config, metrics, logs
                ├── RCON  →  in-container command execution
                └── Velocity HTTP API plugin  →  dynamic proxy registration

Velocity proxy (port 25565)
  ├── {shortId}.domain  →  Minecraft server A
  └── {shortId}.domain  →  Minecraft server B
```

### Services

| Container | Image | Role |
|---|---|---|
| `novabox-traefik-dev` | `traefik:v3.6` | Reverse proxy, routes panel + map URLs |
| `novabox-master-dev` | Custom Rust build (hot-reload) | Backend API + container manager |
| `novabox-frontend-dev` | `node:20-alpine` | React dev server (Vite HMR) |
| `novabox-velocity-dev` | Custom (itzg/mc-proxy + plugin) | Minecraft reverse proxy |
| *(per server)* | `itzg/minecraft-server` | Isolated Minecraft container |

### Velocity Dynamic Registration

A custom Java plugin built into the Velocity image exposes an HTTP API on port 7000. When NovaBox starts or stops a Minecraft server, it calls this API instead of rewriting `velocity.toml` or restarting any container:

- `POST   /servers`        — registers a backend (`proxy.registerServer()`)
- `DELETE /servers/{name}` — unregisters a backend
- `GET    /servers`        — lists all registered backends
- `GET    /health`         — health check

The initial `velocity.toml` is written at backend startup for Velocity's first boot. After that, registration is fully dynamic.

### Fabric Forwarding

When a Fabric server is started with Velocity enabled, NovaBox automatically:
1. Writes a `FabricProxy-Lite.toml` config into the server's data directory with the correct forwarding secret
2. Installs the `fabricproxy-lite` mod via `MODRINTH_PROJECTS`

This means Fabric servers work with Velocity modern forwarding out of the box.

---

## Development

The backend hot-reloads via `cargo-watch` inside the dev container. The frontend uses Vite HMR. Rebuild triggers:

```bash
# Rebuild velocity image after plugin changes
docker compose build velocity
docker compose up -d velocity

# Tail all logs
docker compose logs -f

# Tail a specific service
docker compose logs -f novabox
```

### Project Layout

```
novabox/
├── backend/              Rust (Axum + SQLx + Bollard)
│   ├── src/
│   │   ├── main.rs
│   │   ├── api/          HTTP handlers (servers, players, metrics, moderation, settings, ws)
│   │   ├── docker/       Container lifecycle + monitor loop
│   │   ├── velocity.rs   Velocity toml generation + HTTP API client
│   │   ├── rcon.rs       RCON client
│   │   └── mc_ping.rs    Server list ping
│   ├── migrations/       SQLite schema
│   └── Dockerfile.dev    Dev image with cargo-watch
├── frontend/             React + TypeScript + Vite
│   └── src/
│       ├── pages/        Dashboard, Servers, ServerDetail, Analytics, Players, …
│       ├── pages/server/ Per-server tab components
│       └── components/   Shared UI (ServerCard, ConsolePanel, StatusBadge, …)
├── velocity-plugin/      Java Maven — Velocity HTTP API plugin
│   ├── pom.xml
│   ├── Dockerfile.velocity   Multi-stage: Maven build → itzg/mc-proxy
│   └── src/
└── deploy/
    └── local/            docker-compose.yml + .env.example
```

---

## License

MIT
