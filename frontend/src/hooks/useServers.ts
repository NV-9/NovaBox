import { useState, useEffect, useCallback } from 'react'
import { api } from '@/api/client'
import type { Server } from '@/types'

export function useServers() {
  const [servers, setServers] = useState<Server[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const refreshLiveCounts = useCallback(async (base: Server[]) => {
    const running = base.filter((s) => s.status === 'running')
    if (running.length === 0) {
      return base.map((s) => ({ ...s, online_players: 0 }))
    }

    const results = await Promise.allSettled(
      running.map(async (s) => {
        const online = await api.players.online(s.id)
        return { id: s.id, count: online.length }
      })
    )

    const liveCountById = new Map<string, number>()
    for (const result of results) {
      if (result.status === 'fulfilled') {
        liveCountById.set(result.value.id, result.value.count)
      }
    }

    return base.map((s) => {
      if (s.status !== 'running') {
        return { ...s, online_players: 0 }
      }
      return { ...s, online_players: liveCountById.get(s.id) ?? s.online_players ?? 0 }
    })
  }, [])

  const load = useCallback(async () => {
    try {
      const data = await api.servers.list()
      const withLiveCounts = await refreshLiveCounts(data)
      setServers(withLiveCounts)
      setError(null)
    } catch (e: any) {
      setError(e.message)
    } finally {
      setLoading(false)
    }
  }, [refreshLiveCounts])

  useEffect(() => {
    load()
    const interval = setInterval(load, 10_000)
    return () => clearInterval(interval)
  }, [load])

  return { servers, loading, error, refresh: load }
}

export function useServer(id: string) {
  const [server, setServer] = useState<Server | null>(null)
  const [loading, setLoading] = useState(true)

  const load = useCallback(async () => {
    try {
      const data = await api.servers.get(id)
      setServer(data)
    } catch {
      setServer(null)
    } finally {
      setLoading(false)
    }
  }, [id])

  useEffect(() => {
    load()
    const interval = setInterval(load, 5_000)
    return () => clearInterval(interval)
  }, [load])

  return { server, loading, refresh: load }
}
