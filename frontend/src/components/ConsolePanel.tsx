import { useEffect, useRef, useState } from 'react'
import { Terminal, Wifi, WifiOff, Trash2, Send } from 'lucide-react'
import { useConsole } from '@/hooks/useConsole'
import { api } from '@/api/client'
import { clsx } from 'clsx'

interface Props {
  serverId: string | null
  serverStatus?: string
}

function colorLine(line: string): string {
  if (line.includes('[ERROR]') || line.includes('ERROR')) return 'text-red-400'
  if (line.includes('[WARN]') || line.includes('WARN')) return 'text-amber-400'
  if (line.includes('[INFO]')) return 'text-dark-300'
  if (line.includes('joined the game') || line.includes('left the game')) return 'text-emerald-400'
  if (line.includes('Done') && line.includes('For help')) return 'text-emerald-400 font-medium'
  return 'text-dark-300'
}

export function ConsolePanel({ serverId, serverStatus }: Props) {
  const { lines, connected, clear } = useConsole(serverId)
  const [input, setInput] = useState('')
  const [sending, setSending] = useState(false)
  const scrollRef = useRef<HTMLDivElement>(null)
  const atBottomRef = useRef(true)

  function onScroll() {
    const el = scrollRef.current
    if (!el) return
    const threshold = 40
    atBottomRef.current = el.scrollHeight - el.scrollTop - el.clientHeight <= threshold
  }

  useEffect(() => {
    if (!atBottomRef.current) return
    const el = scrollRef.current
    if (el) el.scrollTop = el.scrollHeight
  }, [lines])

  async function sendCommand(e: React.FormEvent) {
    e.preventDefault()
    if (!input.trim() || !serverId) return
    setSending(true)
    try {
      await api.servers.command(serverId, input.trim())
      setInput('')
    } finally {
      setSending(false)
    }
  }

  const isRunning = serverStatus === 'running'

  return (
    <div className="card flex flex-col h-full min-h-0">
      <div className="flex items-center justify-between mb-3 shrink-0">
        <div className="flex items-center gap-2">
          <Terminal className="w-4 h-4 text-nova-400" />
          <span className="text-sm font-medium">Console</span>
          <span className={clsx('w-2 h-2 rounded-full', connected ? 'bg-emerald-400' : 'bg-dark-500')} />
          <span className="text-xs text-dark-500">{connected ? 'Live' : 'Offline'}</span>
        </div>
        <button onClick={clear} className="btn-ghost text-xs py-1 px-2 flex items-center gap-1">
          <Trash2 className="w-3 h-3" /> Clear
        </button>
      </div>

      <div
        ref={scrollRef}
        onScroll={onScroll}
        className="flex-1 min-h-0 overflow-y-auto bg-dark-950 rounded-lg p-3 font-mono text-xs leading-relaxed"
      >
        {lines.length === 0 ? (
          <p className="text-dark-600 italic">
            {serverId ? 'Waiting for output…' : 'Select a server to view its console.'}
          </p>
        ) : (
          lines.map((line, i) => (
            <div key={i} className={clsx('whitespace-pre-wrap break-all', colorLine(line))}>
              {line}
            </div>
          ))
        )}
      </div>

      <form onSubmit={sendCommand} className="flex gap-2 mt-3 shrink-0">
        <span className="text-nova-400 font-mono text-sm self-center">&gt;</span>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder={isRunning ? 'Enter command…' : 'Server offline'}
          disabled={!isRunning || !serverId}
          className="input font-mono text-xs flex-1 disabled:opacity-40"
        />
        <button
          type="submit"
          disabled={!isRunning || !input.trim() || sending}
          className="btn-primary py-1.5 px-3 disabled:opacity-40"
        >
          <Send className="w-4 h-4" />
        </button>
      </form>
    </div>
  )
}
