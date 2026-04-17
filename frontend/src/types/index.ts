export type UserRole = 'admin' | 'user'

export interface AuthUser {
  id:          string
  username:    string
  role:        UserRole
  permissions: string[]
  settings:    Record<string, unknown>
  created_at:  string
}

export interface AuthResponse {
  token: string
  user:  AuthUser
}

export const ALL_PERMISSIONS = [
  { key: 'servers.view',       label: 'View Servers',      description: 'See the server list and details' },
  { key: 'servers.create',     label: 'Create Servers',    description: 'Spin up new Minecraft servers' },
  { key: 'servers.delete',     label: 'Delete Servers',    description: 'Permanently delete servers' },
  { key: 'servers.power',      label: 'Power Control',     description: 'Start, stop, and restart servers' },
  { key: 'servers.console',    label: 'Console',           description: 'View and send console commands' },
  { key: 'servers.files',      label: 'File Browser',      description: 'Browse and edit server files' },
  { key: 'servers.settings',   label: 'Server Settings',   description: 'Change server configuration' },
  { key: 'servers.players',    label: 'Player Monitoring', description: 'View connected players' },
  { key: 'servers.moderation', label: 'Moderation',        description: 'Whitelist, ban, and op management' },
  { key: 'servers.modrinth',   label: 'Modrinth',          description: 'Browse and install mods' },
  { key: 'analytics.view',     label: 'Analytics',         description: 'View analytics and metrics' },
  { key: 'mods.browse',        label: 'Mod Browser',       description: 'Global mod browser access' },
] as const

export type Permission = typeof ALL_PERMISSIONS[number]['key']

export type ServerStatus = 'stopped' | 'starting' | 'running' | 'stopping' | 'error'
export type ServerLoader = 'VANILLA' | 'PAPER' | 'FABRIC' | 'FORGE' | 'NEOFORGE' | 'QUILT'

export interface Server {
  id: string
  name: string
  description: string
  container_id: string | null
  status: ServerStatus
  loader: ServerLoader
  mc_version: string
  port: number
  rcon_port: number
  max_players: number
  memory_mb: number
  map_mod: string | null
  online_players: number
  online_mode: boolean
  auto_start: boolean
  auto_start_delay: number
  crash_detection: boolean
  shutdown_timeout: number
  show_on_status_page: boolean
  data_dir: string
  created_at: string
  updated_at: string
}

export interface CreateServerRequest {
  name: string
  description?: string
  loader: ServerLoader
  mc_version: string
  port: number
  max_players: number
  memory_mb: number
  map_mod?: string | null
  online_mode?: boolean
  auto_start?: boolean
  auto_start_delay?: number
  crash_detection?: boolean
  shutdown_timeout?: number
  show_on_status_page?: boolean
  min_memory_mb?: number
  jvm_flags?: string
  pause_when_empty_seconds?: number
  difficulty?: string
  gamemode?: string
  simulation_distance?: number
  view_distance?: number
}

export interface RuntimeOptions {
  min_memory_mb: number | null
  jvm_flags: string | null
  pause_when_empty_seconds: number | null
}

export interface StorageUsage {
  bytes: number
  mb: number
  gb: number
}

export interface WorldInfo {
  difficulty: string | null
  gamemode: string | null
  simulation_distance: number | null
  view_distance: number | null
  white_list: boolean | null
  online_mode: boolean | null
}

export interface WorldSettings {
  difficulty: string | null
  gamemode: string | null
  simulation_distance: number | null
  view_distance: number | null
}

export interface ModrinthProjects {
  projects: string[]
}

export interface PlayerSession {
  id: string
  server_id: string
  player_uuid: string
  player_name: string
  joined_at: string
  left_at: string | null
  duration_seconds: number | null
}

export interface MetricPoint {
  timestamp: string
  online_players: number
  cpu_percent: number
  memory_mb: number
  tps: number
}

export interface AppConfig {
  domain: string
  velocity_enabled: boolean
  velocity_secret: string
  velocity_container: string
  traefik_enabled: boolean
  device_hostname?: string
}

export interface ServerSummary {
  total_sessions: number
  unique_players: number
  peak_players: number
}

export interface WhitelistEntry {
  uuid:    string
  name:    string
  created: string
  expires: string
}

export interface WhitelistState {
  enabled: boolean
}

export interface BanEntry {
  uuid:    string
  name:    string
  created: string
  source:  string
  expires: string
  reason:  string
}

export interface OpEntry {
  uuid: string
  name: string
  level: number
  bypassesPlayerLimit: boolean
}

export interface FileEntry {
  name: string
  path: string
  is_dir: boolean
  size: number
  modified: string
}

export interface WorldEntry {
  name: string
  size: number
}

export interface BackupEntry {
  name:       string
  size:       number
  created_at: number
}

export interface LogLine {
  line: number
  text: string
}

export interface ModrinthProject {
  project_id: string
  slug: string
  title: string
  description: string
  author: string
  downloads: number
  follows: number
  categories: string[]
  icon_url: string | null
  versions: string[]
  latest_version: string
}
