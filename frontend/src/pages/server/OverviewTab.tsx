import { Users, MemoryStick, Activity, Server, Wifi, Trash2, HardDrive } from 'lucide-react'
import {
  LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid
} from 'recharts'
import { format } from 'date-fns'
import type { Server as ServerType, MetricPoint, PlayerSession, AppConfig, StorageUsage } from '@/types'

interface Props {
  server:         ServerType
  metrics:        MetricPoint[]
  sessions:       PlayerSession[]
  storage:        StorageUsage | null
  appConfig:      AppConfig | null
  connectAddress: string
  confirmDelete:  boolean
  onConfirmDelete: (v: boolean) => void
  onDelete:       () => void
}

export function OverviewTab({
  server, metrics, sessions, storage, connectAddress,
  confirmDelete, onConfirmDelete, onDelete,
}: Props) {
  const isRunning = server.status === 'running'
  const lastMetric = metrics[metrics.length - 1] ?? null

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 xl:grid-cols-5 gap-4">
        <div className="card">
          <p className="text-xs text-dark-400 mb-1 flex items-center gap-1.5"><Users className="w-3.5 h-3.5" /> Players</p>
          <p className="text-2xl font-bold">{sessions.length} <span className="text-sm text-dark-400">/ {server.max_players}</span></p>
        </div>
        <div className="card">
          <p className="text-xs text-dark-400 mb-1 flex items-center gap-1.5"><MemoryStick className="w-3.5 h-3.5" /> Memory</p>
          {(() => {
            const used = lastMetric?.memory_mb ?? null
            const max  = server.memory_mb
            if (used !== null) {
              const usedLabel = used >= 1024 ? `${(used / 1024).toFixed(1)} GB` : `${used.toFixed(0)} MB`
              const maxLabel  = max  >= 1024 ? `${(max  / 1024).toFixed(0)} GB` : `${max} MB`
              return <p className="text-2xl font-bold">{usedLabel} <span className="text-sm text-dark-400">/ {maxLabel}</span></p>
            }
            const maxLabel = max >= 1024 ? `${(max / 1024).toFixed(0)} GB` : `${max} MB`
            return <p className="text-2xl font-bold">{maxLabel}</p>
          })()}
        </div>
        <div className="card">
          <p className="text-xs text-dark-400 mb-1 flex items-center gap-1.5"><Activity className="w-3.5 h-3.5" /> TPS</p>
          <p className="text-2xl font-bold">{isRunning ? '20.0' : '—'}</p>
        </div>
        <div className="card">
          <p className="text-xs text-dark-400 mb-1 flex items-center gap-1.5"><Server className="w-3.5 h-3.5" /> Type</p>
          <p className="text-2xl font-bold text-base">{server.loader}</p>
        </div>
        <div className="card">
          <p className="text-xs text-dark-400 mb-1 flex items-center gap-1.5"><HardDrive className="w-3.5 h-3.5" /> Disk Usage</p>
          <p className="text-2xl font-bold">{storage ? (storage.gb >= 1 ? `${storage.gb.toFixed(2)} GB` : `${storage.mb.toFixed(0)} MB`) : '—'}</p>
        </div>
      </div>

      {metrics.length > 0 && (
        <div className="grid grid-cols-1 xl:grid-cols-2 gap-4">
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
        </div>
      )}

      <div className="card">
        <p className="text-xs text-dark-400 mb-1.5 flex items-center gap-1.5">
          <Wifi className="w-3.5 h-3.5" /> Connect Address
        </p>
        <p className="font-mono text-lg font-semibold tracking-wide select-all">{connectAddress}</p>
        <p className="text-xs text-dark-500 mt-0.5">Share this with players to join</p>
      </div>

      <div className="card border-red-500/20">
        <p className="text-sm font-medium text-red-400 mb-3">Danger Zone</p>
        {confirmDelete ? (
          <div className="flex gap-2">
            <p className="text-sm text-dark-400 flex-1">Are you sure? This is irreversible.</p>
            <button onClick={() => onConfirmDelete(false)} className="btn-ghost text-sm">Cancel</button>
            <button onClick={onDelete} className="px-3 py-1.5 rounded-lg bg-red-600 text-white text-sm hover:bg-red-500 transition-colors">Delete</button>
          </div>
        ) : (
          <button onClick={() => onConfirmDelete(true)} className="flex items-center gap-2 text-sm text-red-400 hover:text-red-300 transition-colors">
            <Trash2 className="w-4 h-4" /> Delete Server
          </button>
        )}
      </div>
    </div>
  )
}
