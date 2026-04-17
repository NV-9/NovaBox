import { useEffect, useMemo, useState } from 'react'
import { Search, Package, Loader2, Plus, X } from 'lucide-react'
import { api } from '@/api/client'
import type { ServerLoader } from '@/types'

interface ModHit {
  project_id: string
  slug: string
  title: string
  description: string
  author: string
  categories: string[]
}

interface Props {
  serverId: string
  loader: ServerLoader
  mcVersion: string
}

function defaultLoader(loader: ServerLoader): string {
  switch (loader) {
    case 'FABRIC':
      return 'fabric'
    case 'QUILT':
      return 'quilt'
    case 'FORGE':
      return 'forge'
    case 'NEOFORGE':
      return 'neoforge'
    case 'PAPER':
      return 'paper'
    default:
      return ''
  }
}

export function ModrinthTab({ serverId, loader, mcVersion }: Props) {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<ModHit[]>([])
  const [projects, setProjects] = useState<string[]>([])
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const [searched, setSearched] = useState(false)

  useEffect(() => {
    api.servers.modrinthProjects(serverId)
      .then((data) => setProjects(data.projects ?? []))
      .catch(() => setProjects([]))
  }, [serverId])

  const loaderFilter = useMemo(() => defaultLoader(loader), [loader])

  async function runSearch(e: React.FormEvent) {
    e.preventDefault()
    if (!query.trim()) return
    setLoading(true)
    setSearched(true)
    try {
      const data = await api.modrinth.search(query.trim(), loaderFilter || undefined, mcVersion || undefined)
      setResults(data.hits ?? [])
    } catch {
      setResults([])
    } finally {
      setLoading(false)
    }
  }

  async function save(next: string[]) {
    setSaving(true)
    try {
      const cleaned = Array.from(new Set(next.map((v) => v.trim().toLowerCase()).filter(Boolean)))
      const payload = await api.servers.setModrinthProjects(serverId, cleaned)
      setProjects(payload.projects ?? [])
    } finally {
      setSaving(false)
    }
  }

  function addProject(value: string) {
    if (!value.trim()) return
    save([...projects, value])
  }

  function removeProject(value: string) {
    save(projects.filter((p) => p !== value))
  }

  return (
    <div className="space-y-4 max-w-5xl">
      <div className="card">
        <div className="flex items-center justify-between mb-3">
          <div>
            <p className="text-sm font-medium">Server Modrinth Projects</p>
            <p className="text-xs text-dark-400 mt-1">Projects are persisted per server and applied on next start or restart.</p>
          </div>
          {saving && <Loader2 className="w-4 h-4 animate-spin text-dark-400" />}
        </div>

        {projects.length === 0 ? (
          <p className="text-sm text-dark-500">No custom projects configured yet.</p>
        ) : (
          <div className="flex flex-wrap gap-2">
            {projects.map((project) => (
              <span key={project} className="inline-flex items-center gap-1 px-2 py-1 rounded-md bg-dark-border text-xs">
                {project}
                <button onClick={() => removeProject(project)} className="text-dark-400 hover:text-red-400">
                  <X className="w-3 h-3" />
                </button>
              </span>
            ))}
          </div>
        )}
      </div>

      <div className="card">
        <form onSubmit={runSearch} className="flex gap-2 mb-3">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-dark-500" />
            <input
              className="input pl-9"
              placeholder="Search Modrinth projects..."
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          <button type="submit" disabled={loading} className="btn-primary px-4 flex items-center gap-2">
            {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Search className="w-4 h-4" />} Search
          </button>
        </form>

        {!searched && <p className="text-sm text-dark-500">Search for mods/plugins/datapacks compatible with this server.</p>}

        {searched && !loading && results.length === 0 && (
          <p className="text-sm text-dark-500">No matching projects found.</p>
        )}

        <div className="space-y-3">
          {results.map((mod) => {
            const value = (mod.slug || mod.project_id || '').toLowerCase()
            const exists = projects.includes(value)
            return (
              <div key={mod.project_id} className="rounded-lg border border-dark-border p-3 flex items-start gap-3">
                <div className="w-9 h-9 rounded-md bg-dark-border flex items-center justify-center shrink-0">
                  <Package className="w-4 h-4 text-dark-400" />
                </div>
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium truncate">{mod.title}</p>
                  <p className="text-xs text-dark-500 mt-0.5">{value} · by {mod.author}</p>
                  <p className="text-xs text-dark-400 mt-1 line-clamp-2">{mod.description}</p>
                  <div className="flex gap-1 mt-1.5 flex-wrap">
                    {(mod.categories ?? []).slice(0, 4).map((cat) => (
                      <span key={cat} className="badge badge-gray text-[10px]">{cat}</span>
                    ))}
                  </div>
                </div>
                <button
                  disabled={exists || saving}
                  onClick={() => addProject(value)}
                  className="btn-ghost text-xs px-2 py-1 disabled:opacity-40 flex items-center gap-1"
                >
                  <Plus className="w-3 h-3" /> {exists ? 'Added' : 'Add'}
                </button>
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
}
