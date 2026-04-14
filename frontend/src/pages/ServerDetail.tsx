import { useParams, Link, useNavigate } from 'react-router-dom'
import { Play, Square, RotateCcw, ChevronLeft, Loader2, PowerOff } from 'lucide-react'
import { useState, useEffect } from 'react'
import { useServer } from '@/hooks/useServers'
import { StatusBadge } from '@/components/StatusBadge'
import { ConsolePanel } from '@/components/ConsolePanel'
import { api } from '@/api/client'
import { OverviewTab }    from './server/OverviewTab'
import { PlayersTab }     from './server/PlayersTab'
import { MapTab }         from './server/MapTab'
import { SettingsTab }    from './server/SettingsTab'
import { ModerationTab }  from './server/ModerationTab'
import type { MetricPoint, PlayerSession, CreateServerRequest, AppConfig, StorageUsage } from '@/types'

type Tab = 'overview' | 'console' | 'players' | 'moderation' | 'map' | 'settings'

export default function ServerDetail() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { server, loading, refresh } = useServer(id!)

  const [metrics,       setMetrics]       = useState<MetricPoint[]>([])
  const [sessions,      setSessions]      = useState<PlayerSession[]>([])
  const [storage,       setStorage]       = useState<StorageUsage | null>(null)
  const [appConfig,     setAppConfig]     = useState<AppConfig | null>(null)
  const [tab,           setTab]           = useState<Tab>('overview')
  const [actionLoading, setActionLoading] = useState(false)
  const [confirmDelete, setConfirmDelete] = useState(false)

  const [settingsForm,  setSettingsForm]  = useState<Partial<CreateServerRequest>>({})
  const [settingsSaving, setSettingsSaving] = useState(false)
  const [settingsSaved,  setSettingsSaved]  = useState(false)

  useEffect(() => {
    if (!server) return
    setSettingsForm({
      name:                server.name,
      description:         server.description,
      max_players:         server.max_players,
      memory_mb:           server.memory_mb,
      online_mode:         server.online_mode,
      auto_start:          server.auto_start,
      auto_start_delay:    server.auto_start_delay,
      crash_detection:     server.crash_detection,
      shutdown_timeout:    server.shutdown_timeout,
      show_on_status_page: server.show_on_status_page,
      min_memory_mb:       undefined,
      jvm_flags:           undefined,
    })
  }, [server?.id])

  useEffect(() => {
    if (!id) return
    api.servers.runtimeOptions(id)
      .then((opts) => {
        setSettingsForm((f) => ({
          ...f,
          min_memory_mb: opts.min_memory_mb ?? undefined,
          jvm_flags: opts.jvm_flags ?? undefined,
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
    navigate('/servers')
  }

  async function saveSettings(e: React.FormEvent) {
    e.preventDefault()
    setSettingsSaving(true)
    try {
      await api.servers.settings(id!, settingsForm)
      await api.servers.setRuntimeOptions(id!, {
        min_memory_mb: settingsForm.min_memory_mb ?? null,
        jvm_flags: settingsForm.jvm_flags?.trim() ? settingsForm.jvm_flags.trim() : null,
      })
      setSettingsSaved(true)
      refresh()
      setTimeout(() => setSettingsSaved(false), 2000)
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
        <Link to="/servers" className="btn-ghost mt-4 inline-flex">Back to Servers</Link>
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
    { id: 'overview',    label: 'Overview' },
    { id: 'console',     label: 'Console' },
    { id: 'players',     label: 'Players' },
    { id: 'moderation',  label: 'Moderation' },
    ...(server.map_mod ? [{ id: 'map' as Tab, label: 'Map' }] : []),
    { id: 'settings',    label: 'Settings' },
  ]

  return (
    <div className="p-6 space-y-5">
      <div className="flex items-start justify-between">
        <div className="flex items-start gap-4">
          <Link to="/servers" className="btn-ghost p-2 mt-0.5">
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
          {isStopped && (
            <button onClick={() => action(() => api.servers.start(server.id))} disabled={actionLoading} className="btn-primary flex items-center gap-2">
              {actionLoading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />}
              Start
            </button>
          )}
          {isRunning && (
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
          {isTransitioning && (
            <button disabled className="btn-ghost flex items-center gap-2 opacity-50">
              <Loader2 className="w-4 h-4 animate-spin" />
              {server.status === 'starting' ? 'Starting…' : 'Stopping…'}
            </button>
          )}
        </div>
      </div>

      <div className="flex gap-1 border-b border-dark-border">
        {tabs.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className={`px-4 py-2 text-sm border-b-2 transition-colors -mb-px ${
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
          appConfig={appConfig}
          connectAddress={connectAddress}
          confirmDelete={confirmDelete}
          onConfirmDelete={setConfirmDelete}
          onDelete={deleteServer}
        />
      )}

      {tab === 'console' && (
        <div className="h-[calc(100vh-260px)]">
          <ConsolePanel serverId={server.id} serverStatus={server.status} />
        </div>
      )}

      {tab === 'players' && <PlayersTab serverId={server.id} sessions={sessions} />}

      {tab === 'moderation' && <ModerationTab serverId={server.id} />}

      {tab === 'map' && (
        <MapTab server={server} shortId={shortId} appConfig={appConfig} />
      )}

      {tab === 'settings' && (
        <SettingsTab
          server={server}
          form={settingsForm}
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
