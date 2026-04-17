import { useState, useEffect } from 'react'
import { ExternalLink, RefreshCw, Loader2, MapPin } from 'lucide-react'
import { api } from '@/api/client'
import type { Server, AppConfig } from '@/types'

interface Props {
  server:    Server
  shortId:   string
  appConfig: AppConfig | null
}

type ReadyState = 'checking' | 'ready' | 'not_ready'

export function MapTab({ server, shortId, appConfig }: Props) {
  if (!server.map_mod || server.status !== 'running') return null

  const domain  = appConfig?.domain?.trim() || window.location.hostname
  const mapPort = server.map_mod.toUpperCase() === 'DYNMAP' ? 8123 : 8100
  const mapHost = `map.${shortId}.${domain}`
  const mapUrl  = appConfig?.traefik_enabled
    ? `http://${mapHost}`
    : `http://${window.location.hostname}:${mapPort}`

  const [iframeKey,  setIframeKey]  = useState(0)
  const [readyState, setReadyState] = useState<ReadyState>('checking')

  useEffect(() => {
    if (server.status !== 'running') {
      setReadyState('not_ready')
      return
    }

    let cancelled = false
    let attempts  = 0
    const maxAttempts = 24   // 24 × 5 s = 2 min

    setReadyState('checking')

    async function probe() {
      if (cancelled) return
      try {
        await api.servers.mapConfig(server.id)
        if (!cancelled) setReadyState('ready')
        return
      } catch {
      }
      attempts++
      if (attempts >= maxAttempts) {
        if (!cancelled) setReadyState('ready')
        return
      }
      setTimeout(probe, 5000)
    }

    probe()
    return () => { cancelled = true }
  }, [server.status, server.id, iframeKey])

  function reload() {
    setReadyState('checking')
    setIframeKey(k => k + 1)
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <MapPin className="w-3.5 h-3.5 text-nova-400 shrink-0" />
          <p className="text-sm text-dark-400">
            {appConfig?.traefik_enabled
              ? `${server.map_mod} — ${mapHost}`
              : `${server.map_mod} — port ${mapPort}`}
          </p>
        </div>
        <div className="flex items-center gap-2 ml-4">
          <button
            onClick={reload}
            title="Reload map"
            className="p-1.5 rounded text-dark-500 hover:text-white hover:bg-dark-700 transition-colors"
          >
            <RefreshCw className="w-3.5 h-3.5" />
          </button>
          <a
            href={mapUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="btn-ghost text-sm flex items-center gap-1.5 shrink-0"
          >
            <ExternalLink className="w-3.5 h-3.5" /> Open in new tab
          </a>
        </div>
      </div>

      {server.status !== 'running' && (
        <div className="text-xs text-amber-400 bg-amber-500/10 border border-amber-500/20 rounded-lg px-3 py-2">
          Server is not running — start the server to view the live map.
        </div>
      )}

      <div
        className="rounded-xl overflow-hidden border border-dark-border relative"
        style={{ height: 'calc(100vh - 320px)' }}
      >
        {server.status !== 'running' ? (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 bg-dark-900 text-dark-500">
            <MapPin className="w-8 h-8" />
            <p className="text-sm">Map unavailable — server is offline.</p>
          </div>
        ) : readyState === 'checking' ? (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 bg-dark-900 text-dark-400">
            <Loader2 className="w-6 h-6 animate-spin" />
            <p className="text-sm">Waiting for {server.map_mod} to initialise…</p>
            <p className="text-xs text-dark-500">This can take 1–2 minutes after first start.</p>
          </div>
        ) : (
          <iframe
            key={iframeKey}
            src={mapUrl}
            className="w-full h-full"
            title="Server Map"
            referrerPolicy="no-referrer"
          />
        )}
      </div>
    </div>
  )
}
