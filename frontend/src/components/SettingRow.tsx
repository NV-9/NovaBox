interface Props {
  label: string
  description?: string
  children: React.ReactNode
}

export function SettingRow({ label, description, children }: Props) {
  return (
    <div className="flex items-center justify-between gap-6 py-3 border-b border-dark-border last:border-0">
      <div className="min-w-0">
        <p className="text-sm font-medium">{label}</p>
        {description && <p className="text-xs text-dark-400 mt-0.5">{description}</p>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  )
}
