import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Zap, Loader2, ShieldCheck } from 'lucide-react'
import { useAuth } from '@/context/AuthContext'

export default function Setup() {
  const { login }      = useAuth()
  const navigate       = useNavigate()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [confirm,  setConfirm]  = useState('')
  const [error,    setError]    = useState<string | null>(null)
  const [loading,  setLoading]  = useState(false)

  async function submit(e: React.FormEvent) {
    e.preventDefault()
    if (password !== confirm) { setError('Passwords do not match'); return }
    if (password.length < 4)  { setError('Password must be at least 4 characters'); return }
    setLoading(true); setError(null)
    try {
      const res = await fetch('/api/auth/setup', {
        method:  'POST',
        headers: { 'Content-Type': 'application/json' },
        body:    JSON.stringify({ username, password }),
      })
      if (!res.ok) {
        const text = await res.text()
        setError(text || 'Setup failed')
        return
      }
      const { token, user } = await res.json()
      login(user, token)
      navigate('/')
    } catch {
      setError('Could not reach the server')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-dark-900 p-4">
      <div className="w-full max-w-sm space-y-6">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-nova-600 flex items-center justify-center">
            <Zap className="w-5 h-5 text-white" />
          </div>
          <div>
            <p className="font-bold text-lg leading-tight">Welcome to NovaBox</p>
            <p className="text-xs text-dark-400">Create the admin account to get started</p>
          </div>
        </div>

        <div className="flex items-start gap-2 text-xs text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 rounded-lg px-3 py-2">
          <ShieldCheck className="w-3.5 h-3.5 mt-0.5 shrink-0" />
          This account will have full admin access. Additional users can be created from Settings.
        </div>

        <form onSubmit={submit} className="card space-y-4">
          <div>
            <label className="block text-sm font-medium mb-1.5">Username</label>
            <input
              className="input"
              autoFocus
              autoComplete="username"
              placeholder="admin"
              value={username}
              onChange={e => setUsername(e.target.value)}
              disabled={loading}
            />
          </div>
          <div>
            <label className="block text-sm font-medium mb-1.5">Password</label>
            <input
              type="password"
              className="input"
              autoComplete="new-password"
              value={password}
              onChange={e => setPassword(e.target.value)}
              disabled={loading}
            />
          </div>
          <div>
            <label className="block text-sm font-medium mb-1.5">Confirm Password</label>
            <input
              type="password"
              className="input"
              autoComplete="new-password"
              value={confirm}
              onChange={e => setConfirm(e.target.value)}
              disabled={loading}
            />
          </div>

          {error && (
            <p className="text-xs text-red-400 bg-red-500/10 border border-red-500/20 rounded px-3 py-1.5">{error}</p>
          )}

          <button type="submit" disabled={loading || !username || !password || !confirm} className="btn-primary w-full flex items-center justify-center gap-2">
            {loading && <Loader2 className="w-4 h-4 animate-spin" />}
            {loading ? 'Creating account…' : 'Create Admin Account'}
          </button>
        </form>
      </div>
    </div>
  )
}
