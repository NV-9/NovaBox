import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Server, ChevronLeft, Loader2 } from 'lucide-react'
import { api } from '@/api/client'
import type { ServerLoader } from '@/types'

const MAP_MODS = [
  { value: '',        label: 'None',    description: 'No live map' },
  { value: 'BLUEMAP', label: 'BlueMap', description: 'Modern 3D map — all server types (recommended)' },
  { value: 'DYNMAP',  label: 'Dynmap',  description: 'Classic 2D/3D map — Paper / Bukkit only' },
]

const LOADERS: { value: ServerLoader; label: string; description: string }[] = [
  { value: 'VANILLA',  label: 'Vanilla',  description: 'Official Mojang server — pure survival' },
  { value: 'PAPER',    label: 'Paper',    description: 'High-performance Bukkit fork with plugin support' },
  { value: 'FABRIC',   label: 'Fabric',   description: 'Lightweight mod loader, great for client+server mods' },
  { value: 'FORGE',    label: 'Forge',    description: 'Most popular mod loader with the largest ecosystem' },
  { value: 'NEOFORGE', label: 'NeoForge', description: 'Modern Forge fork — recommended for new modpacks' },
  { value: 'QUILT',    label: 'Quilt',    description: 'Fabric-compatible loader with extended APIs' },
]

const VERSIONS = [
  'LATEST',
  '1.21.11', '1.21.10', '1.21.9', '1.21.8', '1.21.7', '1.21.6', '1.21.5',
  '1.21.4', '1.21.3', '1.21.2', '1.21.1', '1.21',
  '1.20.6', '1.20.5', '1.20.4', '1.20.3', '1.20.2', '1.20.1', '1.20',
  '1.19.4', '1.19.3', '1.19.2', '1.19.1', '1.19',
  '1.18.2', '1.18.1', '1.18',
  '1.17.1', '1.17',
  '1.16.5', '1.16.4', '1.16.3', '1.16.2', '1.16.1', '1.16',
  '1.15.2', '1.15.1', '1.15',
  '1.14.4', '1.14.3', '1.14.2', '1.14.1', '1.14',
  '1.13.2', '1.13.1', '1.13',
  '1.12.2', '1.12.1', '1.12',
  '1.11.2', '1.11.1', '1.11',
  '1.10.2', '1.10',
  '1.9.4', '1.9.2', '1.9',
  '1.8.9', '1.8.8', '1.8',
  '1.7.10', '1.7.2',
]

