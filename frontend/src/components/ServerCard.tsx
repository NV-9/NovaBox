import { Link } from 'react-router-dom'
import { Play, Square, RotateCcw, ChevronRight, Cpu, MemoryStick, Users } from 'lucide-react'
import type { Server } from '@/types'
import { StatusBadge } from './StatusBadge'
import { api } from '@/api/client'

interface Props {
  server: Server
  onRefresh: () => void
  velocityEnabled?: boolean
  domain?: string
}

const LOADER_COLOR: Record<string, string> = {
  VANILLA:  'bg-emerald-500/15 text-emerald-400',
  PAPER:    'bg-amber-500/15 text-amber-400',
  FABRIC:   'bg-sky-500/15 text-sky-400',
  FORGE:    'bg-orange-500/15 text-orange-400',
  NEOFORGE: 'bg-purple-500/15 text-purple-400',
  QUILT:    'bg-pink-500/15 text-pink-400',
}

export function ServerCard({ server, onRefresh, velocityEnabled = false, domain = 'localhost' }: Props) {
  const isRunning = server.status === 'running'
  const isStopped = server.status === 'stopped' || server.status === 'error'

  async function start() {
    await api.servers.start(server.id)
    onRefresh()
  }
  async function stop() {
    await api.servers.stop(server.id)
    onRefresh()
  }
  async function restart() {
    await api.servers.restart(server.id)
    onRefresh()
  }

  const loaderCls = LOADER_COLOR[server.loader] ?? 'badge-gray'
  const shortId = server.id.slice(0, 8)
  const displayAddress = velocityEnabled ? `${shortId}.${domain}` : `${window.location.hostname}:${server.port}`

  return (
    <div className="card hover:border-nova-600/40 transition-colors group">
      <div className="flex items-start justify-between mb-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <h3 className="font-semibold truncate">{server.name}</h3>
            <span className={`badge ${loaderCls}`}>{server.loader}</span>
          </div>
          <p className="text-xs text-dark-400 truncate">
            {server.description || `Port ${server.port} · ${server.mc_version}`}
          </p>
        </div>
        <StatusBadge status={server.status} />
      </div>

      <div className="flex gap-4 text-xs text-dark-400 mb-4">
        <span className="flex items-center gap-1">
          <Users className="w-3.5 h-3.5" />
          {server.online_players ?? 0} / {server.max_players}
        </span>
        <span className="flex items-center gap-1">
          <MemoryStick className="w-3.5 h-3.5" />
          {server.memory_mb >= 1024
            ? `${(server.memory_mb / 1024).toFixed(1)} GB`
            : `${server.memory_mb} MB`}
        </span>
        <span className="flex items-center gap-1 font-mono text-xs">
          {displayAddress}
        </span>
      </div>

      <div className="flex items-center gap-2">
        {isStopped && (
          <button onClick={start} className="btn-primary text-xs py-1.5 px-3 flex items-center gap-1.5">
            <Play className="w-3.5 h-3.5" /> Start
          </button>
        )}
        {isRunning && (
          <>
            <button onClick={stop} className="text-xs py-1.5 px-3 rounded-lg bg-red-500/15 text-red-400 hover:bg-red-500/25 transition-colors flex items-center gap-1.5">
              <Square className="w-3.5 h-3.5" /> Stop
            </button>
            <button onClick={restart} className="text-xs py-1.5 px-3 rounded-lg bg-dark-border text-dark-300 hover:text-white transition-colors flex items-center gap-1.5">
              <RotateCcw className="w-3.5 h-3.5" /> Restart
            </button>
          </>
        )}
        <Link
          to={`/servers/${server.id}`}
          className="ml-auto text-xs text-dark-400 hover:text-nova-400 flex items-center gap-1 transition-colors"
        >
          Manage <ChevronRight className="w-3.5 h-3.5" />
        </Link>
      </div>
    </div>
  )
}
