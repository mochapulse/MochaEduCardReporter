import { describe, expect, it, vi } from 'vitest'
import { expectNoA11yViolations } from '@/test/axe'
import { renderUi } from '@/test/render'
import { PathField, splitPathForDisplay } from './path-field'

const path = '/home/profesora/Documentos/Boletines/Francés IV – Grupo 2 – 2025-2'

describe('splitPathForDisplay', () => {
  it('keeps the last two segments in the tail', () => {
    expect(splitPathForDisplay(path)).toEqual({
      head: '/home/profesora/Documentos/',
      tail: 'Boletines/Francés IV – Grupo 2 – 2025-2',
    })
  })

  it('handles Windows paths and short paths', () => {
    expect(splitPathForDisplay('C:\\Users\\Ana\\Documentos\\Boletines')).toEqual({
      head: 'C:\\Users\\Ana\\',
      tail: 'Documentos\\Boletines',
    })
    expect(splitPathForDisplay('/Boletines')).toEqual({ head: '', tail: '/Boletines' })
  })
})

describe('PathField', () => {
  it('exposes the full path to assistive technology and has a labelled change button', async () => {
    const onChange = vi.fn()
    const { container, getByRole, user } = renderUi(
      <PathField label="Los boletines se guardarán en:" path={path} onChange={onChange} />,
    )
    expect(getByRole('group', { name: 'Los boletines se guardarán en:' })).toHaveTextContent(path)
    await user.click(getByRole('button', { name: 'Cambiar carpeta…' }))
    expect(onChange).toHaveBeenCalledTimes(1)
    await expectNoA11yViolations(container)
  })

  it('shows the full path in a tooltip on keyboard focus', async () => {
    const { user, findByRole } = renderUi(
      <PathField label="Carpeta" path={path} onChange={() => undefined} />,
    )
    await user.tab()
    expect(await findByRole('tooltip')).toHaveTextContent(`Ruta completa: ${path}`)
  })
})
