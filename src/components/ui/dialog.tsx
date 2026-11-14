import { Dialog as DialogPrimitive } from 'radix-ui'
import { useRef } from 'react'
import { t } from '@/i18n/es'
import { Button } from './button'

export interface ConfirmDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  description: string
  /** Plain-language action, e.g. "Sí, reemplazar". Never a bare "OK". */
  confirmLabel?: string
  cancelLabel?: string
  /** Style the confirm action as dangerous; focus then starts on the safe option. */
  destructive?: boolean
  onConfirm: () => void
}

/**
 * Confirmation dialog: focus is trapped inside, Esc closes it, and focus returns
 * to the element that opened it. Focus starts on "cancel" so that Enter never
 * confirms by accident.
 */
export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel = t('dialog.defaultConfirm'),
  cancelLabel = t('dialog.defaultCancel'),
  destructive = false,
  onConfirm,
}: ConfirmDialogProps) {
  const cancelRef = useRef<HTMLButtonElement>(null)
  const openerRef = useRef<HTMLElement | null>(null)
  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-40 bg-fg/50 data-[state=open]:animate-in data-[state=open]:fade-in-0" />
        <DialogPrimitive.Content
          onOpenAutoFocus={(event) => {
            event.preventDefault()
            // No Dialog.Trigger is used, so remember the opener to give focus back on close.
            openerRef.current =
              document.activeElement instanceof HTMLElement ? document.activeElement : null
            cancelRef.current?.focus()
          }}
          onCloseAutoFocus={(event) => {
            event.preventDefault()
            openerRef.current?.focus()
          }}
          className="fixed top-1/2 left-1/2 z-50 w-[min(32rem,calc(100vw-2rem))] -translate-x-1/2 -translate-y-1/2 rounded-lg border border-border bg-surface p-6 text-fg shadow-md data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95"
        >
          <DialogPrimitive.Title className="text-xl font-semibold">{title}</DialogPrimitive.Title>
          <DialogPrimitive.Description className="mt-2 text-fg-muted">
            {description}
          </DialogPrimitive.Description>
          <div className="mt-6 flex flex-wrap justify-end gap-3">
            <Button
              ref={cancelRef}
              variant="outline"
              label={cancelLabel}
              onClick={() => {
                onOpenChange(false)
              }}
            />
            <Button
              variant={destructive ? 'danger' : 'primary'}
              label={confirmLabel}
              onClick={() => {
                onConfirm()
                onOpenChange(false)
              }}
            />
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  )
}
