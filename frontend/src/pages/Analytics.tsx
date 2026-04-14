import { useState, useEffect } from 'react'
import {
  LineChart, Line, BarChart, Bar, XAxis, YAxis, Tooltip,
  ResponsiveContainer, CartesianGrid, Legend
} from 'recharts'
import { BarChart2, Users, Clock, Zap } from 'lucide-react'
import { useServers } from '@/hooks/useServers'
import { api } from '@/api/client'
import { MetricCard } from '@/components/MetricCard'
import type { MetricPoint, ServerSummary } from '@/types'
import { format } from 'date-fns'

export default function Analytics() {
  const { servers } = useServers()
  const [selectedId, setSelectedId] = useState<string>('')
  const [metrics, setMetrics] = useState<MetricPoint[]>([])
  const [summary, setSummary] = useState<ServerSummary | null>(null)
  const [hours, setHours] = useState(24)

  useEffect(() => {
    if (!selectedId && servers.length) setSelectedId(servers[0].id)
  }, [servers, selectedId])

  useEffect(() => {
    if (!selectedId) return
    api.metrics.history(selectedId, hours).then(setMetrics).catch(() => {})
    api.metrics.summary(selectedId).then(setSummary).catch(() => {})
  }, [selectedId, hours])

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold">Analytics</h1>
          <p className="text-sm text-dark-400 mt-0.5">Turbo nerd charts for your servers</p>
        </div>
        <div className="flex gap-2">
          <select
            className="select w-44"
            value={selectedId}
            onChange={(e) => setSelectedId(e.target.value)}
          >
            {servers.map((s) => (
              <option key={s.id} value={s.id}>{s.name}</option>
            ))}
          </select>
          <select
            className="select w-28"
            value={hours}
            onChange={(e) => setHours(parseInt(e.target.value))}
          >
            <option value={1}>1 hour</option>
            <option value={6}>6 hours</option>
            <option value={24}>24 hours</option>
            <option value={168}>7 days</option>
          </select>
        </div>
      </div>

      <div className="grid grid-cols-2 xl:grid-cols-4 gap-4">
        <MetricCard
          label="Total Sessions"
          value={summary?.total_sessions ?? '—'}
          icon={<Clock className="w-4 h-4" />}
          accent="blue"
        />
        <MetricCard
          label="Unique Players"
          value={summary?.unique_players ?? '—'}
          icon={<Users className="w-4 h-4" />}
          accent="green"
        />
        <MetricCard
          label="Peak Concurrent"
          value={summary?.peak_players ?? '—'}
          icon={<Zap className="w-4 h-4" />}
          accent="yellow"
        />
        <MetricCard
          label="Data Points"
          value={metrics.length}
          sub="in selected period"
          icon={<BarChart2 className="w-4 h-4" />}
          accent="blue"
        />
      </div>

      {metrics.length === 0 ? (
        <div className="card text-center py-12">
          <BarChart2 className="w-10 h-10 text-dark-600 mx-auto mb-3" />
          <p className="text-dark-400">No metrics collected yet.</p>
          <p className="text-sm text-dark-600 mt-1">Start a server and data will appear here.</p>
        </div>
      ) : (
        <div className="space-y-4">
          <div className="card">
            <p className="text-sm font-medium mb-4">Players Online</p>
            <ResponsiveContainer width="100%" height={200}>
              <BarChart data={metrics}>
                <CartesianGrid strokeDasharray="3 3" stroke="#1e2233" />
                <XAxis
                  dataKey="timestamp"
                  tickFormatter={(t) => format(new Date(t), 'HH:mm')}
                  stroke="#414b62"
                  tick={{ fontSize: 11, fill: '#647592' }}
                />
                <YAxis stroke="#414b62" tick={{ fontSize: 11, fill: '#647592' }} />
                <Tooltip
                  contentStyle={{ background: '#141720', border: '1px solid #1e2233', borderRadius: 8 }}
                />
                <Bar dataKey="online_players" fill="#2855ff" radius={[4, 4, 0, 0]} name="Players" />
              </BarChart>
            </ResponsiveContainer>
          </div>

          <div className="card">
            <p className="text-sm font-medium mb-4">Resource Usage</p>
            <ResponsiveContainer width="100%" height={200}>
              <LineChart data={metrics}>
                <CartesianGrid strokeDasharray="3 3" stroke="#1e2233" />
                <XAxis
                  dataKey="timestamp"
                  tickFormatter={(t) => format(new Date(t), 'HH:mm')}
                  stroke="#414b62"
                  tick={{ fontSize: 11, fill: '#647592' }}
                />
                <YAxis stroke="#414b62" tick={{ fontSize: 11, fill: '#647592' }} />
                <Tooltip
                  contentStyle={{ background: '#141720', border: '1px solid #1e2233', borderRadius: 8 }}
                />
                <Legend wrapperStyle={{ fontSize: 12 }} />
                <Line type="monotone" dataKey="cpu_percent" stroke="#2855ff" strokeWidth={2} dot={false} name="CPU %" />
                <Line type="monotone" dataKey="memory_mb" stroke="#10b981" strokeWidth={2} dot={false} name="Memory MB" />
              </LineChart>
            </ResponsiveContainer>
          </div>
        </div>
      )}
    </div>
  )
}
