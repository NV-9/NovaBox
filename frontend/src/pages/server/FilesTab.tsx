import { useState, useEffect, useCallback, useRef } from 'react'
import {
  Folder, FileText, ChevronRight, ArrowLeft, Download, Trash2,
  Upload, RefreshCw, Loader2, Save, X, Globe
} from 'lucide-react'
import Editor from '@monaco-editor/react'
import { api } from '@/api/client'
import type { FileEntry, WorldEntry } from '@/types'

const TEXT_EXTENSIONS = new Set([
  'txt', 'md', 'log', 'csv',
  'json', 'json5',
  'yml', 'yaml',
  'toml',
  'properties',
  'cfg', 'conf', 'config', 'ini',
  'xml', 'html', 'htm',
  'css', 'js', 'ts', 'jsx', 'tsx',
  'sh', 'bash', 'env',
  'mcmeta', 'mcfunction', 'nbt',
])

const EXT_COLORS: Record<string, string> = {
  json:       'text-yellow-400',
  json5:      'text-yellow-400',
  yml:        'text-emerald-400',
  yaml:       'text-emerald-400',
  toml:       'text-orange-400',
  properties: 'text-sky-400',
  log:        'text-dark-400',
  xml:        'text-purple-400',
  mcmeta:     'text-nova-400',
  mcfunction: 'text-nova-400',
  md:         'text-dark-300',
}

function fileExt(name: string): string {
  const i = name.lastIndexOf('.')
  return i === -1 ? '' : name.slice(i + 1).toLowerCase()
}

function isTextFile(name: string): boolean {
  return TEXT_EXTENSIONS.has(fileExt(name))
}

function editorLanguage(name: string): string {
  const ext = fileExt(name)
  switch (ext) {
    case 'json':
    case 'json5':
      return 'json'
    case 'yml':
    case 'yaml':
      return 'yaml'
    case 'toml':
      return 'ini'
    case 'properties':
    case 'cfg':
    case 'conf':
    case 'config':
    case 'ini':
      return 'ini'
    case 'xml':
      return 'xml'
    case 'html':
    case 'htm':
      return 'html'
    case 'css':
      return 'css'
    case 'js':
      return 'javascript'
    case 'ts':
      return 'typescript'
    case 'jsx':
      return 'javascript'
    case 'tsx':
      return 'typescript'
    case 'sh':
    case 'bash':
      return 'shell'
    case 'md':
      return 'markdown'
    default:
      return 'plaintext'
  }
}

interface Props {
  serverId: string
  serverStatus: string
  onDirtyChange?: (dirty: boolean) => void
}

