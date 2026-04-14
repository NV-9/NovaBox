import { ExternalLink } from 'lucide-react'
import type { Server, AppConfig } from '@/types'

interface Props {
  server:    Server
  shortId:   string
  appConfig: AppConfig | null
}

export function MapTab({ server, shortId, appConfig }: Props) {
  if (!server.map_mod) return null

  const domain  = appConfig?.domain?.trim() || window.location.hostname
  const mapPort = server.map_mod.toUpperCase() === 'DYNMAP' ? 8123 : 8100
  const mapHost = `map.${shortId}.${domain}`
  const mapUrl  = appConfig?.traefik_enabled
    ? `http://${mapHost}`
    : `http://${window.location.hostname}:${mapPort}`

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <p className="text-sm text-dark-400">
          {appConfig?.traefik_enabled
            ? `${server.map_mod} live map via ${mapHost}.`
            : `${server.map_mod} live map on port ${mapPort}.`}
        </p>
        <a
          href={mapUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="btn-ghost text-sm flex items-center gap-1.5 shrink-0 ml-4"
        >
          <ExternalLink className="w-3.5 h-3.5" /> Open in new tab
        </a>
      </div>
      <div className="rounded-xl overflow-hidden border border-dark-border" style={{ height: 'calc(100vh - 320px)' }}>
        <iframe src={mapUrl} className="w-full h-full" title="Server Map" />
      </div>
    </div>
  )
}
