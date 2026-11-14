import { Tooltip as TooltipPrimitive } from 'radix-ui'
import type { ReactElement, ReactNode } from 'react'

export const TooltipProvider = TooltipPrimitive.Provider

export interface TooltipProps {
  /** Supplementary text only: it must never be the sole carrier of information. */
  content: ReactNode
  /** Single focusable element that triggers the tooltip. */
  children: ReactElement
}

export function Tooltip({ content, children }: TooltipProps) {
  return (
    <TooltipPrimitive.Root>
      <TooltipPrimitive.Trigger asChild>{children}</TooltipPrimitive.Trigger>
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Content
          sideOffset={6}
          className="z-50 max-w-sm rounded-md bg-fg px-3 py-2 text-sm text-background shadow-md break-all"
        >
          {content}
        </TooltipPrimitive.Content>
      </TooltipPrimitive.Portal>
    </TooltipPrimitive.Root>
  )
}
