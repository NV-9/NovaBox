import { useState, useEffect, useRef, useCallback } from 'react'
import { createConsoleSocket } from '@/api/client'

export function useConsole(serverId: string | null, maxLines = 1000) {
  const [lines, setLines] = useState<string[]>([])
  const [connected, setConnected] = useState(false)
  const wsRef = useRef<WebSocket | null>(null)

  useEffect(() => {
    if (!serverId) return

    setLines([])

    const ws = createConsoleSocket(serverId)
    wsRef.current = ws

    ws.onopen  = () => setConnected(true)
    ws.onclose = () => setConnected(false)
    ws.onmessage = (e) => {
      setLines((prev) => {
        const next = [...prev, e.data as string]
        return next.length > maxLines ? next.slice(next.length - maxLines) : next
      })
    }

    return () => {
      ws.close()
      wsRef.current = null
      setConnected(false)
    }
  }, [serverId, maxLines])

  const clear = useCallback(() => setLines([]), [])

  return { lines, connected, clear }
}
