import { useState, useEffect, useCallback } from 'react'
import { Loader2, RefreshCw } from 'lucide-react'
import { format, formatDuration, intervalToDuration } from 'date-fns'
import { api } from '@/api/client'
import type { PlayerSession, ServerSummary } from '@/types'

interface Props {
  serverId: string
  sessions: PlayerSession[]
}

function duration(seconds: number | null): string {
  if (!seconds || seconds < 1) return '< 1m'
  const d = intervalToDuration({ start: 0, end: seconds * 1000 })
  return formatDuration(d, { format: ['hours', 'minutes'], zero: false }) || '< 1m'
}

export function PlayersTab({ serverId, sessions }: Props) {
  const [history, setHistory]     = useState<PlayerSession[]>([])
  const [summary, setSummary]     = useState<ServerSummary | null>(null)
  const [loading, setLoading]     = useState(true)
  const [page, setPage]           = useState(0)
  const [hasMore, setHasMore]     = useState(true)
  const PAGE = 25

  const load = useCallback(async (p: number) => {
    setLoading(true)
    try {
      const rows = await api.players.sessions(serverId, PAGE, p * PAGE)
      if (p === 0) {
        setHistory(rows)
      } else {
        setHistory(prev => [...prev, ...rows])
      }
      setHasMore(rows.length === PAGE)
    } catch {}
    finally { setLoading(false) }
  }, [serverId])

  useEffect(() => { setPage(0); load(0) }, [load])

  useEffect(() => {
    api.metrics.summary(serverId).then(setSummary).catch(() => setSummary(null))
  }, [serverId])

  function loadMore() {
    const next = page + 1
    setPage(next)
    load(next)
  }

  return (
    <div className="space-y-4 max-w-3xl">
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <div className="card py-3">
          <p className="text-xs text-dark-400">Online Now</p>
          <p className="text-xl font-bold">{sessions.length}</p>
        </div>
        <div className="card py-3">
          <p className="text-xs text-dark-400">Total Sessions</p>
          <p className="text-xl font-bold">{summary?.total_sessions ?? '—'}</p>
        </div>
        <div className="card py-3">
          <p className="text-xs text-dark-400">Unique Players</p>
          <p className="text-xl font-bold">{summary?.unique_players ?? '—'}</p>
        </div>
        <div className="card py-3">
          <p className="text-xs text-dark-400">Peak Concurrent</p>
          <p className="text-xl font-bold">{summary?.peak_players ?? '—'}</p>
        </div>
      </div>

      <div className="card">
        <div className="flex items-center justify-between mb-3">
          <p className="text-sm font-medium">Online Now</p>
          <span className="text-xs text-dark-400">{sessions.length} player{sessions.length !== 1 ? 's' : ''}</span>
        </div>
        {sessions.length === 0 ? (
          <p className="text-sm text-dark-500">No players online.</p>
        ) : (
          <ul className="space-y-2">
            {sessions.map((s) => (
              <li key={s.id} className="flex items-center gap-3 text-sm">
                <img
                  src={`https://crafatar.com/avatars/${s.player_uuid}?size=24&overlay`}
                  className="w-6 h-6 rounded shrink-0"
                  alt=""
                  onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
                />
                <span className="font-medium">{s.player_name}</span>
                <span className="text-dark-400 text-xs ml-auto">since {format(new Date(s.joined_at), 'HH:mm')}</span>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="card">
        <div className="flex items-center justify-between mb-3">
          <p className="text-sm font-medium">Session History</p>
          <button
            onClick={() => { setPage(0); load(0) }}
            className="btn-ghost p-1.5"
            title="Refresh"
          >
            <RefreshCw className="w-3.5 h-3.5" />
          </button>
        </div>

        {loading && history.length === 0 ? (
          <p className="text-sm text-dark-500 flex items-center gap-2">
            <Loader2 className="w-3.5 h-3.5 animate-spin" /> Loading…
          </p>
        ) : history.length === 0 ? (
          <p className="text-sm text-dark-500">No session history yet.</p>
        ) : (
          <>
            <div className="divide-y divide-dark-border">
              {history.map((s) => (
                <div key={s.id} className="flex items-center gap-3 py-2.5">
                  <img
                    src={`https://crafatar.com/avatars/${s.player_uuid}?size=20&overlay`}
                    className="w-5 h-5 rounded shrink-0"
                    alt=""
                    onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
                  />
                  <span className="text-sm font-medium w-36 truncate">{s.player_name}</span>
                  <span className="text-xs text-dark-400 flex-1">
                    {format(new Date(s.joined_at), 'MMM d, HH:mm')}
                    {s.left_at && ` → ${format(new Date(s.left_at), 'HH:mm')}`}
                  </span>
                  <span className="text-xs text-dark-400 text-right shrink-0 w-16">
                    {s.left_at ? duration(s.duration_seconds) : <span className="text-emerald-400">online</span>}
                  </span>
                </div>
              ))}
            </div>

            {hasMore && (
              <button
                onClick={loadMore}
                disabled={loading}
                className="mt-3 w-full btn-ghost text-sm flex items-center justify-center gap-2"
              >
                {loading ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : null}
                Load more
              </button>
            )}
          </>
        )}
      </div>
    </div>
  )
}
