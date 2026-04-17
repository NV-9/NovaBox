const BASE = '/api'

function getToken(): string | null {
  return localStorage.getItem('novabox_token')
}

function blobDownload(url: string, filename: string) {
  const token = getToken()
  fetch(url, { headers: token ? { Authorization: `Bearer ${token}` } : {} })
    .then(r => r.ok ? r.blob() : Promise.reject(r.statusText))
    .then(blob => {
      const a = document.createElement('a')
      a.href = URL.createObjectURL(blob)
      a.download = filename
      a.click()
      URL.revokeObjectURL(a.href)
    })
    .catch(err => console.error('Download failed:', err))
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const token = getToken()
  const res = await fetch(`${BASE}${path}`, {
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...init?.headers,
    },
    ...init,
  })
  if (!res.ok) {
    if (res.status === 401) {
      localStorage.removeItem('novabox_token')
      window.location.href = '/login'
    }
    const err = await res.json().catch(() => ({ error: res.statusText }))
    throw new Error(err.error ?? res.statusText)
  }
  if (res.status === 204) return undefined as unknown as T
  return res.json()
}

import type {
  Server, CreateServerRequest, PlayerSession, MetricPoint, ServerSummary, AppConfig,
  WhitelistEntry, BanEntry, WhitelistState, RuntimeOptions, StorageUsage,
  OpEntry, FileEntry, WorldInfo, WorldEntry, WorldSettings, ModrinthProjects,
  AuthUser, BackupEntry, LogLine,
} from '@/types'

