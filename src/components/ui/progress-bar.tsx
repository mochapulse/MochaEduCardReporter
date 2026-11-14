import { useId } from 'react'
import { cn } from '@/lib/utils'

export interface ProgressBarProps {
  value: number
  max: number
  /** Visible status text, e.g. "12 de 27 — MARTÍNEZ, Ana". */
  label: string
  className?: string
}

export function ProgressBar({ value, max, label, className }: ProgressBarProps) {
  const labelId = useId()
  const percent = max > 0 ? Math.min(100, Math.max(0, (value / max) * 100)) : 0
  return (
    <div className={className}>
      <p id={labelId} className="mb-2 text-lg font-semibold">
        {label}
      </p>
      <div
        role="progressbar"
        aria-labelledby={labelId}
        aria-valuemin={0}
        aria-valuemax={max}
        aria-valuenow={value}
        aria-valuetext={label}
        className="h-4 w-full overflow-hidden rounded-full border border-fg-muted bg-surface-muted"
      >
        <div
          className={cn('h-full bg-primary transition-[width]')}
          style={{ width: `${String(percent)}%` }}
        />
      </div>
    </div>
  )
}
