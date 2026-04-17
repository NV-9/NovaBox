import { useState, useEffect, useCallback } from 'react'
import { ShieldCheck, Plus, Trash2, Loader2, RefreshCw } from 'lucide-react'
import { api } from '@/api/client'
import type { OpEntry } from '@/types'

interface Props {
  serverId: string
}

export function OpsTab({ serverId }: Props) {
  const [ops,     setOps]     = useState<OpEntry[]>([])
  const [loading, setLoading] = useState(true)
  const [busy,    setBusy]    = useState(false)
  const [input,   setInput]   = useState('')
  const [error,   setError]   = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    try { setOps(await api.ops.list(serverId)) } catch {}
    finally { setLoading(false) }
  }, [serverId])

  useEffect(() => { load() }, [load])

  async function addOp(e: React.FormEvent) {
    e.preventDefault()
    const name = input.trim()
    if (!name) return
    setBusy(true); setError(null)
    try {
      setOps(await api.ops.add(serverId, name))
      setInput('')
    } catch (err: any) {
      setError(err.message)
    } finally {
      setBusy(false)
    }
  }

  async function removeOp(name: string) {
    setBusy(true); setError(null)
    try {
      setOps(await api.ops.remove(serverId, name))
    } catch (err: any) {
      setError(err.message)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="max-w-lg space-y-4">
      <div className="card space-y-4">
        <div className="flex items-center justify-between">
          <p className="text-sm font-semibold flex items-center gap-2">
            <ShieldCheck className="w-4 h-4 text-nova-400" /> Server Operators
            <span className="text-dark-400 font-normal">({ops.length})</span>
          </p>
          <button onClick={load} className="btn-ghost p-1.5" title="Refresh">
            <RefreshCw className="w-3.5 h-3.5" />
          </button>
        </div>

        <p className="text-xs text-dark-400 leading-relaxed">
          Ops have full in-game admin permissions (level 4). Changes are written to <span className="font-mono">ops.json</span> and synced to a running server via RCON.
        </p>

        <form onSubmit={addOp} className="flex gap-2">
          <input
            className="input flex-1 text-sm"
            placeholder="Player name…"
            value={input}
            onChange={e => setInput(e.target.value)}
            disabled={busy}
          />
          <button
            type="submit"
            disabled={busy || !input.trim()}
            className="btn-primary flex items-center gap-1.5 text-sm px-3"
          >
            {busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Plus className="w-3.5 h-3.5" />}
            Op
          </button>
        </form>

        {error && (
          <p className="text-xs text-red-400 bg-red-500/10 border border-red-500/20 rounded px-3 py-1.5">{error}</p>
        )}

        {loading ? (
          <p className="text-sm text-dark-500 flex items-center gap-2"><Loader2 className="w-3.5 h-3.5 animate-spin" /> Loading…</p>
        ) : ops.length === 0 ? (
          <p className="text-sm text-dark-500">No operators configured.</p>
        ) : (
          <ul className="space-y-1.5 max-h-96 overflow-y-auto">
            {ops.map((entry) => (
              <li key={entry.name} className="flex items-center gap-3 py-1.5 px-2 rounded-lg hover:bg-dark-800/50 group">
                <img
                  src={`https://crafatar.com/avatars/${entry.uuid || entry.name}?size=20&overlay`}
                  className="w-5 h-5 rounded shrink-0"
                  alt=""
                  onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
                />
                <span className="text-sm font-medium flex-1">{entry.name}</span>
                <span className="text-xs text-dark-500 mr-1">level {entry.level}</span>
                <button
                  onClick={() => removeOp(entry.name)}
                  disabled={busy}
                  className="opacity-0 group-hover:opacity-100 text-red-400 hover:text-red-300 transition-opacity disabled:opacity-20"
                  title="Deop player"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}
