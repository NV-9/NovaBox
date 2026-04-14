import { useEffect, useState } from 'react'
import {
  Settings as SettingsIcon, Server, Shield, HardDrive,
  Globe, Zap, Save, Loader2, RefreshCw, Info
} from 'lucide-react'
import { api } from '@/api/client'
import type { AppConfig } from '@/types'
import { clsx } from 'clsx'

function Toggle({ value, onChange }: { value: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      type="button"
      onClick={() => onChange(!value)}
      className={clsx(
        'relative inline-flex h-5 w-9 shrink-0 rounded-full border-2 border-transparent transition-colors',
        value ? 'bg-nova-500' : 'bg-dark-600'
      )}
    >
      <span className={clsx(
        'inline-block h-4 w-4 rounded-full bg-white shadow transition-transform',
        value ? 'translate-x-4' : 'translate-x-0'
      )} />
    </button>
  )
}

function SettingRow({ label, description, children }: {
  label: string; description?: string; children: React.ReactNode
}) {
  return (
    <div className="flex items-center justify-between gap-6 py-3 border-b border-dark-border last:border-0">
      <div className="min-w-0">
        <p className="text-sm font-medium">{label}</p>
        {description && <p className="text-xs text-dark-400 mt-0.5 leading-snug">{description}</p>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  )
}

export default function Settings() {
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [saved, setSaved] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    api.settings.get()
      .then(setConfig)
      .catch(e => setError(e.message))
      .finally(() => setLoading(false))
  }, [])

  function set<K extends keyof AppConfig>(key: K, value: AppConfig[K]) {
    setConfig(c => c ? { ...c, [key]: value } : c)
  }

  async function save(e: React.FormEvent) {
    e.preventDefault()
    if (!config) return
    setSaving(true)
    setError(null)
    try {
      const updated = await api.settings.update(config)
      setConfig(updated)
      setSaved(true)
      setTimeout(() => setSaved(false), 2500)
    } catch (e: any) {
      setError(e.message)
    } finally {
      setSaving(false)
    }
  }

  if (loading) return (
    <div className="p-6 flex items-center gap-2 text-dark-400">
      <Loader2 className="w-5 h-5 animate-spin" /> Loading settings…
    </div>
  )

  return (
    <form onSubmit={save} className="p-6 space-y-6 max-w-2xl">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold">Settings</h1>
          <p className="text-sm text-dark-400 mt-0.5">Global configuration for NovaBox — saved to <code className="text-xs font-mono">/app/data/novabox.json</code></p>
        </div>
        <button
          type="submit"
          disabled={saving || !config}
          className="btn-primary flex items-center gap-2"
        >
          {saving
            ? <Loader2 className="w-4 h-4 animate-spin" />
            : saved
              ? <span className="text-emerald-300">Saved ✓</span>
              : <><Save className="w-4 h-4" /> Save</>
          }
        </button>
      </div>

      {error && (
        <p className="text-sm text-red-400 bg-red-500/10 border border-red-500/20 rounded-lg px-4 py-2">
          {error}
        </p>
      )}

      <div className="card space-y-1">
        <div className="flex items-center gap-2 mb-2">
          <Globe className="w-4 h-4 text-nova-400" />
          <h2 className="font-medium text-sm">Networking</h2>
        </div>
        <SettingRow
          label="Base Domain"
          description="Used for per-server addresses. Players connect to {short_id}.{domain}:25565 via Velocity. Maps at map.{short_id}.{domain} via Traefik."
        >
          <input
            className="input w-52 text-sm font-mono"
            placeholder="localhost"
            value={config?.domain ?? ''}
            onChange={e => set('domain', e.target.value)}
          />
        </SettingRow>
      </div>

      <div className="card space-y-1">
        <div className="flex items-center gap-2 mb-2">
          <Zap className="w-4 h-4 text-amber-400" />
          <h2 className="font-medium text-sm">Velocity Proxy</h2>
        </div>

        <div className="bg-dark-800/60 rounded-lg p-3 text-xs text-dark-400 mb-2 flex gap-2">
          <Info className="w-3.5 h-3.5 shrink-0 mt-0.5 text-nova-400" />
          <span>
            Velocity routes Minecraft traffic by hostname — players connect to
            <code className="mx-1 font-mono text-dark-300">{'{'}short_id{'}'}.{config?.domain ?? 'localhost'}:25565</code>
            and land on the right server. When enabled, backend servers run in offline mode
            (Velocity handles authentication) and receive a shared forwarding secret.
          </span>
        </div>

        <SettingRow label="Enable Velocity" description="Start routing players through the Velocity proxy container.">
          <Toggle value={config?.velocity_enabled ?? false} onChange={v => set('velocity_enabled', v)} />
        </SettingRow>
        <SettingRow
          label="Velocity Container Name"
          description="Must match container_name in your docker-compose."
        >
          <input
            className="input w-52 text-sm font-mono"
            value={config?.velocity_container ?? ''}
            onChange={e => set('velocity_container', e.target.value)}
          />
        </SettingRow>
        <SettingRow label="Forwarding Secret" description="Shared secret for Velocity modern forwarding. Auto-generated on first run.">
          <input
            className="input w-52 text-sm font-mono"
            value={config?.velocity_secret ?? ''}
            onChange={e => set('velocity_secret', e.target.value)}
          />
        </SettingRow>
      </div>

      <div className="card space-y-1">
        <div className="flex items-center gap-2 mb-2">
          <RefreshCw className="w-4 h-4 text-emerald-400" />
          <h2 className="font-medium text-sm">Traefik</h2>
        </div>

        <div className="bg-dark-800/60 rounded-lg p-3 text-xs text-dark-400 mb-2 flex gap-2">
          <Info className="w-3.5 h-3.5 shrink-0 mt-0.5 text-nova-400" />
          <span>
            When enabled, Minecraft containers receive Traefik labels so map subdomains
            (<code className="mx-1 font-mono text-dark-300">map.{'{'}short_id{'}'}.{config?.domain ?? 'localhost'}</code>)
            are routed automatically — no host port binding required for maps.
          </span>
        </div>

        <SettingRow label="Enable Traefik" description="Add routing labels to Minecraft containers on next start.">
          <Toggle value={config?.traefik_enabled ?? false} onChange={v => set('traefik_enabled', v)} />
        </SettingRow>
      </div>

      <div className="card space-y-4">
        <div className="flex items-center gap-2 mb-1">
          <Server className="w-4 h-4 text-nova-400" />
          <h2 className="font-medium text-sm">Runtime Info</h2>
        </div>
        {[
          ['Config File', '/app/data/novabox.json'],
          ['Velocity TOML', '/app/data/velocity.toml'],
          ['Forwarding Secret', '/app/data/forwarding.secret'],
          ['Docker Network', 'novabox-mc-net (or per DOCKER_NETWORK env)'],
          ['Servers Mount', '/servers (container path)'],
        ].map(([k, v]) => (
          <div key={k} className="flex justify-between text-sm">
            <dt className="text-dark-400">{k}</dt>
            <dd className="font-mono text-xs text-right">{v}</dd>
          </div>
        ))}
      </div>

      <div className="card space-y-4">
        <div className="flex items-center gap-2 mb-1">
          <Shield className="w-4 h-4 text-amber-400" />
          <h2 className="font-medium text-sm">Security</h2>
        </div>
        <div className="bg-amber-500/10 border border-amber-500/20 rounded-lg p-3 text-sm text-amber-400">
          RCON is isolated to the internal Docker network and never exposed to the internet.
          The Velocity forwarding secret is shared only between the proxy and backend containers.
        </div>
      </div>

      <div className="card">
        <h2 className="font-medium text-sm mb-3">About</h2>
        <dl className="space-y-2">
          {[
            ['Version', '0.1.0'],
            ['License', 'MIT — Free & Unlocked'],
            ['Backend', 'Rust (axum + tokio + sqlx)'],
            ['Frontend', 'React 18 + TypeScript + Tailwind'],
            ['Proxy', 'Velocity (itzg/mc-proxy)'],
            ['HTTP Router', 'Traefik v3'],
          ].map(([k, v]) => (
            <div key={k} className="flex justify-between text-sm">
              <dt className="text-dark-400">{k}</dt>
              <dd className="font-mono text-xs">{v}</dd>
            </div>
          ))}
        </dl>
      </div>
    </form>
  )
}
