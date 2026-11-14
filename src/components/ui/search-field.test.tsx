import { useState } from 'react'
import { describe, expect, it } from 'vitest'
import { expectNoA11yViolations } from '@/test/axe'
import { renderUi } from '@/test/render'
import { SearchField } from './search-field'

function Harness() {
  const [value, setValue] = useState('')
  return <SearchField label="Buscar estudiante" value={value} onValueChange={setValue} />
}

describe('SearchField', () => {
  it('has a visible label and filters as the user types', async () => {
    const { container, user, getByRole } = renderUi(<Harness />)
    const input = getByRole('searchbox', { name: 'Buscar estudiante' })
    await user.type(input, 'Ana')
    expect(input).toHaveValue('Ana')
    await expectNoA11yViolations(container)
  })

  it('clears with the text button and with Esc', async () => {
    const { user, getByRole, queryByRole } = renderUi(<Harness />)
    const input = getByRole('searchbox')
    await user.type(input, 'Ana')
    await user.click(getByRole('button', { name: 'Borrar búsqueda' }))
    expect(input).toHaveValue('')
    expect(queryByRole('button', { name: 'Borrar búsqueda' })).not.toBeInTheDocument()
    await user.type(input, 'Luis{Escape}')
    expect(input).toHaveValue('')
  })
})
