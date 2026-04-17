# NovaBox

NovaBox turns any server, NAS, or PC into a self-hosted Minecraft hosting platform. It handles container orchestration, a real-time web panel, RCON, live maps, backups, mod installation, and player moderation — all from a single Docker Compose file.

## Built with

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Axum](https://img.shields.io/badge/Axum-111111?style=for-the-badge)](https://github.com/tokio-rs/axum)
[![SQLite](https://img.shields.io/badge/SQLite-003B57?style=for-the-badge&logo=sqlite&logoColor=white)](https://www.sqlite.org/)
[![SQLx](https://img.shields.io/badge/SQLx-222222?style=for-the-badge)](https://github.com/launchbadge/sqlx)
[![React](https://img.shields.io/badge/React-20232A?style=for-the-badge&logo=react&logoColor=61DAFB)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=for-the-badge&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Tailwind CSS](https://img.shields.io/badge/Tailwind_CSS-0F172A?style=for-the-badge&logo=tailwindcss&logoColor=38BDF8)](https://tailwindcss.com/)
[![Docker](https://img.shields.io/badge/Docker-2496ED?style=for-the-badge&logo=docker&logoColor=white)](https://www.docker.com/)
[![Traefik](https://img.shields.io/badge/Traefik-24A1C1?style=for-the-badge&logo=traefikproxy&logoColor=white)](https://traefik.io/)
[![Velocity](https://img.shields.io/badge/Velocity_Proxy-3B82F6?style=for-the-badge)](https://papermc.io/software/velocity)

---

## Requirements

- Docker (with Compose v2) and access to the Docker socket
- Docker network `novabox-mc-net-dev` pre-created (`docker network create novabox-mc-net-dev`)
- Ports 80 and 25565 available on the host

---

## Quick Start

```bash
git clone https://github.com/your-org/novabox
cd novabox/deploy/local

docker network create novabox-mc-net-dev
docker compose up -d
```

Open **http://lvh.me** in your browser. On first load you will be prompted to create an admin account.

> **Windows / Docker Desktop:** The backend mounts `SERVERS_HOST_PATH` into child Minecraft containers via the Docker socket — the path must be a real host path that Docker Desktop can bind-mount (e.g. `C:/Users/you/novabox/data/servers`). Set it in `.env` before starting.

---

## Configuration

Environment variables are read from `deploy/local/.env` (copy from `.env.example` or set inline).

| Variable | Description | Default |
|---|---|---|
| `DATA_PATH` | SQLite DB, Velocity config, session data | `../../data` |
| `SERVERS_HOST_PATH` | Host path bound into Minecraft containers as `/data` | `../../data/servers` |
| `CARGO_CACHE_PATH` | Cargo registry cache (speeds up backend rebuilds) | `../../data/cargo-cache` |
| `DOMAIN` | Base domain for per-server subdomains | `lvh.me` |
| `VELOCITY_API_SECRET` | Shared secret for the Velocity HTTP plugin | *(empty)* |

Runtime variables injected into the backend container:

| Variable | Description | Default |
|---|---|---|
| `ALLOWED_ORIGINS` | CORS allowed origins (comma-separated, or `*`) | `localhost:5173,localhost:8080,127.0.0.1:8080` |
| `RCON_IDLE_TIMEOUT_SECONDS` | Close idle RCON connections after this many seconds | `120` |
| `DOCKER_NETWORK` | Docker network Minecraft containers join | `novabox-mc-net-dev` |

---

## Architecture

```
Browser
  └── Traefik (port 80)
        ├── lvh.me                →  Frontend (React + Vite)
        └── map.{shortId}.domain →  BlueMap / Dynmap (per-server)

Frontend  →  NovaBox Backend (Rust / Axum, port 8080)
                ├── Docker socket  →  spawns & monitors Minecraft containers
                ├── SQLite         →  servers, metrics, player sessions, backups
                ├── users.json     →  accounts & sessions (Argon2id passwords)
                ├── RCON           →  in-container command execution
                └── Velocity HTTP plugin  →  dynamic proxy registration (port 7000)

Velocity proxy (port 25565)
  ├── {shortId}.domain  →  Minecraft server A
  └── {shortId}.domain  →  Minecraft server B
```

---

## Features

### Authentication & Users

- **First-run setup** — guided wizard creates the initial admin account; the setup endpoint closes automatically once an admin exists.
- **Session tokens** — UUID v4 bearer tokens persisted to `sessions.json`; survive backend restarts.
- **Argon2id** password hashing (memory-hard, GPU/ASIC resistant).
- **Roles** — `admin` (full access) or `user` (explicit permission grants).
- **Granular permissions** — admins grant any subset of `servers.view`, `servers.power`, `servers.console`, `servers.files`, `servers.settings`, `servers.players`, `servers.moderation`, `servers.modrinth`, `analytics.view`, `mods.browse` to regular users.
- **User management** — admin panel to create, edit, and delete accounts; update roles and permission sets.

### Navigation

- **Tabbed top bar** — Dashboard tab followed by one tab per server (with a coloured status dot), a `+` button to add a server, a Users link (admin only), and a Settings gear (admin only).
- **Server tabs** scroll horizontally when many servers are open.

### Dashboard

- **Server grid** — live online-player counts, status badges, and quick links to each server.
- **Summary metrics** — total servers online, total players online.
- **Inline analytics** — select a server and time range to view: total sessions, unique players, peak concurrent, a players-over-time bar chart, and CPU/memory line charts.

### Server Management

Each server runs as an isolated `itzg/minecraft-server` Docker container. The panel manages the full container lifecycle.

#### Power Controls
- **Start / Stop / Restart / Force Kill** — available from the server header, gated by `servers.power`.
- **Auto Start** — launch the server automatically on NovaBox startup, with optional delay.
- **Crash Detection** — restart the container up to 3 times after an unexpected exit.
- **Pause When Empty** — freeze the container after all players disconnect for a configurable number of seconds.

#### Supported Server Types
Vanilla, Fabric, Forge, Quilt, Paper, Spigot, and any type supported by `itzg/minecraft-server`.

#### Settings Tab (`servers.settings`)
- Server name, description, max players, memory (Xmx), min RAM (Xms), custom JVM flags.
- Online mode toggle.
- Difficulty, game mode, simulation distance, view distance (written to `server.properties`).
- Shutdown timeout.
- Show on status page toggle.
- Live map mod selection (None / BlueMap / Dynmap) — changing triggers a confirmation dialog, wipes old plugin data, recreates the container, and restarts.
- Auto start delay, crash detection toggle.

#### Members Tab (`admin only`)
- Add NovaBox users to a server by username — validated against the user store.
- Non-admin users can only see and interact with servers they have been explicitly added to.
- Admins always have access to all servers regardless of membership.

### Console (`servers.console`)
- **Real-time WebSocket console** — streamed from the container's stdout, authenticated via token query parameter (browsers cannot set headers on WebSocket upgrades).
- **RCON command input** — send commands and receive output in-panel.
- **stdin passthrough** — write directly to the container's process stdin as a fallback for servers that don't expose RCON.

### Logs Tab (`servers.console`)
- Search `logs/latest.log` server-side with a live query input.
- Lines are colour-coded: red for `ERROR`/exception, yellow for `WARN`, white for `INFO`, grey for other.
- Tails the last 500 lines on mount; searches return up to 500 matching lines.

### Players Tab (`servers.players`)
- Live roster of connected players.
- Historical session list with join/leave times and duration.

### Moderation Tab (`servers.moderation`)
- Whitelist management — enable/disable, add/remove players.
- Ban list — ban with optional reason, view active bans, pardon.
- Ops list — add/remove server operators.

### Files Tab (`servers.files`)
- Browse the server's data directory tree.
- View and edit text files in-panel with a code editor.
- Delete files and directories.
- Download files — uses fetch + blob so the auth header is sent.
- Upload files via drag-and-drop or file picker.
- World browser — list worlds and delete individual world folders.

### Backups Tab (`servers.files`)
- Create a compressed ZIP backup of the entire server directory.
- List backups with name, size, and creation time.
- Download backups — fetch + blob with auth header.
- Delete individual backups.
- Backups stored at `/app/data/backups/{server-id}/`.

### Modrinth Tab (`servers.modrinth`)
- Browse and search Modrinth for mods filtered by loader and Minecraft version.
- Add/remove mods from the server's `MODRINTH_PROJECTS` list — applied on next container start.

### Map Tab
- BlueMap (port 8100) or Dynmap (port 8123) embedded as an iframe.
- Readiness probe polls `settings.json` (BlueMap) or `/up/configuration` (Dynmap) every 5 seconds — shows a spinner until the plugin has finished generating its initial data (avoids the "Failed to load settings.json" error on first start).
- Refresh button forces a new probe cycle.
- Open-in-new-tab link for full-screen access.
- When routing via Traefik, maps are served at `http://map.{shortId}.{domain}` with no host port binding required.

### Overview Tab
- Live stats: players, memory (with progress bar), TPS, disk usage.
- Server info: version, loader, difficulty, game mode, simulation/view distance.
- Connection routing: local address and internet address (Velocity subdomain or direct port).
- CPU %, memory MB, and TPS line charts (last 6 hours).

---

## Access Control Model

```
Admin
  └── All servers, all tabs, all actions. Sees Users and Settings in the nav.

Regular user
  └── Only servers where they have been added as a Member.
      └── Per-server actions gated by their permission set:
          servers.power      →  Start / Stop / Restart / Kill
          servers.console    →  Console tab, Logs tab, RCON commands
          servers.files      →  Files tab, Backups tab
          servers.settings   →  Settings tab
          servers.players    →  Players tab
          servers.moderation →  Moderation tab
          servers.modrinth   →  Modrinth tab
          servers.view       →  See the server at all (list + detail)
```

Default permissions granted to new regular users: `servers.view`, `servers.power`, `servers.console`, `servers.players`.

---

## Velocity Proxy

NovaBox ships a custom Java plugin bundled into the `itzg/mc-proxy` Velocity image. Instead of editing `velocity.toml` and restarting the proxy, the backend calls the plugin's HTTP API on port 7000:

| Method | Path | Action |
|---|---|---|
| `POST` | `/servers` | Register a backend server |
| `DELETE` | `/servers/{name}` | Unregister a backend server |
| `GET` | `/servers` | List all registered backends |
| `GET` | `/health` | Health check |

The initial `velocity.toml` is written at backend startup for Velocity's first boot. All subsequent registration is dynamic — no proxy restart needed.

### Fabric Forwarding

When a Fabric server starts with Velocity enabled, NovaBox automatically:
1. Writes `FabricProxy-Lite.toml` with the correct forwarding secret into the server's config directory.
2. Injects `fabricproxy-lite` into `MODRINTH_PROJECTS`.

Fabric servers work with Velocity modern forwarding out of the box.

---

## Services

| Container | Image | Role |
|---|---|---|
| `novabox-traefik-dev` | `traefik:v3.6` | Reverse proxy — routes panel and map subdomains |
| `novabox-master-dev` | Custom Rust build (cargo-watch hot-reload) | Backend API and container manager |
| `novabox-frontend-dev` | `node:20-alpine` | React / Vite dev server with HMR |
| `novabox-velocity-dev` | Custom (`itzg/mc-proxy` + plugin) | Minecraft reverse proxy |
| *(per server)* | `itzg/minecraft-server` | Isolated Minecraft server |

---

## Development

The backend recompiles automatically via `cargo-watch` inside the dev container. The frontend uses Vite HMR.

```bash
# Rebuild Velocity image after plugin changes
docker compose build velocity
docker compose up -d velocity

# Tail all container logs
docker compose logs -f

# Tail a specific service
docker compose logs -f novabox
```

## Production Image Publishing

Tagged pushes (for example `v1.0.0`) trigger GitHub Actions to build and push production images to GHCR and publish a GitHub Release with autogenerated notes.
Each tagged push publishes two tags per image: the exact git tag (for example `v1.0.0`) and `latest`.

Workflow file:

- `.github/workflows/publish-production-images.yml`

Built images:

- `ghcr.io/NV-9/novabox-master`
- `ghcr.io/NV-9/novabox-panel`
- `ghcr.io/NV-9/novabox-velocity`

### Project Layout

```
novabox/
├── backend/                   Rust (Axum + SQLx + Bollard)
│   ├── src/
│   │   ├── main.rs            Entry point, CORS, router wiring
│   │   ├── auth.rs            User model, AuthStore, Argon2id hashing, session tokens
│   │   ├── config.rs          AppConfig (domain, Velocity, Traefik)
│   │   ├── state.rs           Shared AppState (DB pool, Docker, RCON cache, config)
│   │   ├── db.rs              SQLite init + migrations
│   │   ├── docker/            Container lifecycle (start/stop/monitor/auto-start)
│   │   ├── velocity.rs        Velocity TOML generation + HTTP API client
│   │   ├── rcon.rs            RCON client with idle-timeout pruning
│   │   ├── mc_ping.rs         Server list ping
│   │   ├── ws/                Authenticated WebSocket console handler
│   │   └── api/
│   │       ├── mod.rs         Router, require_auth middleware
│   │       ├── auth.rs        Setup, login, logout, /me
│   │       ├── servers.rs     Server CRUD, power, settings, map switch, members
│   │       ├── files.rs       File browser, editor, upload, download, worlds
│   │       ├── backups.rs     ZIP backup create/list/download/delete
│   │       ├── logs.rs        Log search (latest.log)
│   │       ├── players.rs     Session history, online roster
│   │       ├── metrics.rs     Time-series metrics + analytics summary
│   │       ├── moderation.rs  Whitelist, bans, ops
│   │       ├── modrinth.rs    Modrinth search proxy
│   │       ├── settings.rs    Global AppConfig read/write
│   │       └── users.rs       User management (admin only)
│   ├── migrations/
│   │   ├── 001_init.sql       servers, player_sessions, metrics, backups, mods
│   │   └── 002_server_members.sql  Per-server access control list
│   └── entrypoint-dev.sh
├── frontend/                  React 18 + TypeScript + Vite + Tailwind
│   └── src/
│       ├── App.tsx            Route tree, AuthGuard, PublicAuthGuard
│       ├── context/
│       │   └── AuthContext.tsx   Session restore, login/logout, can(), isAdmin
│       ├── api/
│       │   └── client.ts      Typed API wrapper (fetch + auth headers)
│       ├── components/
│       │   ├── TopBar.tsx     Horizontal nav — Dashboard, server tabs, Users, Settings
│       │   ├── ConsolePanel.tsx   WebSocket console (token via query param)
│       │   └── ...
│       ├── pages/
│       │   ├── Dashboard.tsx  Server grid + inline analytics
│       │   ├── ServerDetail.tsx   Per-server page with tab router
│       │   ├── NewServer.tsx
│       │   ├── Users.tsx      Admin user management
│       │   ├── Settings.tsx   Global config (admin only)
│       │   ├── Login.tsx
│       │   └── Setup.tsx      First-run wizard
│       └── pages/server/      Per-server tab components
│           ├── OverviewTab.tsx
│           ├── MembersTab.tsx
│           ├── PlayersTab.tsx
│           ├── ModerationTab.tsx
│           ├── FilesTab.tsx
│           ├── BackupsTab.tsx
│           ├── LogsTab.tsx
│           ├── MapTab.tsx
│           ├── ModrinthTab.tsx
│           ├── OpsTab.tsx
│           └── SettingsTab.tsx
├── velocity-plugin/           Java / Maven — Velocity HTTP API plugin
│   ├── pom.xml
│   └── src/
└── deploy/
  ├── local/
  │   ├── docker-compose.yml
  │   ├── .env.example
  │   ├── master/
  │   │   └── Dockerfile
  │   ├── panel/
  │   │   └── Dockerfile
  │   └── velocity/
  │       └── Dockerfile
  └── production/
    ├── master/
    │   └── Dockerfile
    ├── panel/
    │   └── Dockerfile
    └── velocity/
      └── Dockerfile
```

---

## API Reference (summary)

All endpoints under `/api/*` require `Authorization: Bearer <token>` except `/api/auth/*` and `/api/health`.

| Method | Path | Permission |
|---|---|---|
| `GET` | `/api/auth/setup` | Public |
| `POST` | `/api/auth/setup` | Public (first run only) |
| `POST` | `/api/auth/login` | Public |
| `POST` | `/api/auth/logout` | Authenticated |
| `GET` | `/api/auth/me` | Authenticated |
| `GET` | `/api/servers` | `servers.view` (filtered by membership) |
| `POST` | `/api/servers` | `servers.create` |
| `GET` | `/api/servers/:id` | `servers.view` + member |
| `PUT` | `/api/servers/:id` | `servers.settings` + member |
| `DELETE` | `/api/servers/:id` | `servers.delete` + member |
| `POST` | `/api/servers/:id/start` | `servers.power` + member |
| `POST` | `/api/servers/:id/stop` | `servers.power` + member |
| `POST` | `/api/servers/:id/restart` | `servers.power` + member |
| `POST` | `/api/servers/:id/kill` | `servers.power` + member |
| `POST` | `/api/servers/:id/apply-map` | `servers.settings` + member |
| `GET` | `/api/servers/:id/members` | Admin |
| `POST` | `/api/servers/:id/members` | Admin |
| `DELETE` | `/api/servers/:id/members/:uid` | Admin |
| `GET` | `/api/servers/:id/logs` | `servers.console` |
| `GET/POST/DELETE` | `/api/servers/:id/backups` | `servers.files` |
| `GET` | `/api/servers/:id/files` | `servers.files` |
| `GET/PUT` | `/api/servers/:id/files/content` | `servers.files` |
| `POST` | `/api/servers/:id/files/upload` | `servers.files` |
| `GET` | `/api/servers/:id/files/download` | `servers.files` |
| `GET/POST/DELETE` | `/api/servers/:id/whitelist` | `servers.moderation` |
| `GET/POST/DELETE` | `/api/servers/:id/bans` | `servers.moderation` |
| `GET/POST/DELETE` | `/api/servers/:id/ops` | `servers.moderation` |
| `GET/PUT` | `/api/settings` | GET: authenticated · PUT: admin |
| `GET/POST/PUT/DELETE` | `/api/users` | Admin |
| `WS` | `/ws/console/:id?token=` | `servers.console` + member |

---

## License

MIT
