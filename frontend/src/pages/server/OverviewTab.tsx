import { Users, MemoryStick, Activity, HardDrive } from 'lucide-react'
import {
  LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid
} from 'recharts'
import { format } from 'date-fns'
import type { Server as ServerType, MetricPoint, PlayerSession, AppConfig, StorageUsage, WorldInfo } from '@/types'

interface Props {
  server:         ServerType
  metrics:        MetricPoint[]
  sessions:       PlayerSession[]
  storage:        StorageUsage | null
  worldInfo:      WorldInfo | null
  appConfig:      AppConfig | null
  connectAddress: string
}

export function OverviewTab({
  server, metrics, sessions, storage, worldInfo, appConfig, connectAddress,
}: Props) {
  const isRunning = server.status === 'running'
  const lastMetric = metrics[metrics.length - 1] ?? null
  const usedMemory = lastMetric?.memory_mb ?? 0
  const tps = lastMetric?.tps ?? null
  const memoryPct = server.memory_mb > 0 ? Math.min(100, (usedMemory / server.memory_mb) * 100) : 0
  const storagePct = storage ? Math.min(100, storage.mb / Math.max(storage.mb + 1, 1) * 100) : 0
  const localHostname = appConfig?.device_hostname?.trim() || window.location.hostname

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 xl:grid-cols-4 gap-4">
        <div className="card">
          <p className="text-xs text-dark-400 mb-1 flex items-center gap-1.5"><Users className="w-3.5 h-3.5" /> Players</p>
          <p className="text-2xl font-bold">{sessions.length} <span className="text-sm text-dark-400">/ {server.max_players}</span></p>
        </div>
        <div className="card">
          <p className="text-xs text-dark-400 mb-1 flex items-center gap-1.5"><MemoryStick className="w-3.5 h-3.5" /> Memory</p>
          <p className="text-2xl font-bold">
            {usedMemory >= 1024 ? `${(usedMemory / 1024).toFixed(1)} GB` : `${usedMemory.toFixed(0)} MB`}
            <span className="text-sm text-dark-400"> / {server.memory_mb >= 1024 ? `${(server.memory_mb / 1024).toFixed(0)} GB` : `${server.memory_mb} MB`}</span>
          </p>
          <div className="mt-2 h-2 rounded-full bg-dark-800 overflow-hidden">
            <div className="h-full bg-emerald-500" style={{ width: `${memoryPct}%` }} />
          </div>
        </div>
        <div className="card">
          <p className="text-xs text-dark-400 mb-1 flex items-center gap-1.5"><Activity className="w-3.5 h-3.5" /> TPS</p>
          <p className="text-2xl font-bold">
            {isRunning && tps !== null ? tps.toFixed(2) : '—'}
          </p>
          {isRunning && tps !== null && (
            <p className={`text-xs mt-1 ${tps >= 19 ? 'text-emerald-400' : tps >= 17 ? 'text-amber-400' : 'text-red-400'}`}>
              {tps >= 19 ? 'Healthy tick rate' : tps >= 17 ? 'Tick rate degraded' : 'Severe tick lag'}
            </p>
          )}
        </div>
        <div className="card">
          <p className="text-xs text-dark-400 mb-1 flex items-center gap-1.5"><HardDrive className="w-3.5 h-3.5" /> Disk Usage</p>
          <p className="text-2xl font-bold">{storage ? (storage.gb >= 1 ? `${storage.gb.toFixed(2)} GB` : `${storage.mb.toFixed(0)} MB`) : '—'}</p>
          <div className="mt-2 h-2 rounded-full bg-dark-800 overflow-hidden">
            <div className="h-full bg-blue-500" style={{ width: `${storagePct}%` }} />
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-2 gap-4">
        <div className="card space-y-2">
          <p className="text-xs text-dark-400 uppercase tracking-wider font-semibold">Server Info</p>
          <div className="grid grid-cols-2 gap-3 text-sm">
            <div>
              <p className="text-dark-500">Minecraft Version</p>
              <p className="font-medium">{server.mc_version}</p>
            </div>
            <div>
              <p className="text-dark-500">Type</p>
              <p className="font-medium">{server.loader}</p>
            </div>
            <div>
              <p className="text-dark-500">Difficulty</p>
              <p className="font-medium">{worldInfo?.difficulty ?? '—'}</p>
            </div>
            <div>
              <p className="text-dark-500">Game Mode</p>
              <p className="font-medium">{worldInfo?.gamemode ?? '—'}</p>
            </div>
            <div>
              <p className="text-dark-500">Simulation Distance</p>
              <p className="font-medium">{worldInfo?.simulation_distance ?? '—'} chunks</p>
            </div>
            <div>
              <p className="text-dark-500">View Distance</p>
              <p className="font-medium">{worldInfo?.view_distance ?? '—'} chunks</p>
            </div>
          </div>
        </div>

        <div className="card space-y-3">
          <p className="text-xs text-dark-400 uppercase tracking-wider font-semibold">Connection Routing</p>
          <div className="space-y-2 text-sm">
            <div className="rounded-lg border border-dark-border p-3">
              <p className="text-dark-500">Local Players</p>
              <p className="font-mono text-sm">{localHostname}:{server.port}</p>
            </div>
            <div className="rounded-lg border border-dark-border p-3">
              <p className="text-dark-500">Internet Players</p>
              <p className="font-mono text-sm">{connectAddress}</p>
            </div>
          </div>
          <p className="text-xs text-dark-500">Use the local address on LAN. Use the routed address when connecting from outside the host.</p>
        </div>
      </div>

      {metrics.length > 0 && (
        <div className="grid grid-cols-1 xl:grid-cols-3 gap-4">
          <div className="card">
            <p className="text-sm font-medium mb-4">CPU % (last 6 hours)</p>
            <ResponsiveContainer width="100%" height={160}>
              <LineChart data={metrics}>
                <CartesianGrid strokeDasharray="3 3" stroke="#1e2233" />
                <XAxis dataKey="timestamp" tickFormatter={(t) => format(new Date(t), 'HH:mm')} stroke="#414b62" tick={{ fontSize: 11, fill: '#647592' }} />
                <YAxis stroke="#414b62" tick={{ fontSize: 11, fill: '#647592' }} unit="%" />
                <Tooltip contentStyle={{ background: '#141720', border: '1px solid #1e2233', borderRadius: 8 }} labelStyle={{ color: '#8494ac' }} formatter={(v: number) => [`${v.toFixed(1)}%`, 'CPU']} />
                <Line type="monotone" dataKey="cpu_percent" stroke="#2855ff" strokeWidth={2} dot={false} />
              </LineChart>
            </ResponsiveContainer>
          </div>
          <div className="card">
            <p className="text-sm font-medium mb-4">Memory MB (last 6 hours)</p>
            <ResponsiveContainer width="100%" height={160}>
              <LineChart data={metrics}>
                <CartesianGrid strokeDasharray="3 3" stroke="#1e2233" />
                <XAxis dataKey="timestamp" tickFormatter={(t) => format(new Date(t), 'HH:mm')} stroke="#414b62" tick={{ fontSize: 11, fill: '#647592' }} />
                <YAxis stroke="#414b62" tick={{ fontSize: 11, fill: '#647592' }} unit=" MB" />
                <Tooltip contentStyle={{ background: '#141720', border: '1px solid #1e2233', borderRadius: 8 }} labelStyle={{ color: '#8494ac' }} formatter={(v: number) => [`${v.toFixed(0)} MB`, 'Memory']} />
                <Line type="monotone" dataKey="memory_mb" stroke="#10b981" strokeWidth={2} dot={false} />
              </LineChart>
            </ResponsiveContainer>
          </div>
          <div className="card">
            <p className="text-sm font-medium mb-4">TPS (last 6 hours)</p>
            <ResponsiveContainer width="100%" height={160}>
              <LineChart data={metrics}>
                <CartesianGrid strokeDasharray="3 3" stroke="#1e2233" />
                <XAxis dataKey="timestamp" tickFormatter={(t) => format(new Date(t), 'HH:mm')} stroke="#414b62" tick={{ fontSize: 11, fill: '#647592' }} />
                <YAxis stroke="#414b62" tick={{ fontSize: 11, fill: '#647592' }} domain={[0, 20]} />
                <Tooltip contentStyle={{ background: '#141720', border: '1px solid #1e2233', borderRadius: 8 }} labelStyle={{ color: '#8494ac' }} formatter={(v: number) => [v.toFixed(2), 'TPS']} />
                <Line type="monotone" dataKey="tps" stroke="#eab308" strokeWidth={2} dot={false} />
              </LineChart>
            </ResponsiveContainer>
          </div>
        </div>
      )}
    </div>
  )
}
