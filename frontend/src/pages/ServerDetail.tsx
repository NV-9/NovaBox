import { useParams, Link, useNavigate } from 'react-router-dom'
import { Play, Square, RotateCcw, ChevronLeft, Loader2, PowerOff } from 'lucide-react'
import { useState, useEffect } from 'react'
import { useServer } from '@/hooks/useServers'
import { StatusBadge } from '@/components/StatusBadge'
import { ConsolePanel } from '@/components/ConsolePanel'
import { api } from '@/api/client'
import { useAuth } from '@/context/AuthContext'
import { OverviewTab }    from './server/OverviewTab.tsx'
import { PlayersTab }     from './server/PlayersTab'
import { MapTab }         from './server/MapTab'
import { ModrinthTab }    from './server/ModrinthTab'
import { SettingsTab }    from './server/SettingsTab'
import { ModerationTab }  from './server/ModerationTab'
import { MembersTab }     from './server/MembersTab'
import { FilesTab }       from './server/FilesTab'
import { LogsTab }        from './server/LogsTab'
import { BackupsTab }     from './server/BackupsTab'
import type { MetricPoint, PlayerSession, CreateServerRequest, AppConfig, StorageUsage, WorldInfo, WorldSettings } from '@/types'

type Tab = 'overview' | 'console' | 'players' | 'moderation' | 'members' | 'files' | 'logs' | 'backups' | 'map' | 'modrinth' | 'settings'

