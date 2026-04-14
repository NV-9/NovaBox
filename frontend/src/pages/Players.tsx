import { useState, useEffect } from 'react'
import { Users, Clock, User } from 'lucide-react'
import { useServers } from '@/hooks/useServers'
import { api } from '@/api/client'
import type { PlayerSession } from '@/types'
import { format, formatDistanceToNow } from 'date-fns'

function durationStr(seconds: number | null) {
  if (!seconds) return '—'
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  if (h > 0) return `${h}h ${m}m`
  return `${m}m`
}

export default function Players() {
  const { servers } = useServers()
  const [selectedId, setSelectedId] = useState('')
  const [sessions, setSessions] = useState<PlayerSession[]>([])
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    if (!selectedId && servers.length) setSelectedId(servers[0].id)
  }, [servers, selectedId])

  useEffect(() => {
    if (!selectedId) return
    setLoading(true)
    api.players.sessions(selectedId, 100).then(setSessions).finally(() => setLoading(false))
  }, [selectedId])

  const online = sessions.filter((s) => !s.left_at)

  return (
    <div className="p-6 space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold">Players</h1>
          <p className="text-sm text-dark-400 mt-0.5">Session history and live presence</p>
        </div>
        <select
          className="select w-44"
          value={selectedId}
          onChange={(e) => setSelectedId(e.target.value)}
        >
          {servers.map((s) => (
            <option key={s.id} value={s.id}>{s.name}</option>
          ))}
        </select>
      </div>

      <div className="card">
        <div className="flex items-center gap-2 mb-3">
          <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
          <p className="text-sm font-medium">Online Now ({online.length})</p>
        </div>
        {online.length === 0 ? (
          <p className="text-sm text-dark-500">No players online.</p>
        ) : (
          <div className="space-y-2">
            {online.map((s) => (
              <div key={s.id} className="flex items-center gap-3 text-sm">
                <img
                  src={`https://crafatar.com/avatars/${s.player_uuid}?size=28&overlay`}
                  className="w-7 h-7 rounded"
                  alt=""
                  onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
                />
                <span className="font-medium">{s.player_name}</span>
                <span className="text-dark-400 text-xs ml-auto flex items-center gap-1">
                  <Clock className="w-3 h-3" />
                  {formatDistanceToNow(new Date(s.joined_at))} ago
                </span>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="card">
        <p className="text-sm font-medium mb-3">Session History</p>
        {loading ? (
          <div className="space-y-2">
            {[...Array(5)].map((_, i) => (
              <div key={i} className="h-8 rounded animate-pulse bg-dark-border" />
            ))}
          </div>
        ) : sessions.length === 0 ? (
          <div className="text-center py-8">
            <Users className="w-10 h-10 text-dark-600 mx-auto mb-2" />
            <p className="text-dark-500 text-sm">No session data recorded yet.</p>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-dark-500 text-xs border-b border-dark-border">
                  <th className="pb-2 font-medium">Player</th>
                  <th className="pb-2 font-medium">Joined</th>
                  <th className="pb-2 font-medium">Left</th>
                  <th className="pb-2 font-medium">Duration</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-dark-border">
                {sessions.map((s) => (
                  <tr key={s.id}>
                    <td className="py-2.5 flex items-center gap-2">
                      <User className="w-3.5 h-3.5 text-dark-500" />
                      <span className="font-medium">{s.player_name}</span>
                    </td>
                    <td className="py-2.5 text-dark-400">{format(new Date(s.joined_at), 'MMM d, HH:mm')}</td>
                    <td className="py-2.5 text-dark-400">
                      {s.left_at ? format(new Date(s.left_at), 'MMM d, HH:mm') : (
                        <span className="badge-green">Online</span>
                      )}
                    </td>
                    <td className="py-2.5 text-dark-400">{durationStr(s.duration_seconds)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  )
}
