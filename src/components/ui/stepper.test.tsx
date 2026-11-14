import { describe, expect, it } from 'vitest'
import { expectNoA11yViolations } from '@/test/axe'
import { renderUi } from '@/test/render'
import { Stepper } from './stepper'

const steps = ['Revisar', 'Guardar', 'Generar']

describe('Stepper', () => {
  it('announces "Paso 2 de 3"', () => {
    const { getByText } = renderUi(<Stepper steps={steps} current={2} />)
    expect(getByText('Paso 2 de 3')).toBeInTheDocument()
  })

  it('exposes done / current / upcoming to assistive technology', async () => {
    const { container, getAllByRole, getByRole } = renderUi(<Stepper steps={steps} current={2} />)
    const items = getAllByRole('listitem')
    expect(items.map((item) => item.dataset['state'])).toEqual(['done', 'current', 'upcoming'])
    expect(items[0]).toHaveTextContent('Revisar (completado)')
    expect(items[1]).toHaveTextContent('Guardar (paso actual)')
    expect(items[2]).toHaveTextContent('Generar (pendiente)')
    expect(items[1]).toHaveAttribute('aria-current', 'step')
    expect(items[0]).not.toHaveAttribute('aria-current')
    expect(getByRole('navigation')).toHaveAccessibleName('Pasos del asistente')
    await expectNoA11yViolations(container)
  })
})
