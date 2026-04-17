import { Server, Users, BarChart2 } from 'lucide-react'
import { useEffect, useState } from 'react'
import {
  LineChart, Line, BarChart, Bar, XAxis, YAxis, Tooltip,
  ResponsiveContainer, CartesianGrid, Legend,
} from 'recharts'
import { MetricCard } from '@/components/MetricCard'
import { ServerCard } from '@/components/ServerCard'
import { useServers } from '@/hooks/useServers'
import { api } from '@/api/client'
import { Link } from 'react-router-dom'
import { format } from 'date-fns'
import type { AppConfig, MetricPoint, ServerSummary } from '@/types'

export default function Dashboard() {
  const { servers, loading, refresh } = useServers()
  const [appConfig, setAppConfig] = useState<AppConfig | null>(null)

  const [selectedId, setSelectedId] = useState<string>('')
  const [hours,      setHours]      = useState(24)
  const [metrics,    setMetrics]    = useState<MetricPoint[]>([])
  const [summary,    setSummary]    = useState<ServerSummary | null>(null)

  useEffect(() => {
    api.settings.get().then(setAppConfig).catch(() => {})
  }, [])

  useEffect(() => {
    if (!selectedId && servers.length) setSelectedId(servers[0].id)
  }, [servers, selectedId])

  useEffect(() => {
    if (!selectedId) return
    api.metrics.history(selectedId, hours).then(setMetrics).catch(() => {})
    api.metrics.summary(selectedId).then(setSummary).catch(() => {})
  }, [selectedId, hours])

  const running      = servers.filter(s => s.status === 'running').length
  const total        = servers.length
  const totalPlayers = servers.reduce((sum, s) => sum + (s.online_players ?? 0), 0)

  return (
    <div className="p-6 space-y-8">
      <div>
        <h1 className="text-xl font-bold">Dashboard</h1>
        <p className="text-sm text-dark-400 mt-0.5">All systems at a glance.</p>
      </div>

      <div className="grid grid-cols-2 gap-4 max-w-sm">
        <MetricCard
          label="Servers Online"
          value={`${running} / ${total}`}
          sub="active instances"
          icon={<Server className="w-4 h-4" />}
          accent="blue"
        />
        <MetricCard
          label="Players Online"
          value={totalPlayers}
          sub="across all servers"
          icon={<Users className="w-4 h-4" />}
          accent="green"
        />
      </div>

      <div>
        <div className="flex items-center justify-between mb-3">
          <h2 className="font-semibold text-sm text-dark-300 uppercase tracking-wider">Servers</h2>
          <Link to="/servers/new" className="btn-primary text-xs py-1.5 px-3">+ Add Server</Link>
        </div>

        {loading ? (
          <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
            {[...Array(3)].map((_, i) => (
              <div key={i} className="card h-36 animate-pulse bg-dark-border" />
            ))}
          </div>
        ) : servers.length === 0 ? (
          <div className="card text-center py-12">
            <Server className="w-10 h-10 text-dark-600 mx-auto mb-3" />
            <p className="font-medium text-dark-400">No servers yet</p>
            <p className="text-sm text-dark-600 mt-1">Create your first Minecraft server to get started.</p>
            <Link to="/servers/new" className="btn-primary inline-flex mt-4 text-sm">Add Server</Link>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
            {servers.map(server => (
              <ServerCard
                key={server.id}
                server={server}
                onRefresh={refresh}
                velocityEnabled={appConfig?.velocity_enabled ?? false}
                domain={appConfig?.domain ?? 'localhost'}
              />
            ))}
          </div>
        )}
      </div>

      {servers.length > 0 && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="font-semibold text-sm text-dark-300 uppercase tracking-wider flex items-center gap-2">
              <BarChart2 className="w-4 h-4" /> Analytics
            </h2>
            <div className="flex gap-2">
              <select
                className="select text-xs py-1 h-8 w-40"
                value={selectedId}
                onChange={e => setSelectedId(e.target.value)}
              >
                {servers.map(s => (
                  <option key={s.id} value={s.id}>{s.name}</option>
                ))}
              </select>
              <select
                className="select text-xs py-1 h-8 w-24"
                value={hours}
                onChange={e => setHours(parseInt(e.target.value))}
              >
                <option value={1}>1 hour</option>
                <option value={6}>6 hours</option>
                <option value={24}>24 hours</option>
                <option value={168}>7 days</option>
              </select>
            </div>
          </div>

          {summary && (
            <div className="grid grid-cols-3 gap-3">
              <div className="card py-3 px-4">
                <p className="text-xs text-dark-500">Total Sessions</p>
                <p className="text-lg font-bold mt-0.5">{summary.total_sessions}</p>
              </div>
              <div className="card py-3 px-4">
                <p className="text-xs text-dark-500">Unique Players</p>
                <p className="text-lg font-bold mt-0.5">{summary.unique_players}</p>
              </div>
              <div className="card py-3 px-4">
                <p className="text-xs text-dark-500">Peak Concurrent</p>
                <p className="text-lg font-bold mt-0.5">{summary.peak_players}</p>
              </div>
            </div>
          )}

          {metrics.length === 0 ? (
            <div className="card text-center py-10">
              <BarChart2 className="w-8 h-8 text-dark-600 mx-auto mb-2" />
              <p className="text-dark-400 text-sm">No metrics collected yet.</p>
              <p className="text-xs text-dark-600 mt-1">Start a server and data will appear here.</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 xl:grid-cols-2 gap-4">
              <div className="card">
                <p className="text-sm font-medium mb-4">Players Online</p>
                <ResponsiveContainer width="100%" height={180}>
                  <BarChart data={metrics}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#1e2233" />
                    <XAxis
                      dataKey="timestamp"
                      tickFormatter={t => format(new Date(t), 'HH:mm')}
                      stroke="#414b62"
                      tick={{ fontSize: 10, fill: '#647592' }}
                    />
                    <YAxis stroke="#414b62" tick={{ fontSize: 10, fill: '#647592' }} />
                    <Tooltip contentStyle={{ background: '#141720', border: '1px solid #1e2233', borderRadius: 8 }} />
                    <Bar dataKey="online_players" fill="#2855ff" radius={[3, 3, 0, 0]} name="Players" />
                  </BarChart>
                </ResponsiveContainer>
              </div>

              <div className="card">
                <p className="text-sm font-medium mb-4">Resource Usage</p>
                <ResponsiveContainer width="100%" height={180}>
                  <LineChart data={metrics}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#1e2233" />
                    <XAxis
                      dataKey="timestamp"
                      tickFormatter={t => format(new Date(t), 'HH:mm')}
                      stroke="#414b62"
                      tick={{ fontSize: 10, fill: '#647592' }}
                    />
                    <YAxis stroke="#414b62" tick={{ fontSize: 10, fill: '#647592' }} />
                    <Tooltip contentStyle={{ background: '#141720', border: '1px solid #1e2233', borderRadius: 8 }} />
                    <Legend wrapperStyle={{ fontSize: 11 }} />
                    <Line type="monotone" dataKey="cpu_percent" stroke="#2855ff" strokeWidth={2} dot={false} name="CPU %" />
                    <Line type="monotone" dataKey="memory_mb"   stroke="#10b981" strokeWidth={2} dot={false} name="Memory MB" />
                  </LineChart>
                </ResponsiveContainer>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  )
}
