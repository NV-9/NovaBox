import { useState, useEffect } from 'react'
import { useServers } from '@/hooks/useServers'
import { ConsolePanel } from '@/components/ConsolePanel'

export default function ConsolePage() {
  const { servers } = useServers()
  const [selectedId, setSelectedId] = useState('')

  useEffect(() => {
    if (!selectedId && servers.length) setSelectedId(servers[0].id)
  }, [servers, selectedId])

  const server = servers.find((s) => s.id === selectedId)

  return (
    <div className="p-6 flex flex-col h-[calc(100vh-0px)] gap-4">
      <div className="flex items-center justify-between shrink-0">
        <div>
          <h1 className="text-xl font-bold">Console</h1>
          <p className="text-sm text-dark-400 mt-0.5">Live server output &amp; command runner</p>
        </div>
        <select
          className="select w-48"
          value={selectedId}
          onChange={(e) => setSelectedId(e.target.value)}
        >
          {servers.map((s) => (
            <option key={s.id} value={s.id}>{s.name}</option>
          ))}
        </select>
      </div>
      <div className="flex-1 min-h-0">
        <ConsolePanel serverId={selectedId || null} serverStatus={server?.status} />
      </div>
    </div>
  )
}
