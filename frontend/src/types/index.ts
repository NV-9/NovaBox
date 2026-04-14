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
}

export interface RuntimeOptions {
  min_memory_mb: number | null
  jvm_flags: string | null
}

export interface StorageUsage {
  bytes: number
  mb: number
  gb: number
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
