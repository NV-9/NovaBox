import { useState, useEffect, useRef } from 'react'
import { Search, Loader2, RefreshCw, FileText } from 'lucide-react'
import { api } from '@/api/client'
import type { LogLine } from '@/types'

function lineClass(text: string): string {
  const t = text.toLowerCase()
  if (t.includes('[error]') || t.includes('[severe]') || t.includes('exception') || t.includes('fatal'))
    return 'text-red-400'
  if (t.includes('[warn]'))
    return 'text-yellow-400'
  if (t.includes('[info]'))
    return 'text-dark-200'
  return 'text-dark-400'
}

interface Props {
  serverId: string
}

export function LogsTab({ serverId }: Props) {
  const [query,   setQuery]   = useState('')
  const [lines,   setLines]   = useState<LogLine[]>([])
  const [loading, setLoading] = useState(true)
  const [error,   setError]   = useState<string | null>(null)
  const bottomRef = useRef<HTMLDivElement>(null)

  async function load(q: string) {
    setLoading(true)
    setError(null)
    try {
      const results = await api.logs.search(serverId, q || undefined, 500)
      setLines(results)
    } catch (e: any) {
      setError(e.message)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { load('') }, [serverId])

  useEffect(() => {
    if (!query) bottomRef.current?.scrollIntoView()
  }, [lines])

  function handleSearch(e: React.FormEvent) {
    e.preventDefault()
    load(query)
  }

  return (
    <div className="space-y-3">
      <form onSubmit={handleSearch} className="flex gap-2">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-dark-500" />
          <input
            className="input pl-9 text-sm w-full"
            placeholder="Search logs… (e.g. error, player name, event)"
            value={query}
            onChange={e => setQuery(e.target.value)}
          />
        </div>
        <button type="submit" disabled={loading} className="btn-primary text-sm px-4 flex items-center gap-1.5">
          {loading ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Search className="w-3.5 h-3.5" />}
          Search
        </button>
        <button
          type="button"
          onClick={() => { setQuery(''); load('') }}
          disabled={loading}
          title="Reload tail"
          className="btn-ghost p-2"
        >
          <RefreshCw className="w-3.5 h-3.5" />
        </button>
      </form>

      {error ? (
        <div className="card py-8 text-center">
          <FileText className="w-8 h-8 text-dark-600 mx-auto mb-2" />
          <p className="text-dark-400 text-sm">{error}</p>
          <p className="text-xs text-dark-600 mt-1">The server may not have generated logs yet.</p>
        </div>
      ) : (
        <div className="card p-0 overflow-hidden">
          <div className="flex items-center justify-between px-3 py-2 border-b border-dark-border">
            <span className="text-xs text-dark-500">
              {query
                ? `${lines.length} match${lines.length === 1 ? '' : 'es'} for "${query}"`
                : `Last ${lines.length} lines`}
            </span>
            <span className="text-xs text-dark-600">latest.log</span>
          </div>
          <div className="overflow-y-auto max-h-[calc(100vh-320px)] font-mono text-xs bg-dark-900 p-3 space-y-0.5">
            {loading ? (
              <div className="flex items-center gap-2 text-dark-500 py-4 justify-center">
                <Loader2 className="w-3.5 h-3.5 animate-spin" /> Loading…
              </div>
            ) : lines.length === 0 ? (
              <p className="text-dark-500 py-4 text-center">No matching lines.</p>
            ) : (
              lines.map(l => (
                <div key={l.line} className="flex gap-3 hover:bg-dark-800/40 px-1 rounded leading-5">
                  <span className="text-dark-600 select-none w-10 text-right shrink-0">{l.line}</span>
                  <span className={lineClass(l.text)}>{l.text}</span>
                </div>
              ))
            )}
            <div ref={bottomRef} />
          </div>
        </div>
      )}
    </div>
  )
}
