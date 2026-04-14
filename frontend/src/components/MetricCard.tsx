import { type ReactNode } from 'react'
import { clsx } from 'clsx'

interface Props {
  label: string
  value: string | number
  sub?: string
  icon: ReactNode
  accent?: 'blue' | 'green' | 'yellow' | 'red'
}

const ACCENT = {
  blue:   'text-nova-400',
  green:  'text-emerald-400',
  yellow: 'text-amber-400',
  red:    'text-red-400',
}

export function MetricCard({ label, value, sub, icon, accent = 'blue' }: Props) {
  return (
    <div className="card flex items-start gap-4">
      <div className={clsx('p-2 rounded-lg bg-dark-border mt-0.5 shrink-0', ACCENT[accent])}>
        {icon}
      </div>
      <div className="min-w-0">
        <p className="text-xs text-dark-400 mb-1">{label}</p>
        <p className="text-2xl font-bold leading-tight">{value}</p>
        {sub && <p className="text-xs text-dark-500 mt-0.5">{sub}</p>}
      </div>
    </div>
  )
}
