import { clsx } from 'clsx'
import type { ServerStatus } from '@/types'

interface Props {
  status: ServerStatus
}

const CONFIG: Record<ServerStatus, { label: string; cls: string; dot: string }> = {
  running:  { label: 'Running',  cls: 'badge-green',  dot: 'bg-emerald-400' },
  starting: { label: 'Starting', cls: 'badge-yellow', dot: 'bg-amber-400 animate-pulse' },
  stopping: { label: 'Stopping', cls: 'badge-yellow', dot: 'bg-amber-400 animate-pulse' },
  stopped:  { label: 'Stopped',  cls: 'badge-gray',   dot: 'bg-dark-500' },
  error:    { label: 'Error',    cls: 'badge-red',    dot: 'bg-red-400' },
}

export function StatusBadge({ status }: Props) {
  const { label, cls, dot } = CONFIG[status] ?? CONFIG.stopped
  return (
    <span className={cls}>
      <span className={clsx('w-1.5 h-1.5 rounded-full', dot)} />
      {label}
    </span>
  )
}
