import type { LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

export interface EmptyStateProps {
  icon?: LucideIcon
  title: string
  description?: string
  /** Optional next step, usually a Button. */
  action?: ReactNode
  className?: string
}

export function EmptyState({ icon: Icon, title, description, action, className }: EmptyStateProps) {
  return (
    <div
      className={cn(
        'flex flex-col items-center gap-3 rounded-lg border border-dashed border-fg-muted bg-surface p-8 text-center',
        className,
      )}
    >
      {Icon ? <Icon aria-hidden="true" className="size-10 text-fg-muted" /> : null}
      <h3 className="text-xl font-semibold">{title}</h3>
      {description ? <p className="max-w-prose text-fg-muted">{description}</p> : null}
      {action}
    </div>
  )
}
