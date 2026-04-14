import { Routes, Route } from 'react-router-dom'
import { Sidebar } from '@/components/Sidebar'
import Dashboard from '@/pages/Dashboard'
import Servers from '@/pages/Servers'
import ServerDetail from '@/pages/ServerDetail'
import NewServer from '@/pages/NewServer'
import Players from '@/pages/Players'
import Analytics from '@/pages/Analytics'
import ConsolePage from '@/pages/ConsolePage'
import ModBrowser from '@/pages/ModBrowser'
import Settings from '@/pages/Settings'

export default function App() {
  return (
    <div className="flex h-screen overflow-hidden">
      <Sidebar />
      <main className="flex-1 overflow-y-auto">
        <Routes>
          <Route path="/"             element={<Dashboard />} />
          <Route path="/servers"      element={<Servers />} />
          <Route path="/servers/new"  element={<NewServer />} />
          <Route path="/servers/:id"  element={<ServerDetail />} />
          <Route path="/players"      element={<Players />} />
          <Route path="/analytics"    element={<Analytics />} />
          <Route path="/console"      element={<ConsolePage />} />
          <Route path="/mods"         element={<ModBrowser />} />
          <Route path="/settings"     element={<Settings />} />
        </Routes>
      </main>
    </div>
  )
}