export const api = {
  servers: {
    list:     () => request<Server[]>('/servers'),
    get:      (id: string) => request<Server>(`/servers/${id}`),
    worldInfo:(id: string) => request<WorldInfo>(`/servers/${id}/world-info`),
    worldSettings: (id: string) => request<WorldSettings>(`/servers/${id}/world-settings`),
    setWorldSettings: (id: string, data: WorldSettings) =>
      request<WorldSettings>(`/servers/${id}/world-settings`, { method: 'PUT', body: JSON.stringify(data) }),
    create:   (data: CreateServerRequest) =>
      request<Server>('/servers', { method: 'POST', body: JSON.stringify(data) }),
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
    stdin:    (id: string, command: string) =>
      request<{ status: string }>(`/servers/${id}/stdin`, {
        method: 'POST',
        body: JSON.stringify({ command }),
      }),
    command:  (id: string, command: string) =>
      request<{ output: string }>(`/servers/${id}/command`, {
        method: 'POST',
        body: JSON.stringify({ command }),
      }),
    modrinthProjects: (id: string) => request<ModrinthProjects>(`/servers/${id}/modrinth-projects`),
    setModrinthProjects: (id: string, projects: string[]) =>
      request<ModrinthProjects>(`/servers/${id}/modrinth-projects`, {
        method: 'PUT',
        body: JSON.stringify({ projects }),
      }),
    applyMapSwitch: (id: string) =>
      request<{ ok: boolean }>(`/servers/${id}/apply-map`, { method: 'POST' }),
    mapConfig: (id: string) =>
      request<string>(`/servers/${id}/map-config`),
    members: (id: string) =>
      request<{ user_id: string; username: string; added_at: string }[]>(`/servers/${id}/members`),
    addMember: (id: string, username: string) =>
      request<void>(`/servers/${id}/members`, { method: 'POST', body: JSON.stringify({ username }) }),
    removeMember: (id: string, userId: string) =>
      request<void>(`/servers/${id}/members/${encodeURIComponent(userId)}`, { method: 'DELETE' }),
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

  ops: {
    list:   (id: string) => request<OpEntry[]>(`/servers/${id}/ops`),
    add:    (id: string, name: string) => request<OpEntry[]>(`/servers/${id}/ops`, { method: 'POST', body: JSON.stringify({ name }) }),
    remove: (id: string, name: string) => request<OpEntry[]>(`/servers/${id}/ops/${encodeURIComponent(name)}`, { method: 'DELETE' }),
  },

  files: {
    list:        (id: string, path: string) => request<FileEntry[]>(`/servers/${id}/files?path=${encodeURIComponent(path)}`),
    content:     async (id: string, path: string) => {
      const token = getToken()
      const res = await fetch(`/api/servers/${id}/files/content?path=${encodeURIComponent(path)}`, {
        headers: { ...(token ? { Authorization: `Bearer ${token}` } : {}) },
      })
      if (!res.ok) {
        const errText = await res.text().catch(() => res.statusText)
        throw new Error(errText || res.statusText)
      }
      return res.text()
    },
    saveContent: async (id: string, path: string, content: string) => {
      const token = getToken()
      const res = await fetch(`/api/servers/${id}/files/content?path=${encodeURIComponent(path)}`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'text/plain; charset=utf-8',
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
        body: content,
      })
      if (!res.ok) {
        const errText = await res.text().catch(() => res.statusText)
        throw new Error(errText || res.statusText)
      }
    },
    delete:      (id: string, path: string) => request<void>(`/servers/${id}/files?path=${encodeURIComponent(path)}`, { method: 'DELETE' }),
    download:    (id: string, path: string) =>
      blobDownload(
        `/api/servers/${id}/files/download?path=${encodeURIComponent(path)}`,
        path.split('/').pop() || 'download',
      ),
    upload:      (id: string, dir: string, files: FileList) => {
      const token = getToken()
      const form = new FormData()
      Array.from(files).forEach(f => form.append('files', f))
      return fetch(`/api/servers/${id}/files/upload?path=${encodeURIComponent(dir)}`, {
        method: 'POST',
        headers: { ...(token ? { Authorization: `Bearer ${token}` } : {}) },
        body: form,
      }).then(r => { if (!r.ok) throw new Error(r.statusText) })
    },
    worlds:      (id: string) => request<WorldEntry[]>(`/servers/${id}/worlds`),
    deleteWorld: (id: string, name: string) => request<void>(`/servers/${id}/worlds/${encodeURIComponent(name)}`, { method: 'DELETE' }),
  },

  logs: {
    search: (id: string, q?: string, limit?: number) => {
      const params = new URLSearchParams()
      if (q) params.set('q', q)
      if (limit) params.set('limit', String(limit))
      return request<LogLine[]>(`/servers/${id}/logs?${params}`)
    },
  },

  backups: {
    list:     (id: string) => request<BackupEntry[]>(`/servers/${id}/backups`),
    create:   (id: string) => request<BackupEntry>(`/servers/${id}/backups`, { method: 'POST' }),
    delete:   (id: string, name: string) =>
      request<void>(`/servers/${id}/backups/${encodeURIComponent(name)}`, { method: 'DELETE' }),
    download: (id: string, name: string) =>
      blobDownload(`/api/servers/${id}/backups/${encodeURIComponent(name)}/download`, name),
  },

  modrinth: {
    search: (q: string, loader?: string, gameVersion?: string, limit = 20) => {
      const params = new URLSearchParams({ q, limit: String(limit) })
      if (loader) params.set('loader', loader)
      if (gameVersion) params.set('game_version', gameVersion)
      return request<{ hits: any[]; total_hits: number }>(`/modrinth/search?${params}`)
    },
  },

  users: {
    list:   () => request<AuthUser[]>('/users'),
    get:    (id: string) => request<AuthUser>(`/users/${id}`),
    create: (data: { username: string; password: string; role?: string; permissions?: string[] }) =>
      request<AuthUser>('/users', { method: 'POST', body: JSON.stringify(data) }),
    update: (id: string, data: { username?: string; password?: string; role?: string; permissions?: string[] }) =>
      request<AuthUser>(`/users/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
    delete: (id: string) => request<void>(`/users/${id}`, { method: 'DELETE' }),
    getSettings:    (id: string) => request<Record<string, unknown>>(`/users/${id}/settings`),
    updateSettings: (id: string, settings: Record<string, unknown>) =>
      request<void>(`/users/${id}/settings`, { method: 'PUT', body: JSON.stringify(settings) }),
  },
}

export function createConsoleSocket(serverId: string): WebSocket {
  const protocol = location.protocol === 'https:' ? 'wss' : 'ws'
  const token = getToken() ?? ''
  return new WebSocket(
    `${protocol}://${location.host}/ws/console/${serverId}?token=${encodeURIComponent(token)}`
  )
}
