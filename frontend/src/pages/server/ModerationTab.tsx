import { useState, useEffect, useCallback } from 'react'
import { UserCheck, UserX, Plus, Trash2, Loader2, RefreshCw } from 'lucide-react'
import { api } from '@/api/client'
import { Toggle } from '@/components/Toggle'
import type { WhitelistEntry, BanEntry } from '@/types'

interface Props {
  serverId: string
}

export function ModerationTab({ serverId }: Props) {
  const [whitelist, setWhitelist]     = useState<WhitelistEntry[]>([])
  const [bans,      setBans]          = useState<BanEntry[]>([])
  const [wlLoading, setWlLoading]     = useState(true)
  const [bansLoading, setBansLoading] = useState(true)
  const [wlInput,   setWlInput]       = useState('')
  const [banInput,  setBanInput]      = useState('')
  const [banReason, setBanReason]     = useState('')
  const [wlEnabled, setWlEnabled]     = useState(false)
  const [wlBusy,    setWlBusy]        = useState(false)
  const [banBusy,   setBanBusy]       = useState(false)
  const [wlStateBusy, setWlStateBusy] = useState(false)
  const [wlError,   setWlError]       = useState<string | null>(null)
  const [banError,  setBanError]      = useState<string | null>(null)

  const loadWhitelist = useCallback(async () => {
    setWlLoading(true)
    try { setWhitelist(await api.moderation.whitelist(serverId)) } catch {}
    finally { setWlLoading(false) }
  }, [serverId])

  const loadBans = useCallback(async () => {
    setBansLoading(true)
    try { setBans(await api.moderation.bans(serverId)) } catch {}
    finally { setBansLoading(false) }
  }, [serverId])

  const loadWhitelistState = useCallback(async () => {
    try {
      const state = await api.moderation.whitelistState(serverId)
      setWlEnabled(state.enabled)
    } catch {}
  }, [serverId])

  useEffect(() => { loadWhitelist(); loadWhitelistState(); loadBans() }, [loadWhitelist, loadWhitelistState, loadBans])

  async function setWhitelistEnabled(enabled: boolean) {
    setWlStateBusy(true)
    setWlError(null)
    try {
      const state = await api.moderation.setWhitelistState(serverId, enabled)
      setWlEnabled(state.enabled)
    } catch (err: any) {
      setWlError(err.message)
    } finally {
      setWlStateBusy(false)
    }
  }

  async function addToWhitelist(e: React.FormEvent) {
    e.preventDefault()
    const name = wlInput.trim()
    if (!name) return
    setWlBusy(true); setWlError(null)
    try {
      setWhitelist(await api.moderation.addWhitelist(serverId, name))
      setWlInput('')
    } catch (err: any) {
      setWlError(err.message)
    } finally {
      setWlBusy(false)
    }
  }

  async function removeFromWhitelist(name: string) {
    setWlBusy(true); setWlError(null)
    try {
      setWhitelist(await api.moderation.removeWhitelist(serverId, name))
    } catch (err: any) {
      setWlError(err.message)
    } finally {
      setWlBusy(false)
    }
  }

  async function addBan(e: React.FormEvent) {
    e.preventDefault()
    const name = banInput.trim()
    if (!name) return
    setBanBusy(true); setBanError(null)
    try {
      setBans(await api.moderation.addBan(serverId, name, banReason.trim()))
      setBanInput(''); setBanReason('')
    } catch (err: any) {
      setBanError(err.message)
    } finally {
      setBanBusy(false)
    }
  }

  async function removeBan(name: string) {
    setBanBusy(true); setBanError(null)
    try {
      setBans(await api.moderation.removeBan(serverId, name))
    } catch (err: any) {
      setBanError(err.message)
    } finally {
      setBanBusy(false)
    }
  }

  return (
    <div className="grid grid-cols-1 xl:grid-cols-2 gap-5 max-w-5xl">
      <div className="card space-y-4">
        <div className="flex items-center justify-between">
          <p className="text-sm font-semibold flex items-center gap-2">
            <UserCheck className="w-4 h-4 text-emerald-400" /> Whitelist
            <span className="text-dark-400 font-normal">({whitelist.length})</span>
          </p>
          <div className="flex items-center gap-2">
            <span className="text-xs text-dark-400">Enforce</span>
            <Toggle value={wlEnabled} onChange={setWhitelistEnabled} disabled={wlStateBusy} />
            <button onClick={() => { loadWhitelist(); loadWhitelistState() }} className="btn-ghost p-1.5" title="Refresh">
              <RefreshCw className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>

        <p className="text-xs text-dark-400 leading-relaxed">
          {wlEnabled
            ? 'Whitelist enforcement is ON. Only players listed below can join this server.'
            : 'Whitelist enforcement is OFF. Entries are saved, but anyone can still join until Enforce is turned on.'}
        </p>

        <form onSubmit={addToWhitelist} className="flex gap-2">
          <input
            className="input flex-1 text-sm"
            placeholder="Player name…"
            value={wlInput}
            onChange={e => setWlInput(e.target.value)}
            disabled={wlBusy}
          />
          <button
            type="submit"
            disabled={wlBusy || !wlInput.trim()}
            className="btn-primary flex items-center gap-1.5 text-sm px-3"
          >
            {wlBusy ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Plus className="w-3.5 h-3.5" />}
            Add
          </button>
        </form>

        {wlError && (
          <p className="text-xs text-red-400 bg-red-500/10 border border-red-500/20 rounded px-3 py-1.5">{wlError}</p>
        )}

        {wlLoading ? (
          <p className="text-sm text-dark-500 flex items-center gap-2"><Loader2 className="w-3.5 h-3.5 animate-spin" /> Loading…</p>
        ) : whitelist.length === 0 ? (
          <p className="text-sm text-dark-500">Whitelist is empty — all players can join.</p>
        ) : (
          <ul className="space-y-1.5 max-h-80 overflow-y-auto">
            {whitelist.map((entry) => (
              <li key={entry.name} className="flex items-center gap-3 py-1.5 px-2 rounded-lg hover:bg-dark-800/50 group">
                <img
                  src={`https://crafatar.com/avatars/${entry.uuid || entry.name}?size=20&overlay`}
                  className="w-5 h-5 rounded shrink-0"
                  alt=""
                  onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
                />
                <span className="text-sm font-medium flex-1">{entry.name}</span>
                <button
                  onClick={() => removeFromWhitelist(entry.name)}
                  disabled={wlBusy}
                  className="opacity-0 group-hover:opacity-100 text-red-400 hover:text-red-300 transition-opacity disabled:opacity-20"
                  title="Remove from whitelist"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="card space-y-4">
        <div className="flex items-center justify-between">
          <p className="text-sm font-semibold flex items-center gap-2">
            <UserX className="w-4 h-4 text-red-400" /> Banned Players
            <span className="text-dark-400 font-normal">({bans.length})</span>
          </p>
          <button onClick={loadBans} className="btn-ghost p-1.5" title="Refresh">
            <RefreshCw className="w-3.5 h-3.5" />
          </button>
        </div>

        <form onSubmit={addBan} className="space-y-2">
          <div className="flex gap-2">
            <input
              className="input flex-1 text-sm"
              placeholder="Player name…"
              value={banInput}
              onChange={e => setBanInput(e.target.value)}
              disabled={banBusy}
            />
            <button
              type="submit"
              disabled={banBusy || !banInput.trim()}
              className="px-3 rounded-lg bg-red-500/15 text-red-400 hover:bg-red-500/25 transition-colors flex items-center gap-1.5 text-sm disabled:opacity-40"
            >
              {banBusy ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Plus className="w-3.5 h-3.5" />}
              Ban
            </button>
          </div>
          <input
            className="input text-sm"
            placeholder="Reason (optional)"
            value={banReason}
            onChange={e => setBanReason(e.target.value)}
            disabled={banBusy}
          />
        </form>

        {banError && (
          <p className="text-xs text-red-400 bg-red-500/10 border border-red-500/20 rounded px-3 py-1.5">{banError}</p>
        )}

        {bansLoading ? (
          <p className="text-sm text-dark-500 flex items-center gap-2"><Loader2 className="w-3.5 h-3.5 animate-spin" /> Loading…</p>
        ) : bans.length === 0 ? (
          <p className="text-sm text-dark-500">No players are banned.</p>
        ) : (
          <ul className="space-y-1.5 max-h-80 overflow-y-auto">
            {bans.map((entry) => (
              <li key={entry.name} className="py-2 px-2 rounded-lg hover:bg-dark-800/50 group">
                <div className="flex items-center gap-3">
                  <img
                    src={`https://crafatar.com/avatars/${entry.uuid || entry.name}?size=20&overlay`}
                    className="w-5 h-5 rounded shrink-0"
                    alt=""
                    onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
                  />
                  <span className="text-sm font-medium flex-1">{entry.name}</span>
                  <button
                    onClick={() => removeBan(entry.name)}
                    disabled={banBusy}
                    className="opacity-0 group-hover:opacity-100 text-emerald-400 hover:text-emerald-300 transition-opacity disabled:opacity-20 text-xs flex items-center gap-1"
                    title="Pardon player"
                  >
                    Pardon
                  </button>
                </div>
                {entry.reason && entry.reason !== 'Banned by an operator.' && (
                  <p className="text-xs text-dark-400 mt-0.5 pl-8">{entry.reason}</p>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}
