import { Link } from 'react-router-dom'
import { Server, Plus } from 'lucide-react'
import { useEffect, useState } from 'react'
import { useServers } from '@/hooks/useServers'
import { ServerCard } from '@/components/ServerCard'
import { api } from '@/api/client'
import type { AppConfig } from '@/types'

export default function Servers() {
  const { servers, loading, refresh } = useServers()
  const [appConfig, setAppConfig] = useState<AppConfig | null>(null)

  useEffect(() => {
    api.settings.get().then(setAppConfig).catch(() => {})
  }, [])

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold">Servers</h1>
          <p className="text-sm text-dark-400 mt-0.5">{servers.length} server{servers.length !== 1 ? 's' : ''} configured</p>
        </div>
        <Link to="/servers/new" className="btn-primary flex items-center gap-2">
          <Plus className="w-4 h-4" />
          Add Server
        </Link>
      </div>

      {loading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
          {[...Array(3)].map((_, i) => (
            <div key={i} className="card h-36 animate-pulse bg-dark-border" />
          ))}
        </div>
      ) : servers.length === 0 ? (
        <div className="card text-center py-16">
          <Server className="w-12 h-12 text-dark-600 mx-auto mb-4" />
          <p className="font-semibold text-lg mb-1">No servers yet</p>
          <p className="text-sm text-dark-400 mb-6">
            Deploy a new Minecraft server in seconds.
          </p>
            <Link to="/servers/new" className="btn-primary inline-flex items-center gap-2">
            <Plus className="w-4 h-4" /> Add Your First Server
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
  )
}
