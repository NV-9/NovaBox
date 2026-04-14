import { Loader2, Save, Settings, FolderOpen, Shield, RefreshCw, Clock, Globe } from 'lucide-react'
import { format } from 'date-fns'
import { Toggle } from '@/components/Toggle'
import { SettingRow } from '@/components/SettingRow'
import { InfoRow } from '@/components/InfoRow'
import { Trash2 } from 'lucide-react'
import type { Server, CreateServerRequest } from '@/types'

interface Props {
  server:          Server
  form:            Partial<CreateServerRequest>
  saving:          boolean
  saved:           boolean
  confirmDelete:   boolean
  onFormChange:    <K extends keyof CreateServerRequest>(key: K, value: CreateServerRequest[K]) => void
  onSave:          (e: React.FormEvent) => void
  onConfirmDelete: (v: boolean) => void
  onDelete:        () => void
}

export function SettingsTab({
  server, form, saving, saved, confirmDelete,
  onFormChange, onSave, onConfirmDelete, onDelete,
}: Props) {
  return (
    <form onSubmit={onSave} className="space-y-5 max-w-2xl">
      <div className="card space-y-1">
        <p className="text-xs font-semibold text-dark-400 uppercase tracking-wider mb-2 flex items-center gap-1.5">
          <FolderOpen className="w-3.5 h-3.5" /> Server Info
        </p>
        <dl>
          <InfoRow label="Server ID"    value={server.id} />
          <InfoRow label="Container ID" value={server.container_id ?? 'None'} />
          <InfoRow label="Server Image" value="itzg/minecraft-server" />
          <InfoRow label="Working Dir"  value={server.data_dir} />
          <InfoRow label="Log Location" value={`${server.data_dir}/logs/latest.log`} />
          <InfoRow label="Game Port"    value={server.port} />
          <InfoRow label="RCON Port"    value={server.rcon_port} />
          <InfoRow label="Map Mod"      value={server.map_mod ?? 'None'} />
          <InfoRow label="Created"      value={format(new Date(server.created_at), 'PPp')} />
        </dl>
      </div>

      <div className="card">
        <p className="text-xs font-semibold text-dark-400 uppercase tracking-wider mb-3 flex items-center gap-1.5">
          <Settings className="w-3.5 h-3.5" /> General
        </p>
        <div className="space-y-3">
          <div>
            <label className="block text-sm font-medium mb-1.5">Server Name</label>
            <input className="input" value={form.name ?? ''} onChange={e => onFormChange('name', e.target.value)} />
          </div>
          <div>
            <label className="block text-sm font-medium mb-1.5">Description</label>
            <input className="input" value={form.description ?? ''} onChange={e => onFormChange('description', e.target.value)} />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-sm font-medium mb-1.5">Max Players</label>
              <input type="number" className="input" min={1} max={200} value={form.max_players ?? 20} onChange={e => onFormChange('max_players', parseInt(e.target.value))} />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1.5">Memory</label>
              <select className="select" value={form.memory_mb ?? 2048} onChange={e => onFormChange('memory_mb', parseInt(e.target.value))}>
                <option value={512}>512 MB</option>
                <option value={1024}>1 GB</option>
                <option value={2048}>2 GB</option>
                <option value={4096}>4 GB</option>
                <option value={8192}>8 GB</option>
                <option value={12288}>12 GB</option>
                <option value={16384}>16 GB</option>
              </select>
            </div>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-sm font-medium mb-1.5">Min RAM (Xms)</label>
              <input
                type="number"
                className="input"
                min={128}
                max={form.memory_mb ?? 2048}
                value={form.min_memory_mb ?? ''}
                placeholder="Default: max RAM"
                onChange={e => onFormChange('min_memory_mb', e.target.value ? parseInt(e.target.value) : undefined)}
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1.5">Custom JVM Flags</label>
              <input
                className="input"
                value={form.jvm_flags ?? ''}
                placeholder="e.g. -XX:+UseG1GC -XX:MaxGCPauseMillis=100"
                onChange={e => onFormChange('jvm_flags', e.target.value || undefined)}
              />
            </div>
          </div>
        </div>
      </div>

      <div className="card">
        <p className="text-xs font-semibold text-dark-400 uppercase tracking-wider mb-1 flex items-center gap-1.5">
          <Shield className="w-3.5 h-3.5" /> Authentication
        </p>
        <SettingRow label="Online Mode" description="Require players to have a legitimate Minecraft account. Disable for LAN or offline play.">
          <Toggle value={form.online_mode ?? true} onChange={v => onFormChange('online_mode', v)} />
        </SettingRow>
      </div>

      <div className="card">
        <p className="text-xs font-semibold text-dark-400 uppercase tracking-wider mb-1 flex items-center gap-1.5">
          <RefreshCw className="w-3.5 h-3.5" /> Lifecycle
        </p>
        <SettingRow label="Auto Start" description="Automatically start this server when NovaBox starts.">
          <Toggle value={form.auto_start ?? false} onChange={v => onFormChange('auto_start', v)} />
        </SettingRow>
        {form.auto_start && (
          <SettingRow label="Auto Start Delay" description="Seconds to wait before starting.">
            <div className="flex items-center gap-2">
              <input type="number" className="input w-20 text-right" min={0} max={300} value={form.auto_start_delay ?? 0} onChange={e => onFormChange('auto_start_delay', parseInt(e.target.value) || 0)} />
              <span className="text-sm text-dark-400">s</span>
            </div>
          </SettingRow>
        )}
        <SettingRow label="Crash Detection" description="Automatically restart the server after a crash (up to 3 times).">
          <Toggle value={form.crash_detection ?? true} onChange={v => onFormChange('crash_detection', v)} />
        </SettingRow>
      </div>

      <div className="card">
        <p className="text-xs font-semibold text-dark-400 uppercase tracking-wider mb-1 flex items-center gap-1.5">
          <Clock className="w-3.5 h-3.5" /> Shutdown
        </p>
        <SettingRow label="Shutdown Timeout" description="Seconds to wait for a graceful stop before force-killing the container.">
          <div className="flex items-center gap-2">
            <input type="number" className="input w-20 text-right" min={5} max={300} value={form.shutdown_timeout ?? 30} onChange={e => onFormChange('shutdown_timeout', parseInt(e.target.value) || 30)} />
            <span className="text-sm text-dark-400">s</span>
          </div>
        </SettingRow>
      </div>

      <div className="card">
        <p className="text-xs font-semibold text-dark-400 uppercase tracking-wider mb-1 flex items-center gap-1.5">
          <Globe className="w-3.5 h-3.5" /> Visibility
        </p>
        <SettingRow label="Show on Status Page" description="Display this server on the public NovaBox status page.">
          <Toggle value={form.show_on_status_page ?? false} onChange={v => onFormChange('show_on_status_page', v)} />
        </SettingRow>
      </div>

      <div className="flex gap-3">
        <button type="submit" disabled={saving} className="btn-primary flex items-center gap-2">
          {saving
            ? <Loader2 className="w-4 h-4 animate-spin" />
            : saved
              ? <span className="text-emerald-300">Saved ✓</span>
              : <><Save className="w-4 h-4" /> Save Changes</>
          }
        </button>
      </div>

      <div className="card border-red-500/20">
        <p className="text-sm font-medium text-red-400 mb-3">Danger Zone</p>
        {confirmDelete ? (
          <div className="flex gap-2">
            <p className="text-sm text-dark-400 flex-1">Are you sure? This is irreversible.</p>
            <button type="button" onClick={() => onConfirmDelete(false)} className="btn-ghost text-sm">Cancel</button>
            <button type="button" onClick={onDelete} className="px-3 py-1.5 rounded-lg bg-red-600 text-white text-sm hover:bg-red-500 transition-colors">Delete</button>
          </div>
        ) : (
          <button type="button" onClick={() => onConfirmDelete(true)} className="flex items-center gap-2 text-sm text-red-400 hover:text-red-300 transition-colors">
            <Trash2 className="w-4 h-4" /> Delete Server
          </button>
        )}
      </div>
    </form>
  )
}
