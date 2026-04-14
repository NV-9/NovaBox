const BASE = '/api'

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { 'Content-Type': 'application/json', ...init?.headers },
    ...init,
  })
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }))
    throw new Error(err.error ?? res.statusText)
  }
  if (res.status === 204) return undefined as unknown as T
  return res.json()
}

import type {
  Server, CreateServerRequest, PlayerSession, MetricPoint, ServerSummary, AppConfig,
  WhitelistEntry, BanEntry, WhitelistState, RuntimeOptions, StorageUsage,
} from '@/types'

export const api = {
  servers: {
    list:     () => request<Server[]>('/servers'),
    get:      (id: string) => request<Server>(`/servers/${id}`),
    create:   (data: CreateServerRequest) =>
      request<Server>('/servers', { method: 'POST', body: JSON.stringify(data) }),
    update:   (id: string, data: Partial<CreateServerRequest>) =>
      request<Server>(`/servers/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
    settings: (id: string, data: Partial<CreateServerRequest>) =>
      request<Server>(`/servers/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
    delete:   (id: string) => request<void>(`/servers/${id}`, { method: 'DELETE' }),
    start:    (id: string) => request<{ status: string }>(`/servers/${id}/start`, { method: 'POST' }),
    stop:     (id: string) => request<{ status: string }>(`/servers/${id}/stop`, { method: 'POST' }),
    kill:     (id: string) => request<{ status: string }>(`/servers/${id}/kill`, { method: 'POST' }),
    restart:  (id: string) => request<{ status: string }>(`/servers/${id}/restart`, { method: 'POST' }),
    storage:  (id: string) => request<StorageUsage>(`/servers/${id}/storage`),
    runtimeOptions: (id: string) => request<RuntimeOptions>(`/servers/${id}/runtime`),
    setRuntimeOptions: (id: string, data: RuntimeOptions) =>
      request<RuntimeOptions>(`/servers/${id}/runtime`, { method: 'PUT', body: JSON.stringify(data) }),
    command:  (id: string, command: string) =>
      request<{ output: string }>(`/servers/${id}/command`, {
        method: 'POST',
        body: JSON.stringify({ command }),
      }),
  },

  players: {
    sessions: (serverId: string, limit = 50, offset = 0) =>
      request<PlayerSession[]>(`/players/${serverId}/sessions?limit=${limit}&offset=${offset}`),
    online: (serverId: string) =>
      request<PlayerSession[]>(`/players/${serverId}/online`),
  },

  metrics: {
    history: (serverId: string, hours = 24) =>
      request<MetricPoint[]>(`/metrics/${serverId}?hours=${hours}`),
    summary: (serverId: string) =>
      request<ServerSummary>(`/metrics/${serverId}/summary`),
  },

  settings: {
    get:    () => request<AppConfig>('/settings'),
    update: (data: AppConfig) =>
      request<AppConfig>('/settings', { method: 'PUT', body: JSON.stringify(data) }),
  },

  moderation: {
    whitelist:       (id: string) =>
      request<WhitelistEntry[]>(`/servers/${id}/whitelist`),
    whitelistState:  (id: string) =>
      request<WhitelistState>(`/servers/${id}/whitelist/state`),
    setWhitelistState: (id: string, enabled: boolean) =>
      request<WhitelistState>(`/servers/${id}/whitelist/state`, {
        method: 'PUT',
        body: JSON.stringify({ enabled }),
      }),
    addWhitelist:    (id: string, name: string) =>
      request<WhitelistEntry[]>(`/servers/${id}/whitelist`, { method: 'POST', body: JSON.stringify({ name }) }),
    removeWhitelist: (id: string, name: string) =>
      request<WhitelistEntry[]>(`/servers/${id}/whitelist/${encodeURIComponent(name)}`, { method: 'DELETE' }),
    bans:            (id: string) =>
      request<BanEntry[]>(`/servers/${id}/bans`),
    addBan:          (id: string, name: string, reason?: string) =>
      request<BanEntry[]>(`/servers/${id}/bans`, { method: 'POST', body: JSON.stringify({ name, reason: reason ?? '' }) }),
    removeBan:       (id: string, name: string) =>
      request<BanEntry[]>(`/servers/${id}/bans/${encodeURIComponent(name)}`, { method: 'DELETE' }),
  },

  modrinth: {
    search: (q: string, loader?: string, gameVersion?: string, limit = 20) => {
      const params = new URLSearchParams({ q, limit: String(limit) })
      if (loader) params.set('loader', loader)
      if (gameVersion) params.set('game_version', gameVersion)
      return request<{ hits: any[]; total_hits: number }>(`/modrinth/search?${params}`)
    },
  },
}

export function createConsoleSocket(serverId: string): WebSocket {
  const protocol = location.protocol === 'https:' ? 'wss' : 'ws'
  return new WebSocket(`${protocol}://${location.host}/ws/console/${serverId}`)
}
