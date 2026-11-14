import type { HTMLAttributes } from 'react'
import { cn } from '@/lib/utils'

export function Card({ className, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn('rounded-lg border border-border bg-surface p-6 text-fg shadow-sm', className)}
      {...rest}
    />
  )
}

interface CardTitleProps extends HTMLAttributes<HTMLHeadingElement> {
  /** Heading level, so the card fits the page outline. */
  level?: 2 | 3 | 4
}

export function CardTitle({ level = 3, className, ...rest }: CardTitleProps) {
  const Heading = `h${level}` as const
  return <Heading className={cn('text-xl font-semibold', className)} {...rest} />
}

export function CardContent({ className, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn('mt-2 text-fg-muted', className)} {...rest} />
}
