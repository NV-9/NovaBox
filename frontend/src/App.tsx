import { useState, useEffect } from 'react'
import { Routes, Route, Navigate, useLocation } from 'react-router-dom'
import { Loader2 } from 'lucide-react'
import { TopBar } from '@/components/TopBar'
import { AuthProvider, useAuth } from '@/context/AuthContext'
import Dashboard from '@/pages/Dashboard'
import ServerDetail from '@/pages/ServerDetail'
import NewServer from '@/pages/NewServer'
import Settings from '@/pages/Settings'
import Login from '@/pages/Login'
import Setup from '@/pages/Setup'
import Users from '@/pages/Users'

function AuthGuard({ children }: { children: React.ReactNode }) {
  const { user, loading } = useAuth()

  if (loading) {
    return (
      <div className="min-h-screen bg-dark-900 flex items-center justify-center">
        <Loader2 className="w-6 h-6 animate-spin text-dark-400" />
      </div>
    )
  }
  if (!user) return <Navigate to="/login" replace />
  return <>{children}</>
}

function PublicAuthGuard({ children }: { children: React.ReactNode }) {
  const { user, loading } = useAuth()
  const location = useLocation()
  const [needsSetup, setNeedsSetup] = useState<boolean | null>(null)

  useEffect(() => {
    fetch('/api/auth/setup')
      .then(r => r.ok ? r.json() : Promise.reject())
      .then(d => setNeedsSetup(!!d.needs_setup))
      .catch(() => setNeedsSetup(false))
  }, [])

  if (loading || needsSetup === null) {
    return (
      <div className="min-h-screen bg-dark-900 flex items-center justify-center">
        <Loader2 className="w-6 h-6 animate-spin text-dark-400" />
      </div>
    )
  }

  if (needsSetup) {
    return location.pathname === '/setup' ? <>{children}</> : <Navigate to="/setup" replace />
  }

  if (user) {
    return <Navigate to="/" replace />
  }

  if (location.pathname === '/setup') {
    return <Navigate to="/login" replace />
  }

  return <>{children}</>
}

function AppLayout() {
  return (
    <div className="flex flex-col h-screen overflow-hidden">
      <TopBar />
      <main className="flex-1 overflow-y-auto">
        <Routes>
          <Route path="/"            element={<Dashboard />} />
          <Route path="/servers/new" element={<NewServer />} />
          <Route path="/servers/:id" element={<ServerDetail />} />
          <Route path="/settings"    element={<Settings />} />
          <Route path="/users"       element={<Users />} />
          <Route path="/servers"     element={<Navigate to="/" replace />} />
          <Route path="/analytics"   element={<Navigate to="/" replace />} />
          <Route path="/mods"        element={<Navigate to="/" replace />} />
          <Route path="*"            element={<Navigate to="/" replace />} />
        </Routes>
      </main>
    </div>
  )
}

export default function App() {
  return (
    <AuthProvider>
      <Routes>
        <Route
          path="/login"
          element={
            <PublicAuthGuard>
              <Login />
            </PublicAuthGuard>
          }
        />
        <Route
          path="/setup"
          element={
            <PublicAuthGuard>
              <Setup />
            </PublicAuthGuard>
          }
        />
        <Route path="/*" element={
          <AuthGuard>
            <AppLayout />
          </AuthGuard>
        } />
      </Routes>
    </AuthProvider>
  )
}
