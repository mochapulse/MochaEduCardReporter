import { useState } from 'react'
import { describe, expect, it } from 'vitest'
import { expectNoA11yViolations } from '@/test/axe'
import { renderUi } from '@/test/render'
import { Checkbox } from './checkbox'

function Harness() {
  const [checked, setChecked] = useState(false)
  return (
    <Checkbox label="Recordar esta carpeta base" checked={checked} onCheckedChange={setChecked} />
  )
}

describe('Checkbox', () => {
  it('toggles with the keyboard (Space) and the mouse, label included', async () => {
    const { user, getByRole, getByText } = renderUi(<Harness />)
    const box = getByRole('checkbox', { name: 'Recordar esta carpeta base' })
    await user.tab()
    expect(box).toHaveFocus()
    await user.keyboard(' ')
    expect(box).toBeChecked()
    await user.click(getByText('Recordar esta carpeta base'))
    expect(box).not.toBeChecked()
  })

  it('exposes the mixed state', async () => {
    const { container, getByRole } = renderUi(
      <Checkbox label="Todos" checked="indeterminate" onCheckedChange={() => undefined} />,
    )
    expect(getByRole('checkbox')).toHaveAttribute('aria-checked', 'mixed')
    await expectNoA11yViolations(container)
  })

  it('supports a visually hidden label that still names the box', async () => {
    const { container, getByRole } = renderUi(
      <Checkbox
        hideLabel
        label="Seleccionar a Ana"
        checked={false}
        onCheckedChange={() => undefined}
      />,
    )
    expect(getByRole('checkbox')).toHaveAccessibleName('Seleccionar a Ana')
    await expectNoA11yViolations(container)
  })
})