export function FilesTab({ serverId, serverStatus, onDirtyChange }: Props) {
  const [path,       setPath]       = useState('/')
  const [entries,    setEntries]    = useState<FileEntry[]>([])
  const [loading,    setLoading]    = useState(true)
  const [error,      setError]      = useState<string | null>(null)

  const [editing,    setEditing]    = useState<string | null>(null)
  const [content,    setContent]    = useState('')
  const [originalContent, setOriginalContent] = useState('')
  const [saving,     setSaving]     = useState(false)

  const [worlds,     setWorlds]     = useState<WorldEntry[]>([])
  const [worldsLoading, setWorldsLoading] = useState(true)
  const [worldBusy,  setWorldBusy]  = useState<string | null>(null)

  const fileInput = useRef<HTMLInputElement>(null)
  const hasUnsavedChanges = editing !== null && content !== originalContent

  useEffect(() => {
    onDirtyChange?.(hasUnsavedChanges)
  }, [hasUnsavedChanges, onDirtyChange])

  useEffect(() => {
    return () => onDirtyChange?.(false)
  }, [onDirtyChange])

  const confirmDiscardUnsaved = useCallback(() => {
    if (!hasUnsavedChanges) return true
    return confirm('You have unsaved file changes. Discard them?')
  }, [hasUnsavedChanges])

  const loadDir = useCallback(async (p: string) => {
    setLoading(true); setError(null)
    try {
      setEntries(await api.files.list(serverId, p))
      setPath(p)
    } catch (err: any) {
      setError(err.message)
    } finally {
      setLoading(false)
    }
  }, [serverId])

  const openDir = useCallback(async (p: string) => {
    if (!confirmDiscardUnsaved()) return
    setEditing(null)
    setContent('')
    setOriginalContent('')
    await loadDir(p)
  }, [confirmDiscardUnsaved, loadDir])

  const loadWorlds = useCallback(async () => {
    setWorldsLoading(true)
    try { setWorlds(await api.files.worlds(serverId)) } catch {}
    finally { setWorldsLoading(false) }
  }, [serverId])

  useEffect(() => { loadDir('/'); loadWorlds() }, [loadDir, loadWorlds])

  useEffect(() => {
    const onBeforeUnload = (e: BeforeUnloadEvent) => {
      if (!hasUnsavedChanges) return
      e.preventDefault()
      e.returnValue = ''
    }
    window.addEventListener('beforeunload', onBeforeUnload)
    return () => window.removeEventListener('beforeunload', onBeforeUnload)
  }, [hasUnsavedChanges])

  async function openFile(entry: FileEntry) {
    if (entry.is_dir) { await openDir(entry.path); return }
    if (!isTextFile(entry.name) || entry.size > 1_000_000) {
      api.files.download(serverId, entry.path)
      return
    }
    if (editing && editing !== entry.path && !confirmDiscardUnsaved()) return
    setEditing(entry.path)
    setSaving(false)
    try {
      const loaded = await api.files.content(serverId, entry.path)
      setContent(loaded)
      setOriginalContent(loaded)
    } catch {
      setContent('')
      setOriginalContent('')
    }
  }

  async function saveFile() {
    if (!editing) return
    setSaving(true)
    try {
      await api.files.saveContent(serverId, editing, content)
      setOriginalContent(content)
    } finally {
      setSaving(false)
    }
  }

  async function deleteEntry(entry: FileEntry, e: React.MouseEvent) {
    e.stopPropagation()
    if (!confirm(`Delete ${entry.name}?`)) return
    await api.files.delete(serverId, entry.path)
    loadDir(path)
  }

  async function uploadFiles(e: React.ChangeEvent<HTMLInputElement>) {
    const files = e.target.files
    if (!files || files.length === 0) return
    await api.files.upload(serverId, path, files)
    await loadDir(path)
    e.target.value = ''
  }

  async function deleteWorld(name: string) {
    if (!confirm(`Delete world "${name}"? This cannot be undone.`)) return
    setWorldBusy(name)
    try {
      await api.files.deleteWorld(serverId, name)
      loadWorlds()
    } finally {
      setWorldBusy(null)
    }
  }

  function goUp() {
    const parts = path.replace(/\/$/, '').split('/')
    parts.pop()
    openDir(parts.join('/') || '/')
  }

  function formatSize(bytes: number) {
    if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(1)} GB`
    if (bytes >= 1_048_576)     return `${(bytes / 1_048_576).toFixed(1)} MB`
    if (bytes >= 1024)          return `${(bytes / 1024).toFixed(0)} KB`
    return `${bytes} B`
  }

  const isStopped = serverStatus === 'stopped' || serverStatus === 'error'

  return (
    <div className="space-y-4">

      <div className="card">
        <div className="flex items-center gap-3 flex-wrap">
          <p className="text-sm font-semibold flex items-center gap-2 shrink-0">
            <Globe className="w-4 h-4 text-emerald-400" /> Worlds
          </p>
          {!isStopped && (
            <span className="text-xs text-amber-400 bg-amber-500/10 border border-amber-500/20 rounded px-2 py-0.5">
              Stop server to delete
            </span>
          )}
          <div className="flex items-center gap-3 flex-wrap flex-1">
            {worldsLoading ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin text-dark-500" />
            ) : worlds.length === 0 ? (
              <span className="text-sm text-dark-500">No worlds found — start the server first.</span>
            ) : worlds.map((w) => (
              <div key={w.name} className="flex items-center gap-1.5 bg-dark-800/60 border border-dark-border rounded-lg px-2.5 py-1">
                <span className="text-sm font-medium">{w.name}</span>
                <span className="text-xs text-dark-500">{formatSize(w.size)}</span>
                <a
                  href={`/api/servers/${serverId}/worlds/${w.name}/download`}
                  download
                  className="p-0.5 text-dark-400 hover:text-white transition-colors"
                  title="Download as zip"
                  onClick={e => e.stopPropagation()}
                >
                  <Download className="w-3 h-3" />
                </a>
                <button
                  onClick={() => deleteWorld(w.name)}
                  disabled={!isStopped || worldBusy === w.name}
                  className="p-0.5 text-dark-400 hover:text-red-400 transition-colors disabled:opacity-30"
                  title={isStopped ? 'Delete world' : 'Stop server first'}
                >
                  {worldBusy === w.name
                    ? <Loader2 className="w-3 h-3 animate-spin" />
                    : <Trash2 className="w-3 h-3" />
                  }
                </button>
              </div>
            ))}
          </div>
          <button onClick={loadWorlds} className="btn-ghost p-1.5 shrink-0" title="Refresh worlds">
            <RefreshCw className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-2 gap-4" style={{ minHeight: '32rem' }}>
        <div className="card space-y-3 flex flex-col min-h-0">
          <div className="flex items-center justify-between shrink-0">
            <div className="flex items-center gap-2 min-w-0">
              <button onClick={() => openDir('/')} className="text-xs text-dark-400 hover:text-white transition-colors shrink-0">root</button>
              {path !== '/' && path.split('/').filter(Boolean).map((seg, i, arr) => {
                const segPath = '/' + arr.slice(0, i + 1).join('/')
                return (
                  <span key={segPath} className="flex items-center gap-1 min-w-0">
                    <ChevronRight className="w-3 h-3 text-dark-600 shrink-0" />
                    <button onClick={() => openDir(segPath)} className="text-xs text-dark-400 hover:text-white transition-colors truncate max-w-24">{seg}</button>
                  </span>
                )
              })}
            </div>
            <div className="flex items-center gap-1.5 shrink-0">
              <button onClick={() => openDir(path)} className="btn-ghost p-1.5" title="Refresh">
                <RefreshCw className="w-3.5 h-3.5" />
              </button>
              <button onClick={() => fileInput.current?.click()} className="btn-ghost p-1.5" title="Upload">
                <Upload className="w-3.5 h-3.5" />
              </button>
              <input ref={fileInput} type="file" multiple className="hidden" onChange={uploadFiles} />
            </div>
          </div>

          {error && (
            <p className="text-xs text-red-400 bg-red-500/10 border border-red-500/20 rounded px-3 py-1.5 shrink-0">{error}</p>
          )}

          <div className="flex-1 overflow-y-auto divide-y divide-dark-border">
            {loading ? (
              <p className="text-sm text-dark-500 flex items-center gap-2 py-3"><Loader2 className="w-3.5 h-3.5 animate-spin" /> Loading…</p>
            ) : (
              <>
                {path !== '/' && (
                  <button onClick={goUp} className="w-full flex items-center gap-3 py-2 px-1 text-sm text-dark-400 hover:text-white transition-colors">
                    <ArrowLeft className="w-4 h-4 shrink-0" />
                    <span>..</span>
                  </button>
                )}
                {entries.length === 0 && (
                  <p className="text-sm text-dark-500 py-4 text-center">Directory is empty.</p>
                )}
                {entries.map((entry) => (
                  <div
                    key={entry.path}
                    onClick={() => openFile(entry)}
                    className={`flex items-center gap-3 py-2 px-1 rounded cursor-pointer group transition-colors ${
                      editing === entry.path ? 'bg-nova-600/10' : 'hover:bg-dark-800/50'
                    }`}
                  >
                    {entry.is_dir
                      ? <Folder className="w-4 h-4 text-nova-400 shrink-0" />
                      : <FileText className={`w-4 h-4 shrink-0 ${EXT_COLORS[fileExt(entry.name)] ?? 'text-dark-400'}`} />
                    }
                    <span className="text-sm flex-1 truncate">{entry.name}</span>
                    {!entry.is_dir && (
                      <span className="text-xs text-dark-500 shrink-0">
                        {!isTextFile(entry.name) && entry.size <= 1_000_000
                          ? <span className="text-dark-600 mr-1.5" title="Binary — will download">↓</span>
                          : null}
                        {formatSize(entry.size)}
                      </span>
                    )}
                    <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      {!entry.is_dir && (
                        <button
                          onClick={(e) => { e.stopPropagation(); api.files.download(serverId, entry.path) }}
                          className="p-1 text-dark-400 hover:text-white transition-colors"
                          title="Download"
                        >
                          <Download className="w-3.5 h-3.5" />
                        </button>
                      )}
                      <button
                        onClick={(e) => deleteEntry(entry, e)}
                        className="p-1 text-dark-400 hover:text-red-400 transition-colors"
                        title="Delete"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>
                ))}
              </>
            )}
          </div>
        </div>

        <div className="card flex flex-col min-h-0">
          {editing === null ? (
            <div className="flex-1 flex items-center justify-center text-dark-500 text-sm">
              Select a file to edit
            </div>
          ) : (
            <>
              <div className="flex items-center justify-between mb-3 shrink-0">
                <p className="text-sm font-medium font-mono truncate">
                  {editing.split('/').pop()}
                  {hasUnsavedChanges && <span className="text-amber-400 ml-2">(unsaved)</span>}
                </p>
                <div className="flex items-center gap-2 shrink-0">
                  <button
                    onClick={saveFile}
                    disabled={saving || !hasUnsavedChanges}
                    className="btn-primary flex items-center gap-1.5 text-sm px-3 py-1.5"
                  >
                    {saving ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Save className="w-3.5 h-3.5" />}
                    Save
                  </button>
                  <button
                    onClick={() => {
                      if (!confirmDiscardUnsaved()) return
                      setEditing(null)
                      setContent('')
                      setOriginalContent('')
                    }}
                    className="btn-ghost p-1.5"
                  >
                    <X className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
              <div className="flex-1 min-h-0">
                <Editor
                  height="100%"
                  language={editorLanguage(editing.split('/').pop() ?? '')}
                  theme="vs-dark"
                  value={content}
                  onChange={(value) => setContent(value ?? '')}
                  options={{
                    minimap: { enabled: false },
                    fontSize: 13,
                    lineNumbersMinChars: 3,
                    wordWrap: 'on',
                    tabSize: 2,
                    insertSpaces: true,
                    automaticLayout: true,
                    scrollBeyondLastLine: false,
                  }}
                />
              </div>
            </>
          )}
        </div>

      </div>
    </div>
  )
}