export default function ServerDetail() {
  const { can } = useAuth()
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { server, loading, refresh } = useServer(id!)

  const [metrics,       setMetrics]       = useState<MetricPoint[]>([])
  const [sessions,      setSessions]      = useState<PlayerSession[]>([])
  const [storage,       setStorage]       = useState<StorageUsage | null>(null)
  const [worldInfo,     setWorldInfo]     = useState<WorldInfo | null>(null)
  const [worldSettings, setWorldSettings] = useState<WorldSettings | null>(null)
  const [appConfig,     setAppConfig]     = useState<AppConfig | null>(null)
  const [tab,           setTab]           = useState<Tab>('overview')
  const [actionLoading, setActionLoading] = useState(false)
  const [confirmDelete, setConfirmDelete] = useState(false)

  const [settingsForm,    setSettingsForm]    = useState<Partial<CreateServerRequest>>({})
  const [settingsSaving,  setSettingsSaving]  = useState(false)
  const [settingsSaved,   setSettingsSaved]   = useState(false)
  const [filesDirty,      setFilesDirty]      = useState(false)
  const [mapSwitchPending, setMapSwitchPending] = useState(false)
  const [mapSwitchError,   setMapSwitchError]   = useState<string | null>(null)

  useEffect(() => {
    if (!server) return
    setSettingsForm({
      name:                server.name,
      description:         server.description,
      max_players:         server.max_players,
      memory_mb:           server.memory_mb,
      map_mod:             server.map_mod,
      online_mode:         server.online_mode,
      auto_start:          server.auto_start,
      auto_start_delay:    server.auto_start_delay,
      crash_detection:     server.crash_detection,
      shutdown_timeout:    server.shutdown_timeout,
      show_on_status_page: server.show_on_status_page,
      min_memory_mb:       undefined,
      jvm_flags:           undefined,
      pause_when_empty_seconds: undefined,
      difficulty:          undefined,
      gamemode:            undefined,
      simulation_distance: undefined,
      view_distance:       undefined,
    })
  }, [server?.id])

  useEffect(() => {
    if (!id) return
    api.servers.runtimeOptions(id)
      .then(opts => setSettingsForm(f => ({
        ...f,
        min_memory_mb:            opts.min_memory_mb ?? undefined,
        jvm_flags:                opts.jvm_flags ?? undefined,
        pause_when_empty_seconds: opts.pause_when_empty_seconds ?? undefined,
      })))
      .catch(() => {})
  }, [id])

  useEffect(() => {
    if (!id) return
    api.servers.worldSettings(id)
      .then(opts => {
        setWorldSettings(opts)
        setSettingsForm(f => ({
          ...f,
          difficulty:          opts.difficulty ?? undefined,
          gamemode:            opts.gamemode ?? undefined,
          simulation_distance: opts.simulation_distance ?? undefined,
          view_distance:       opts.view_distance ?? undefined,
        }))
      })
      .catch(() => {})
  }, [id])

  useEffect(() => {
    if (!id) return
    const fetch = () => {
      api.metrics.history(id, 6).then(setMetrics).catch(() => {})
      api.players.online(id).then(setSessions).catch(() => setSessions([]))
      api.servers.storage(id).then(setStorage).catch(() => setStorage(null))
      api.servers.worldInfo(id).then(setWorldInfo).catch(() => setWorldInfo(null))
    }
    fetch()
    const t = setInterval(fetch, 10_000)
    return () => clearInterval(t)
  }, [id])

  useEffect(() => {
    api.settings.get().then(setAppConfig).catch(() => {})
  }, [])

  async function action(fn: () => Promise<any>) {
    setActionLoading(true)
    try { await fn() } finally { setActionLoading(false); refresh() }
  }

  async function deleteServer() {
    await api.servers.delete(id!)
    navigate('/')
  }

  async function performSettingsSave() {
    await api.servers.settings(id!, settingsForm)
    await api.servers.setWorldSettings(id!, {
      difficulty:          settingsForm.difficulty ?? null,
      gamemode:            settingsForm.gamemode ?? null,
      simulation_distance: settingsForm.simulation_distance ?? null,
      view_distance:       settingsForm.view_distance ?? null,
    })
    await api.servers.setRuntimeOptions(id!, {
      min_memory_mb:            settingsForm.min_memory_mb ?? null,
      jvm_flags:                settingsForm.jvm_flags?.trim() ? settingsForm.jvm_flags.trim() : null,
      pause_when_empty_seconds: settingsForm.pause_when_empty_seconds ?? null,
    })
  }

  async function saveSettings(e: React.FormEvent) {
    e.preventDefault()
    if (server && settingsForm.map_mod !== server.map_mod) {
      setMapSwitchPending(true)
      return
    }
    setSettingsSaving(true)
    try {
      await performSettingsSave()
      setSettingsSaved(true)
      refresh()
      setTimeout(() => setSettingsSaved(false), 2000)
    } finally {
      setSettingsSaving(false)
    }
  }

  async function confirmMapSwitch() {
    setMapSwitchPending(false)
    setMapSwitchError(null)
    setSettingsSaving(true)
    try {
      await performSettingsSave()
      await api.servers.applyMapSwitch(id!)
      await api.servers.start(id!)
      setSettingsSaved(true)
      refresh()
      setTab('overview')
      setTimeout(() => setSettingsSaved(false), 2500)
    } catch (err: any) {
      setMapSwitchError(err.message ?? 'Map switch failed')
    } finally {
      setSettingsSaving(false)
    }
  }

  function setS<K extends keyof CreateServerRequest>(key: K, value: CreateServerRequest[K]) {
    setSettingsForm(f => ({ ...f, [key]: value }))
  }

  if (loading && !server) {
    return (
      <div className="p-6 flex items-center gap-2 text-dark-400">
        <Loader2 className="w-5 h-5 animate-spin" /> Loading…
      </div>
    )
  }

  if (!server) {
    return (
      <div className="p-6">
        <p className="text-dark-400">Server not found.</p>
        <Link to="/" className="btn-ghost mt-4 inline-flex">Back to Dashboard</Link>
      </div>
    )
  }

  const shortId        = server.id.slice(0, 8)
  const domain         = appConfig?.domain?.trim() || window.location.hostname
  const connectAddress = appConfig?.velocity_enabled
    ? `${shortId}.${domain}:25565`
    : `${window.location.hostname}:${server.port}`

  const isStopped       = server.status === 'stopped' || server.status === 'error'
  const isRunning       = server.status === 'running'
  const isTransitioning = server.status === 'starting' || server.status === 'stopping'

  const tabs: { id: Tab; label: string }[] = [
    { id: 'overview',   label: 'Overview' },
    ...(can('servers.console')    ? [{ id: 'console'    as Tab, label: 'Console' }]    : []),
    ...(can('servers.players')    ? [{ id: 'players'    as Tab, label: 'Players' }]    : []),
    ...(can('servers.modrinth')   ? [{ id: 'modrinth'   as Tab, label: 'Modrinth' }]   : []),
    ...(can('servers.moderation') ? [{ id: 'moderation' as Tab, label: 'Moderation' }] : []),
    ...(can('servers.moderation') ? [{ id: 'members'    as Tab, label: 'Members' }]    : []),
    ...(can('servers.files')      ? [{ id: 'files'      as Tab, label: 'Files' }]      : []),
    ...(can('servers.console')    ? [{ id: 'logs'       as Tab, label: 'Logs' }]       : []),
    ...(can('servers.files')      ? [{ id: 'backups'    as Tab, label: 'Backups' }]    : []),
    ...(server.map_mod && server.status === 'running' ? [{ id: 'map' as Tab, label: 'Map' }] : []),
    ...(can('servers.settings')   ? [{ id: 'settings'   as Tab, label: 'Settings' }]   : []),
  ]

  function selectTab(next: Tab) {
    if (tab === 'files' && next !== 'files' && filesDirty) {
      const leave = confirm('You have unsaved file changes. Leave Files tab and discard them?')
      if (!leave) return
      setFilesDirty(false)
    }
    setTab(next)
  }

  return (
    <div className="p-6 space-y-5">

      {mapSwitchPending && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
          <div className="bg-dark-card border border-dark-border rounded-xl shadow-2xl p-6 max-w-md w-full mx-4 space-y-4">
            <h2 className="text-base font-bold">Switch Live Map?</h2>
            <p className="text-sm text-dark-400 leading-relaxed">
              Changing the live map mod requires the server to be stopped. The existing container
              will be recreated and old map plugin data will be permanently deleted. The server
              will restart automatically with the new configuration.
            </p>
            {mapSwitchError && (
              <p className="text-sm text-red-400 bg-red-500/10 border border-red-500/20 rounded-lg px-3 py-2">
                {mapSwitchError}
              </p>
            )}
            <div className="flex justify-end gap-2 pt-1">
              <button
                type="button"
                onClick={() => {
                  setMapSwitchPending(false)
                  setS('map_mod', server!.map_mod)
                }}
                className="btn-ghost text-sm"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={confirmMapSwitch}
                disabled={settingsSaving}
                className="px-4 py-2 rounded-lg bg-nova-600 text-white text-sm hover:bg-nova-500 transition-colors disabled:opacity-50"
              >
                {settingsSaving ? 'Applying…' : 'Switch & Restart'}
              </button>
            </div>
          </div>
        </div>
      )}

      <div className="flex items-start justify-between">
        <div className="flex items-start gap-3">
          <Link to="/" className="btn-ghost p-2 mt-0.5">
            <ChevronLeft className="w-4 h-4" />
          </Link>
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-xl font-bold">{server.name}</h1>
              <StatusBadge status={server.status} />
            </div>
            <p className="text-sm text-dark-400 mt-0.5">
              {server.loader} · {server.mc_version} · Port {server.port}
            </p>
          </div>
        </div>

        <div className="flex gap-2">
          {can('servers.power') && isStopped && (
            <button onClick={() => action(() => api.servers.start(server.id))} disabled={actionLoading} className="btn-primary flex items-center gap-2">
              {actionLoading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />}
              Start
            </button>
          )}
          {can('servers.power') && isRunning && (
            <>
              <button onClick={() => action(() => api.servers.restart(server.id))} disabled={actionLoading} className="btn-ghost flex items-center gap-2">
                <RotateCcw className="w-4 h-4" /> Restart
              </button>
              <button onClick={() => action(() => api.servers.stop(server.id))} disabled={actionLoading} className="px-3 py-2 rounded-lg bg-red-500/15 text-red-400 hover:bg-red-500/25 transition-colors flex items-center gap-2 text-sm">
                <Square className="w-4 h-4" /> Stop
              </button>
              <button onClick={() => action(() => api.servers.kill(server.id))} disabled={actionLoading} className="px-3 py-2 rounded-lg bg-red-600 text-white hover:bg-red-500 transition-colors flex items-center gap-2 text-sm">
                <PowerOff className="w-4 h-4" /> Force Kill
              </button>
            </>
          )}
          {can('servers.power') && isTransitioning && (
            <button disabled className="btn-ghost flex items-center gap-2 opacity-50">
              <Loader2 className="w-4 h-4 animate-spin" />
              {server.status === 'starting' ? 'Starting…' : 'Stopping…'}
            </button>
          )}
        </div>
      </div>

      <div className="flex gap-1 border-b border-dark-border overflow-x-auto">
        {tabs.map(t => (
          <button
            key={t.id}
            onClick={() => selectTab(t.id)}
            className={`px-4 py-2 text-sm border-b-2 transition-colors -mb-px whitespace-nowrap ${
              tab === t.id
                ? 'border-nova-500 text-nova-400 font-medium'
                : 'border-transparent text-dark-400 hover:text-white'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'overview' && (
        <OverviewTab
          server={server}
          metrics={metrics}
          sessions={sessions}
          storage={storage}
          worldInfo={worldInfo}
          appConfig={appConfig}
          connectAddress={connectAddress}
        />
      )}

      {tab === 'console' && (
        <div className="h-[calc(100vh-260px)]">
          <ConsolePanel serverId={server.id} serverStatus={server.status} />
        </div>
      )}

      {tab === 'players'    && <PlayersTab serverId={server.id} sessions={sessions} />}
      {tab === 'modrinth'   && <ModrinthTab serverId={server.id} loader={server.loader} mcVersion={server.mc_version} />}
      {tab === 'moderation' && <ModerationTab serverId={server.id} />}
      {tab === 'members'    && <MembersTab serverId={server.id} />}
      {tab === 'files'      && <FilesTab serverId={server.id} serverStatus={server.status} onDirtyChange={setFilesDirty} />}
      {tab === 'logs'       && <LogsTab serverId={server.id} />}
      {tab === 'backups'    && <BackupsTab serverId={server.id} />}
      {tab === 'map'        && <MapTab server={server} shortId={shortId} appConfig={appConfig} />}

      {tab === 'settings' && (
        <SettingsTab
          server={server}
          form={settingsForm}
          worldSettings={worldSettings}
          saving={settingsSaving}
          saved={settingsSaved}
          confirmDelete={confirmDelete}
          onFormChange={setS}
          onSave={saveSettings}
          onConfirmDelete={setConfirmDelete}
          onDelete={deleteServer}
        />
      )}
    </div>
  )
}
