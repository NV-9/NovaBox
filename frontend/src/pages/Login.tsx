import { useEffect, useState } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { Zap, Loader2 } from 'lucide-react'
import { useAuth } from '@/context/AuthContext'

export default function Login() {
  const { login }      = useAuth()
  const navigate       = useNavigate()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error,    setError]    = useState<string | null>(null)
  const [loading,  setLoading]  = useState(false)

  useEffect(() => {
    fetch('/api/auth/setup')
      .then(r => r.ok ? r.json() : Promise.reject())
      .then((data: { needs_setup?: boolean }) => {
        if (data.needs_setup) {
          navigate('/setup', { replace: true })
        }
      })
      .catch(() => {})
  }, [navigate])

  async function submit(e: React.FormEvent) {
    e.preventDefault()
    setLoading(true); setError(null)
    try {
      const res = await fetch('/api/auth/login', {
        method:  'POST',
        headers: { 'Content-Type': 'application/json' },
        body:    JSON.stringify({ username, password }),
      })
      if (!res.ok) {
        const text = await res.text()
        setError(text || 'Invalid credentials')
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
            <p className="font-bold text-lg leading-tight">NovaBox</p>
            <p className="text-xs text-dark-400">Sign in to continue</p>
          </div>
        </div>

        <form onSubmit={submit} className="card space-y-4">
          <div>
            <label className="block text-sm font-medium mb-1.5">Username</label>
            <input
              className="input"
              autoFocus
              autoComplete="username"
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
              autoComplete="current-password"
              value={password}
              onChange={e => setPassword(e.target.value)}
              disabled={loading}
            />
          </div>

          {error && (
            <p className="text-xs text-red-400 bg-red-500/10 border border-red-500/20 rounded px-3 py-1.5">{error}</p>
          )}

          <button type="submit" disabled={loading || !username || !password} className="btn-primary w-full flex items-center justify-center gap-2">
            {loading && <Loader2 className="w-4 h-4 animate-spin" />}
            {loading ? 'Signing in…' : 'Sign In'}
          </button>

          <p className="text-xs text-dark-400 text-center">
            First time here? <Link to="/setup" className="text-nova-400 hover:text-nova-300">Create admin account</Link>
          </p>
        </form>
      </div>
    </div>
  )
}