export default function NewServer() {
  const navigate = useNavigate()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const [form, setForm] = useState({
    name: '',
    description: '',
    loader: 'VANILLA' as ServerLoader,
    mc_version: 'LATEST',
    max_players: 20,
    memory_mb: 2048,
  })
  const [mapMod, setMapMod] = useState<string | null>(null)
  const [onlineMode, setOnlineMode] = useState(true)

  function set(key: string, value: string | number) {
    setForm((f) => ({ ...f, [key]: value }))
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault()
    if (!form.name.trim()) { setError('Server name is required'); return }
    setLoading(true)
    setError(null)
    try {
      const server = await api.servers.create({
        ...form,
        port:       25565,
        map_mod:    mapMod,
        online_mode: onlineMode,
      })
      navigate(`/servers/${server.id}`)
    } catch (e: any) {
      setError(e.message)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="p-6 max-w-2xl mx-auto">
      <button onClick={() => navigate(-1)} className="btn-ghost flex items-center gap-1 mb-5 text-sm">
        <ChevronLeft className="w-4 h-4" /> Back
      </button>

      <div className="flex items-center gap-3 mb-6">
        <div className="w-10 h-10 rounded-xl bg-nova-600/15 flex items-center justify-center">
          <Server className="w-5 h-5 text-nova-400" />
        </div>
        <div>
          <h1 className="font-bold text-xl">New Server</h1>
          <p className="text-sm text-dark-400">Configure and deploy a Minecraft server</p>
        </div>
      </div>

      <form onSubmit={submit} className="space-y-6">
        <div className="card space-y-4">
          <h2 className="font-semibold text-sm text-dark-300 uppercase tracking-wider">Basic Info</h2>
          <div>
            <label className="block text-sm font-medium mb-1.5">Server Name *</label>
            <input
              className="input"
              placeholder="My Survival Server"
              value={form.name}
              onChange={(e) => set('name', e.target.value)}
            />
          </div>
          <div>
            <label className="block text-sm font-medium mb-1.5">Description</label>
            <input
              className="input"
              placeholder="Optional description"
              value={form.description}
              onChange={(e) => set('description', e.target.value)}
            />
          </div>
        </div>

        <div className="card space-y-3">
          <h2 className="font-semibold text-sm text-dark-300 uppercase tracking-wider">Server Type</h2>
          <div className="grid grid-cols-2 gap-2">
            {LOADERS.map((l) => (
              <button
                key={l.value}
                type="button"
                onClick={() => set('loader', l.value)}
                className={`text-left p-3 rounded-lg border transition-colors ${
                  form.loader === l.value
                    ? 'border-nova-500 bg-nova-600/10'
                    : 'border-dark-border hover:border-dark-500'
                }`}
              >
                <p className="font-medium text-sm">{l.label}</p>
                <p className="text-xs text-dark-400 mt-0.5 leading-snug">{l.description}</p>
              </button>
            ))}
          </div>
        </div>

        <div className="card space-y-4">
          <h2 className="font-semibold text-sm text-dark-300 uppercase tracking-wider">Configuration</h2>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-1.5">Minecraft Version</label>
              <select
                className="select"
                value={form.mc_version}
                onChange={(e) => set('mc_version', e.target.value)}
              >
                {VERSIONS.map((v) => <option key={v}>{v}</option>)}
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium mb-1.5">Max Players</label>
              <input
                type="number"
                className="input"
                min={1}
                max={200}
                value={form.max_players}
                onChange={(e) => set('max_players', parseInt(e.target.value))}
              />
            </div>
          </div>
          <div>
            <label className="block text-sm font-medium mb-1.5">Memory</label>
            <select
              className="select"
              value={form.memory_mb}
              onChange={(e) => set('memory_mb', parseInt(e.target.value))}
            >
              <option value={512}>512 MB</option>
              <option value={1024}>1 GB</option>
              <option value={2048}>2 GB</option>
              <option value={4096}>4 GB</option>
              <option value={8192}>8 GB</option>
              <option value={12288}>12 GB</option>
              <option value={16384}>16 GB</option>
            </select>
          </div>
        </div>

        <div className="card space-y-3">
          <div>
            <h2 className="font-semibold text-sm text-dark-300 uppercase tracking-wider">Live Map</h2>
            <p className="text-xs text-dark-500 mt-0.5">Automatically installed on first server start. Access via map.<span className="font-mono">&lt;server-id&gt;</span>.domain.</p>
          </div>
          <div className="grid grid-cols-3 gap-2">
            {MAP_MODS.map((m) => (
              <button
                key={m.value}
                type="button"
                onClick={() => setMapMod(m.value || null)}
                className={`text-left p-3 rounded-lg border transition-colors ${
                  (mapMod ?? '') === m.value
                    ? 'border-nova-500 bg-nova-600/10'
                    : 'border-dark-border hover:border-dark-500'
                }`}
              >
                <p className="font-medium text-sm">{m.label}</p>
                <p className="text-xs text-dark-400 mt-0.5 leading-snug">{m.description}</p>
              </button>
            ))}
          </div>
        </div>

        <div className="card space-y-3">
          <div>
            <h2 className="font-semibold text-sm text-dark-300 uppercase tracking-wider">Authentication</h2>
            <p className="text-xs text-dark-500 mt-0.5">
              Online mode requires players to have a legitimate Minecraft account. Disable for LAN or offline play.
            </p>
          </div>
          <div className="grid grid-cols-2 gap-2">
            <button
              type="button"
              onClick={() => setOnlineMode(true)}
              className={`text-left p-3 rounded-lg border transition-colors ${
                onlineMode
                  ? 'border-nova-500 bg-nova-600/10'
                  : 'border-dark-border hover:border-dark-500'
              }`}
            >
              <p className="font-medium text-sm">Online Mode</p>
              <p className="text-xs text-dark-400 mt-0.5 leading-snug">Authenticate via Mojang — recommended</p>
            </button>
            <button
              type="button"
              onClick={() => setOnlineMode(false)}
              className={`text-left p-3 rounded-lg border transition-colors ${
                !onlineMode
                  ? 'border-amber-500 bg-amber-600/10'
                  : 'border-dark-border hover:border-dark-500'
              }`}
            >
              <p className="font-medium text-sm">Offline Mode</p>
              <p className="text-xs text-dark-400 mt-0.5 leading-snug">No account required — LAN / cracked clients</p>
            </button>
          </div>
        </div>

        {error && (
          <p className="text-sm text-red-400 bg-red-500/10 border border-red-500/20 rounded-lg px-4 py-2">
            {error}
          </p>
        )}

        <div className="flex gap-3">
          <button type="button" onClick={() => navigate(-1)} className="btn-ghost flex-1">
            Cancel
          </button>
          <button type="submit" disabled={loading} className="btn-primary flex-1 flex items-center justify-center gap-2">
            {loading && <Loader2 className="w-4 h-4 animate-spin" />}
            {loading ? 'Creating…' : 'Create Server'}
          </button>
        </div>
      </form>
    </div>
  )
}
