import { useState, useEffect, useCallback } from 'react'
import { Archive, Plus, Trash2, Download, Loader2, HardDrive, Clock } from 'lucide-react'
import { api } from '@/api/client'
import { formatDistanceToNow } from 'date-fns'
import type { BackupEntry } from '@/types'

function formatBytes(bytes: number): string {
  if (bytes < 1024)        return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

interface Props {
  serverId: string
}

export function BackupsTab({ serverId }: Props) {
  const [backups,   setBackups]   = useState<BackupEntry[]>([])
  const [loading,   setLoading]   = useState(true)
  const [creating,  setCreating]  = useState(false)
  const [deletingId, setDeletingId] = useState<string | null>(null)
  const [error,     setError]     = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    try { setBackups(await api.backups.list(serverId)) } catch (e: any) { setError(e.message) }
    finally { setLoading(false) }
  }, [serverId])

  useEffect(() => { load() }, [load])

  async function createBackup() {
    setCreating(true)
    setError(null)
    try {
      const entry = await api.backups.create(serverId)
      setBackups(prev => [entry, ...prev])
    } catch (e: any) {
      setError(e.message)
    } finally {
      setCreating(false)
    }
  }

  async function deleteBackup(name: string) {
    if (!confirm(`Delete backup "${name}"? This cannot be undone.`)) return
    setDeletingId(name)
    try {
      await api.backups.delete(serverId, name)
      setBackups(prev => prev.filter(b => b.name !== name))
    } catch (e: any) {
      setError(e.message)
    } finally {
      setDeletingId(null)
    }
  }

  return (
    <div className="space-y-4 max-w-2xl">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-semibold">Server Backups</p>
          <p className="text-xs text-dark-500 mt-0.5">
            Full ZIP snapshots of the server directory. Creating a backup may take a moment for large servers.
          </p>
        </div>
        <button
          onClick={createBackup}
          disabled={creating}
          className="btn-primary flex items-center gap-1.5 text-sm px-3 py-1.5 shrink-0"
        >
          {creating
            ? <><Loader2 className="w-3.5 h-3.5 animate-spin" /> Creating…</>
            : <><Plus className="w-3.5 h-3.5" /> Create Backup</>}
        </button>
      </div>

      {error && (
        <p className="text-xs text-red-400 bg-red-500/10 border border-red-500/20 rounded px-3 py-2">
          {error}
        </p>
      )}

      <div className="card p-0 overflow-hidden divide-y divide-dark-border">
        <div className="px-4 py-3 flex items-center justify-between">
          <p className="text-sm font-semibold flex items-center gap-2">
            <Archive className="w-4 h-4 text-nova-400" /> Backups
          </p>
          <span className="text-xs text-dark-500">{backups.length} total</span>
        </div>

        {loading ? (
          <div className="px-4 py-6 flex items-center gap-2 text-dark-500 text-sm">
            <Loader2 className="w-3.5 h-3.5 animate-spin" /> Loading…
          </div>
        ) : backups.length === 0 ? (
          <div className="px-4 py-10 text-center">
            <Archive className="w-8 h-8 text-dark-600 mx-auto mb-2" />
            <p className="text-dark-400 text-sm">No backups yet.</p>
            <p className="text-xs text-dark-600 mt-1">Click "Create Backup" to snapshot this server.</p>
          </div>
        ) : (
          backups.map(b => (
            <div key={b.name} className="px-4 py-3 flex items-center gap-3 hover:bg-dark-800/30 transition-colors">
              <Archive className="w-4 h-4 text-dark-500 shrink-0" />
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium truncate">{b.name}</p>
                <p className="text-xs text-dark-500 flex items-center gap-2 mt-0.5">
                  <HardDrive className="w-3 h-3" />
                  {formatBytes(b.size)}
                  <Clock className="w-3 h-3 ml-1" />
                  {b.created_at
                    ? formatDistanceToNow(new Date(b.created_at * 1000), { addSuffix: true })
                    : '—'}
                </p>
              </div>
              <div className="flex gap-1 shrink-0">
                <button
                  onClick={() => api.backups.download(serverId, b.name)}
                  title="Download"
                  className="p-1.5 text-dark-400 hover:text-nova-400 transition-colors"
                >
                  <Download className="w-3.5 h-3.5" />
                </button>
                <button
                  onClick={() => deleteBackup(b.name)}
                  disabled={deletingId === b.name}
                  title="Delete"
                  className="p-1.5 text-dark-400 hover:text-red-400 transition-colors"
                >
                  {deletingId === b.name
                    ? <Loader2 className="w-3.5 h-3.5 animate-spin" />
                    : <Trash2 className="w-3.5 h-3.5" />}
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
