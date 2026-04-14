import { clsx } from 'clsx'

interface Props {
  value: boolean
  onChange: (v: boolean) => void
  disabled?: boolean
}

export function Toggle({ value, onChange, disabled }: Props) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={() => onChange(!value)}
      className={clsx(
        'relative inline-flex h-5 w-9 shrink-0 rounded-full border-2 border-transparent transition-colors focus:outline-none disabled:opacity-40',
        value ? 'bg-nova-500' : 'bg-dark-600'
      )}
    >
      <span className={clsx(
        'pointer-events-none inline-block h-4 w-4 rounded-full bg-white shadow transition-transform',
        value ? 'translate-x-4' : 'translate-x-0'
      )} />
    </button>
  )
}
