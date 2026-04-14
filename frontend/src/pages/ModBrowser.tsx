import { useState } from 'react'
import { Search, Package, Download, Heart, Loader2 } from 'lucide-react'
import { api } from '@/api/client'

interface ModHit {
  project_id: string
  slug: string
  title: string
  description: string
  author: string
  downloads: number
  follows: number
  categories: string[]
  icon_url: string | null
  versions: string[]
  latest_version: string
}

function formatNum(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`
  return String(n)
}

export default function ModBrowser() {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<ModHit[]>([])
  const [total, setTotal] = useState(0)
  const [loading, setLoading] = useState(false)
  const [searched, setSearched] = useState(false)

  async function search(e: React.FormEvent) {
    e.preventDefault()
    if (!query.trim()) return
    setLoading(true)
    setSearched(true)
    try {
      const data = await api.modrinth.search(query.trim())
      setResults(data.hits ?? [])
      setTotal(data.total_hits ?? 0)
    } catch {
      setResults([])
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="p-6 space-y-5">
      <div>
        <h1 className="text-xl font-bold">Mod Browser</h1>
        <p className="text-sm text-dark-400 mt-0.5">Search Modrinth for mods, plugins, and datapacks</p>
      </div>

      <form onSubmit={search} className="flex gap-2">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-dark-500" />
          <input
            className="input pl-9"
            placeholder="Search mods, plugins, datapacks…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <button type="submit" disabled={loading} className="btn-primary flex items-center gap-2 px-5">
          {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Search className="w-4 h-4" />}
          Search
        </button>
      </form>

      {loading && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {[...Array(6)].map((_, i) => (
            <div key={i} className="card h-28 animate-pulse bg-dark-border" />
          ))}
        </div>
      )}

      {!loading && searched && (
        <>
          {total > 0 && (
            <p className="text-sm text-dark-400">{formatNum(total)} results</p>
          )}
          {results.length === 0 ? (
            <div className="card text-center py-12">
              <Package className="w-10 h-10 text-dark-600 mx-auto mb-3" />
              <p className="text-dark-400">No results found.</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {results.map((mod) => (
                <div key={mod.project_id} className="card flex gap-4 hover:border-nova-600/40 transition-colors">
                  {mod.icon_url ? (
                    <img src={mod.icon_url} className="w-12 h-12 rounded-lg shrink-0 object-cover" alt="" />
                  ) : (
                    <div className="w-12 h-12 rounded-lg bg-dark-border flex items-center justify-center shrink-0">
                      <Package className="w-6 h-6 text-dark-500" />
                    </div>
                  )}
                  <div className="min-w-0 flex-1">
                    <div className="flex items-start justify-between gap-2">
                      <p className="font-medium text-sm truncate">{mod.title}</p>
                      <div className="flex items-center gap-2 shrink-0 text-xs text-dark-400">
                        <span className="flex items-center gap-0.5">
                          <Download className="w-3 h-3" /> {formatNum(mod.downloads)}
                        </span>
                        <span className="flex items-center gap-0.5">
                          <Heart className="w-3 h-3" /> {formatNum(mod.follows)}
                        </span>
                      </div>
                    </div>
                    <p className="text-xs text-dark-400 mt-0.5 line-clamp-2 leading-snug">{mod.description}</p>
                    <div className="flex items-center gap-2 mt-1.5">
                      <span className="text-[10px] text-dark-500">by {mod.author}</span>
                      {mod.categories.slice(0, 3).map((cat) => (
                        <span key={cat} className="badge badge-gray text-[10px]">{cat}</span>
                      ))}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </>
      )}

      {!searched && !loading && (
        <div className="card text-center py-16">
          <Package className="w-12 h-12 text-dark-600 mx-auto mb-4" />
          <p className="font-medium text-dark-400">Search Modrinth</p>
          <p className="text-sm text-dark-600 mt-1">
            Browse thousands of mods, plugins, and datapacks from the Modrinth registry.
          </p>
        </div>
      )}
    </div>
  )
}
