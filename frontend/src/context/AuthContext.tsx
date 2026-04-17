import { createContext, useContext, useEffect, useState, type ReactNode } from 'react'
import type { AuthUser } from '@/types'

interface AuthContextValue {
  user:          AuthUser | null
  token:         string | null
  loading:       boolean
  login:         (user: AuthUser, token: string) => void
  logout:        () => void
  can:           (permission: string) => boolean
  isAdmin:       boolean
}

const AuthContext = createContext<AuthContextValue | null>(null)

const TOKEN_KEY = 'novabox_token'

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user,    setUser]    = useState<AuthUser | null>(null)
  const [token,   setToken]   = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const stored = localStorage.getItem(TOKEN_KEY)
    if (!stored) { setLoading(false); return }

    fetch('/api/auth/me', {
      headers: { Authorization: `Bearer ${stored}` },
    })
      .then(r => r.ok ? r.json() : Promise.reject())
      .then((u: AuthUser) => { setUser(u); setToken(stored) })
      .catch(() => localStorage.removeItem(TOKEN_KEY))
      .finally(() => setLoading(false))
  }, [])

  function login(u: AuthUser, t: string) {
    setUser(u)
    setToken(t)
    localStorage.setItem(TOKEN_KEY, t)
  }

  function logout() {
    if (token) {
      fetch('/api/auth/logout', {
        method: 'POST',
        headers: { Authorization: `Bearer ${token}` },
      }).catch(() => {})
    }
    setUser(null)
    setToken(null)
    localStorage.removeItem(TOKEN_KEY)
  }

  function can(permission: string): boolean {
    if (!user) return false
    if (user.role === 'admin') return true
    return user.permissions.includes(permission)
  }

  return (
    <AuthContext.Provider value={{
      user, token, loading, login, logout, can,
      isAdmin: user?.role === 'admin',
    }}>
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext)
  if (!ctx) throw new Error('useAuth must be used inside <AuthProvider>')
  return ctx
}
