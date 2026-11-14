import { cva, type VariantProps } from 'class-variance-authority'
import type { LucideIcon } from 'lucide-react'
import type { ButtonHTMLAttributes, Ref } from 'react'
import { cn } from '@/lib/utils'

export const buttonVariants = cva(
  [
    'inline-flex min-h-11 min-w-11 cursor-pointer items-center justify-center gap-2',
    'rounded-md px-5 py-2 text-base font-semibold whitespace-nowrap select-none',
    'transition-colors disabled:cursor-not-allowed disabled:opacity-50',
  ],
  {
    variants: {
      variant: {
        primary: 'bg-primary text-on-primary hover:bg-primary-hover',
        outline: 'border-2 border-primary bg-surface text-primary hover:bg-surface-muted',
        ghost: 'bg-transparent text-primary hover:bg-surface-muted',
        danger: 'border-2 border-danger bg-surface text-danger hover:bg-surface-muted',
      },
    },
    defaultVariants: { variant: 'primary' },
  },
)

export interface ButtonProps
  extends
    Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'children'>,
    VariantProps<typeof buttonVariants> {
  /** Visible text. Required: a button never relies on an icon alone. */
  label: string
  /** Optional leading icon, always shown together with the label. */
  icon?: LucideIcon
  ref?: Ref<HTMLButtonElement>
}

export function Button({
  label,
  icon: Icon,
  variant,
  className,
  type = 'button',
  ref,
  ...rest
}: ButtonProps) {
  return (
    <button ref={ref} type={type} className={cn(buttonVariants({ variant }), className)} {...rest}>
      {Icon ? <Icon aria-hidden="true" className="size-5 shrink-0" /> : null}
      <span>{label}</span>
    </button>
  )
}
