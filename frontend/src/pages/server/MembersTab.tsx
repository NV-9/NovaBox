import { useState, useEffect, useCallback } from 'react'
import { Users, Plus, Trash2, Loader2, RefreshCw } from 'lucide-react'
import { api } from '@/api/client'

interface Member {
  user_id:  string
  username: string
  added_at: string
}

interface Props {
  serverId: string
}

export function MembersTab({ serverId }: Props) {
  const [members, setMembers] = useState<Member[]>([])
  const [loading, setLoading] = useState(true)
  const [usernameInput, setUsernameInput] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const data = await api.servers.members(serverId)
      setMembers(data)
    } catch {}
    finally {
      setLoading(false)
    }
  }, [serverId])

  useEffect(() => {
    load()
  }, [load])

  async function addMember(e: React.FormEvent) {
    e.preventDefault()
    const username = usernameInput.trim()
    if (!username) return
    setBusy(true)
    setError(null)
    try {
      await api.servers.addMember(serverId, username)
      setMembers(await api.servers.members(serverId))
      setUsernameInput('')
    } catch (err: any) {
      setError(err.message)
    } finally {
      setBusy(false)
    }
  }

  async function removeMember(userId: string) {
    setBusy(true)
    setError(null)
    try {
      await api.servers.removeMember(serverId, userId)
      setMembers(await api.servers.members(serverId))
    } catch (err: any) {
      setError(err.message)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="card space-y-4 max-w-2xl">
      <div className="flex items-center justify-between">
        <p className="text-sm font-semibold flex items-center gap-2">
          <Users className="w-4 h-4 text-blue-400" /> Server Members
          <span className="text-dark-400 font-normal">({members.length})</span>
        </p>
        <button onClick={load} className="btn-ghost p-1.5" title="Refresh" disabled={loading}>
          <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
        </button>
      </div>

      <p className="text-xs text-dark-400 leading-relaxed">
        Members are users from your NovaBox installation who have been granted access to manage this server.
      </p>

      <form onSubmit={addMember} className="flex gap-2">
        <input
          className="input flex-1 text-sm"
          placeholder="Username…"
          value={usernameInput}
          onChange={e => setUsernameInput(e.target.value)}
          disabled={busy}
        />
        <button
          type="submit"
          disabled={busy || !usernameInput.trim()}
          className="btn-primary flex items-center gap-1.5 text-sm px-3"
        >
          {busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Plus className="w-3.5 h-3.5" />}
          Add
        </button>
      </form>

      {error && (
        <p className="text-xs text-red-400 bg-red-500/10 border border-red-500/20 rounded px-3 py-1.5">{error}</p>
      )}

      {loading ? (
        <p className="text-sm text-dark-500 flex items-center gap-2">
          <Loader2 className="w-3.5 h-3.5 animate-spin" /> Loading…
        </p>
      ) : members.length === 0 ? (
        <p className="text-sm text-dark-500">No members yet. Add users above to grant them access.</p>
      ) : (
        <ul className="space-y-2 max-h-96 overflow-y-auto">
          {members.map((member) => {
            const addedDate = new Date(member.added_at)
            const dateStr = addedDate.toLocaleDateString('en-US', {
              month: 'short',
              day: 'numeric',
              year: addedDate.getFullYear() !== new Date().getFullYear() ? 'numeric' : undefined,
            })
            const timeStr = addedDate.toLocaleTimeString('en-US', {
              hour: '2-digit',
              minute: '2-digit',
              hour12: true,
            })

            return (
              <li
                key={member.user_id}
                className="flex items-center justify-between bg-dark-700 border border-dark-600 rounded px-3 py-2"
              >
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-white truncate">{member.username}</p>
                  <p className="text-xs text-dark-400">Added {dateStr} at {timeStr}</p>
                </div>
                <button
                  onClick={() => removeMember(member.user_id)}
                  disabled={busy}
                  className="btn-danger-ghost p-1.5 ml-2 flex-shrink-0"
                  title="Remove member"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </li>
            )
          })}
        </ul>
      )}
    </div>
  )
}
