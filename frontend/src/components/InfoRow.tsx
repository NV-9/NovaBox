interface Props {
  label: string
  value: string | number
}

export function InfoRow({ label, value }: Props) {
  return (
    <div className="flex items-center justify-between py-2 border-b border-dark-border last:border-0">
      <dt className="text-sm text-dark-400">{label}</dt>
      <dd className="text-sm font-mono text-right max-w-[60%] truncate" title={String(value)}>{value}</dd>
    </div>
  )
}
