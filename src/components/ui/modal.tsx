import { X } from 'lucide-react'
import { Dialog as DialogPrimitive } from 'radix-ui'
import type { ReactNode } from 'react'
import { t } from '@/i18n/es'
import { Button } from './button'

export interface ModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  description: string
  children: ReactNode
}

/** Large dialog for content (not a confirmation): focus trapped, Esc and "Cerrar" close it. */
export function Modal({ open, onOpenChange, title, description, children }: ModalProps) {
  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-40 bg-fg/50 data-[state=open]:animate-in data-[state=open]:fade-in-0" />
        <DialogPrimitive.Content className="fixed inset-4 z-50 mx-auto flex max-w-5xl flex-col gap-3 rounded-lg border border-border bg-surface p-5 text-fg shadow-md data-[state=open]:animate-in data-[state=open]:fade-in-0">
          <div className="flex items-start justify-between gap-4">
            <div className="min-w-0">
              <DialogPrimitive.Title className="text-xl font-semibold">
                {title}
              </DialogPrimitive.Title>
              <DialogPrimitive.Description className="text-fg-muted">
                {description}
              </DialogPrimitive.Description>
            </div>
            <Button
              variant="outline"
              icon={X}
              label={t('common.close')}
              onClick={() => {
                onOpenChange(false)
              }}
            />
          </div>
          <div className="min-h-0 flex-1 overflow-auto">{children}</div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  )
}
