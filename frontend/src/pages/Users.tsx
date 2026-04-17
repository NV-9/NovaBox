import { useState, useEffect, useCallback } from 'react'
import { Users as UsersIcon, Plus, Trash2, Loader2, ShieldCheck, User, ChevronDown, ChevronUp, Save, KeyRound } from 'lucide-react'
import { api } from '@/api/client'
import { useAuth } from '@/context/AuthContext'
import { ALL_PERMISSIONS } from '@/types'
import type { AuthUser } from '@/types'

export default function UsersPage() {
  const { isAdmin, user: me } = useAuth()
  const [users,   setUsers]   = useState<AuthUser[]>([])
  const [loading, setLoading] = useState(true)
  const [expanded, setExpanded] = useState<string | null>(null)

  const [newName, setNewName] = useState('')
  const [newPass, setNewPass] = useState('')
  const [newRole, setNewRole] = useState<'admin' | 'user'>('user')
  const [creating, setCreating] = useState(false)
  const [createError, setCreateError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    try { setUsers(await api.users.list()) } catch {}
    finally { setLoading(false) }
  }, [])

  useEffect(() => { load() }, [load])

  if (!isAdmin) {
    return (
      <div className="p-6">
        <p className="text-dark-400 text-sm">Admin access required.</p>
      </div>
    )
  }

  async function createUser(e: React.FormEvent) {
    e.preventDefault()
    if (!newName.trim() || !newPass) return
    setCreating(true); setCreateError(null)
    try {
      const u = await api.users.create({ username: newName.trim(), password: newPass, role: newRole })
      setUsers(prev => [...prev, u])
      setNewName(''); setNewPass(''); setNewRole('user')
    } catch (err: any) {
      setCreateError(err.message)
    } finally {
      setCreating(false)
    }
  }

  async function deleteUser(id: string) {
    if (!confirm('Delete this user? This cannot be undone.')) return
    await api.users.delete(id)
    setUsers(prev => prev.filter(u => u.id !== id))
  }

  return (
    <div className="p-6 space-y-6 max-w-3xl">
      <div className="flex items-center gap-3">
        <UsersIcon className="w-5 h-5 text-nova-400" />
        <h1 className="text-xl font-bold">Users</h1>
      </div>

      <div className="card space-y-4">
        <p className="text-sm font-semibold flex items-center gap-2">
          <Plus className="w-4 h-4 text-nova-400" /> New User
        </p>
        <form onSubmit={createUser} className="space-y-3">
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs font-medium text-dark-400 mb-1">Username</label>
              <input className="input text-sm" value={newName} onChange={e => setNewName(e.target.value)} placeholder="username" />
            </div>
            <div>
              <label className="block text-xs font-medium text-dark-400 mb-1">Password</label>
              <input type="password" className="input text-sm" value={newPass} onChange={e => setNewPass(e.target.value)} placeholder="min 4 chars" />
            </div>
          </div>
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-3">
              {(['user', 'admin'] as const).map(r => (
                <label key={r} className="flex items-center gap-1.5 text-sm cursor-pointer">
                  <input type="radio" name="role" value={r} checked={newRole === r} onChange={() => setNewRole(r)} />
                  {r === 'admin' ? <ShieldCheck className="w-3.5 h-3.5 text-nova-400" /> : <User className="w-3.5 h-3.5 text-dark-400" />}
                  {r.charAt(0).toUpperCase() + r.slice(1)}
                </label>
              ))}
            </div>
            <button type="submit" disabled={creating || !newName.trim() || !newPass} className="btn-primary text-sm px-3 py-1.5 flex items-center gap-1.5 ml-auto">
              {creating ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Plus className="w-3.5 h-3.5" />}
              Create
            </button>
          </div>
          {createError && (
            <p className="text-xs text-red-400 bg-red-500/10 border border-red-500/20 rounded px-3 py-1.5">{createError}</p>
          )}
        </form>
      </div>

      <div className="card divide-y divide-dark-border p-0 overflow-hidden">
        <div className="px-4 py-3 flex items-center justify-between">
          <p className="text-sm font-semibold">All Users</p>
          <span className="text-xs text-dark-500">{users.length} total</span>
        </div>
        {loading ? (
          <div className="px-4 py-6 flex items-center gap-2 text-dark-500 text-sm">
            <Loader2 className="w-3.5 h-3.5 animate-spin" /> Loading…
          </div>
        ) : users.map(u => (
          <UserRow
            key={u.id}
            user={u}
            isSelf={u.id === me?.id}
            expanded={expanded === u.id}
            onToggle={() => setExpanded(expanded === u.id ? null : u.id)}
            onDelete={() => deleteUser(u.id)}
            onUpdated={updated => setUsers(prev => prev.map(x => x.id === updated.id ? updated : x))}
          />
        ))}
      </div>
    </div>
  )
}

