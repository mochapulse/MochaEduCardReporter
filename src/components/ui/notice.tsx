import { CircleAlert, CircleCheck, Info, TriangleAlert, type LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'
import { t } from '@/i18n/es'
import { cn } from '@/lib/utils'

export type NoticeVariant = 'info' | 'success' | 'warning' | 'danger'

const variantStyle: Record<NoticeVariant, { icon: LucideIcon; border: string; iconColor: string }> =
  {
    info: { icon: Info, border: 'border-accent', iconColor: 'text-accent' },
    success: { icon: CircleCheck, border: 'border-success', iconColor: 'text-success' },
    warning: { icon: TriangleAlert, border: 'border-warning', iconColor: 'text-warning' },
    danger: { icon: CircleAlert, border: 'border-danger', iconColor: 'text-danger' },
  }

export interface NoticeProps {
  variant: NoticeVariant
  /** Optional bold heading above the message. */
  title?: string
  children: ReactNode
  className?: string
}

/**
 * Status message. Status is carried by icon + text (the variant name is read by
 * screen readers too); color only reinforces it.
 */
export function Notice({ variant, title, children, className }: NoticeProps) {
  const { icon: Icon, border, iconColor } = variantStyle[variant]
  return (
    <div
      role={variant === 'danger' ? 'alert' : 'status'}
      data-variant={variant}
      className={cn(
        'flex items-start gap-3 rounded-md border-2 border-l-8 bg-surface p-4 text-fg',
        border,
        className,
      )}
    >
      <Icon aria-hidden="true" className={cn('mt-0.5 size-6 shrink-0', iconColor)} />
      <div className="min-w-0">
        {title ? (
          <p className="font-semibold">
            <span className="sr-only">{t(`notice.${variant}`)}: </span>
            {title}
          </p>
        ) : null}
        <div>
          {title ? null : <span className="sr-only">{t(`notice.${variant}`)}: </span>}
          {children}
        </div>
      </div>
    </div>
  )
}
