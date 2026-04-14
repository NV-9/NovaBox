import { Server, Users, Activity, Zap } from 'lucide-react'
import { useEffect, useState } from 'react'
import { MetricCard } from '@/components/MetricCard'
import { ServerCard } from '@/components/ServerCard'
import { useServers } from '@/hooks/useServers'
import { api } from '@/api/client'
import { Link } from 'react-router-dom'
import type { AppConfig } from '@/types'

export default function Dashboard() {
  const { servers, loading, refresh } = useServers()
  const [appConfig, setAppConfig] = useState<AppConfig | null>(null)

  useEffect(() => {
    api.settings.get().then(setAppConfig).catch(() => {})
  }, [])

  const running = servers.filter((s) => s.status === 'running').length
  const total = servers.length
  const totalPlayers = servers.reduce((sum, s) => sum + (s.online_players ?? 0), 0)

  return (
    <div className="p-6 space-y-6">
      {/* Title */}
      <div>
        <h1 className="text-xl font-bold">Mission Control</h1>
        <p className="text-sm text-dark-400 mt-0.5">All systems at a glance.</p>
      </div>

      {/* Metric cards */}
      <div className="grid grid-cols-2 xl:grid-cols-4 gap-4">
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
        <MetricCard
          label="Avg TPS"
          value="20.0"
          sub="ticks per second"
          icon={<Activity className="w-4 h-4" />}
          accent="yellow"
        />
        <MetricCard
          label="Uptime"
          value="100%"
          sub="last 24 hours"
          icon={<Zap className="w-4 h-4" />}
          accent="green"
        />
      </div>

      {/* Servers */}
      <div>
        <div className="flex items-center justify-between mb-3">
          <h2 className="font-semibold text-sm text-dark-300 uppercase tracking-wider">Servers</h2>
          <Link to="/servers/new" className="btn-primary text-xs py-1.5 px-3">
            + New Server
          </Link>
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
            <Link to="/servers/new" className="btn-primary inline-flex mt-4 text-sm">
              Create Server
            </Link>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
            {servers.map((server) => (
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
    </div>
  )
}
