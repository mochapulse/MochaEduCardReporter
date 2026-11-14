import { waitFor } from '@testing-library/react'
import { useState } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { expectNoA11yViolations } from '@/test/axe'
import { renderUi } from '@/test/render'
import { Button } from './button'
import { ConfirmDialog } from './dialog'

function Harness({ onConfirm }: { onConfirm: () => void }) {
  const [open, setOpen] = useState(false)
  return (
    <>
      <Button
        label="Abrir"
        onClick={() => {
          setOpen(true)
        }}
      />
      <ConfirmDialog
        destructive
        open={open}
        onOpenChange={setOpen}
        title="¿Reemplazar?"
        description="Se reemplazarán 27 boletines."
        confirmLabel="Sí, reemplazar"
        cancelLabel="No, volver"
        onConfirm={onConfirm}
      />
    </>
  )
}

describe('ConfirmDialog', () => {
  it('is an accessible dialog with explicit buttons and safe initial focus', async () => {
    const { user, getByRole } = renderUi(<Harness onConfirm={vi.fn()} />)
    await user.click(getByRole('button', { name: 'Abrir' }))
    const dialog = getByRole('dialog', { name: '¿Reemplazar?' })
    expect(dialog).toHaveAccessibleDescription('Se reemplazarán 27 boletines.')
    expect(getByRole('button', { name: 'No, volver' })).toHaveFocus()
    await expectNoA11yViolations(document.body)
  })

  it('traps focus inside while open', async () => {
    const { user, getByRole } = renderUi(<Harness onConfirm={vi.fn()} />)
    await user.click(getByRole('button', { name: 'Abrir' }))
    const dialog = getByRole('dialog')
    for (let i = 0; i < 6; i++) {
      await user.tab()
      expect(dialog).toContainElement(document.activeElement as HTMLElement)
    }
  })

  it('closes with Esc, returns focus to the opener and does not confirm', async () => {
    const onConfirm = vi.fn()
    const { user, getByRole, queryByRole } = renderUi(<Harness onConfirm={onConfirm} />)
    const opener = getByRole('button', { name: 'Abrir' })
    await user.click(opener)
    await user.keyboard('{Escape}')
    expect(queryByRole('dialog')).not.toBeInTheDocument()
    await waitFor(() => {
      expect(opener).toHaveFocus()
    })
    expect(onConfirm).not.toHaveBeenCalled()
  })

  it('confirms only through the explicit button', async () => {
    const onConfirm = vi.fn()
    const { user, getByRole, queryByRole } = renderUi(<Harness onConfirm={onConfirm} />)
    await user.click(getByRole('button', { name: 'Abrir' }))
    await user.click(getByRole('button', { name: 'Sí, reemplazar' }))
    expect(onConfirm).toHaveBeenCalledTimes(1)
    expect(queryByRole('dialog')).not.toBeInTheDocument()
  })
})