function UserRow({
  user, isSelf, expanded, onToggle, onDelete, onUpdated,
}: {
  user: AuthUser
  isSelf: boolean
  expanded: boolean
  onToggle: () => void
  onDelete: () => void
  onUpdated: (u: AuthUser) => void
}) {
  const [role,        setRole]        = useState(user.role)
  const [permissions, setPermissions] = useState<string[]>(user.permissions)
  const [newPass,     setNewPass]     = useState('')
  const [saving,      setSaving]      = useState(false)
  const [error,       setError]       = useState<string | null>(null)

  useEffect(() => { setRole(user.role); setPermissions(user.permissions) }, [user])

  async function save() {
    setSaving(true); setError(null)
    try {
      const updated = await api.users.update(user.id, {
        role,
        permissions,
        ...(newPass.length >= 4 ? { password: newPass } : {}),
      })
      onUpdated(updated)
      setNewPass('')
    } catch (err: any) {
      setError(err.message)
    } finally {
      setSaving(false)
    }
  }

  function togglePerm(key: string) {
    setPermissions(prev => prev.includes(key) ? prev.filter(p => p !== key) : [...prev, key])
  }

  return (
    <div>
      <div className="px-4 py-3 flex items-center gap-3 hover:bg-dark-800/40 transition-colors">
        <div className={`w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold ${
          user.role === 'admin' ? 'bg-nova-600/20 text-nova-400' : 'bg-dark-700 text-dark-300'
        }`}>
          {user.username[0]?.toUpperCase()}
        </div>
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium truncate">
            {user.username}
            {isSelf && <span className="text-xs text-dark-400 ml-1">(you)</span>}
          </p>
          <p className="text-xs text-dark-500">
            {user.role === 'admin'
              ? 'Admin · full access'
              : `${user.permissions.length} permission${user.permissions.length === 1 ? '' : 's'}`}
          </p>
        </div>
        <button onClick={onToggle} className="btn-ghost p-1.5">
          {expanded ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
        </button>
        {!isSelf && (
          <button onClick={onDelete} className="p-1.5 text-dark-400 hover:text-red-400 transition-colors">
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        )}
      </div>

      {expanded && (
        <div className="px-4 pb-4 space-y-4 border-t border-dark-border bg-dark-900/40">
          <div className="pt-3">
            <p className="text-xs font-semibold text-dark-400 uppercase tracking-wider mb-2">Role</p>
            <div className="flex items-center gap-4">
              {(['user', 'admin'] as const).map(r => (
                <label key={r} className={`flex items-center gap-1.5 text-sm cursor-pointer ${isSelf && r !== role ? 'opacity-40' : ''}`}>
                  <input type="radio" name={`role-${user.id}`} value={r} checked={role === r} onChange={() => !isSelf && setRole(r)} disabled={isSelf} />
                  {r === 'admin' ? <ShieldCheck className="w-3.5 h-3.5 text-nova-400" /> : <User className="w-3.5 h-3.5 text-dark-400" />}
                  {r.charAt(0).toUpperCase() + r.slice(1)}
                </label>
              ))}
            </div>
          </div>

          {role !== 'admin' && (
            <div>
              <p className="text-xs font-semibold text-dark-400 uppercase tracking-wider mb-2">Permissions</p>
              <div className="grid grid-cols-2 gap-1.5">
                {ALL_PERMISSIONS.map(p => (
                  <label key={p.key} className="flex items-start gap-2 text-sm cursor-pointer group">
                    <input
                      type="checkbox"
                      className="mt-0.5 shrink-0"
                      checked={permissions.includes(p.key)}
                      onChange={() => togglePerm(p.key)}
                    />
                    <span>
                      <span className="font-medium">{p.label}</span>
                      <span className="block text-xs text-dark-500">{p.description}</span>
                    </span>
                  </label>
                ))}
              </div>
            </div>
          )}

          <div>
            <p className="text-xs font-semibold text-dark-400 uppercase tracking-wider mb-2 flex items-center gap-1.5">
              <KeyRound className="w-3 h-3" /> Change Password
            </p>
            <input
              type="password"
              className="input text-sm w-64"
              placeholder="New password (min 4 chars)"
              value={newPass}
              onChange={e => setNewPass(e.target.value)}
            />
          </div>

          {error && (
            <p className="text-xs text-red-400 bg-red-500/10 border border-red-500/20 rounded px-3 py-1.5">{error}</p>
          )}

          <button onClick={save} disabled={saving} className="btn-primary flex items-center gap-1.5 text-sm px-3 py-1.5">
            {saving ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Save className="w-3.5 h-3.5" />}
            Save Changes
          </button>
        </div>
      )}
    </div>
  )
}
