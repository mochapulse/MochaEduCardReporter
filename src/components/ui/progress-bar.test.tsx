import { describe, expect, it } from 'vitest'
import { expectNoA11yViolations } from '@/test/axe'
import { renderUi } from '@/test/render'
import { ProgressBar } from './progress-bar'

describe('ProgressBar', () => {
  it('exposes role, values and the text label', async () => {
    const { container, getByRole } = renderUi(
      <ProgressBar value={12} max={27} label="12 de 27 — MARTÍNEZ, Ana" />,
    )
    const bar = getByRole('progressbar', { name: '12 de 27 — MARTÍNEZ, Ana' })
    expect(bar).toHaveAttribute('aria-valuemin', '0')
    expect(bar).toHaveAttribute('aria-valuemax', '27')
    expect(bar).toHaveAttribute('aria-valuenow', '12')
    expect(container).toHaveTextContent('12 de 27 — MARTÍNEZ, Ana')
    await expectNoA11yViolations(container)
  })

  it('clamps the visual width', () => {
    const { container } = renderUi(<ProgressBar value={99} max={27} label="x" />)
    expect(container.querySelector<HTMLElement>('[role=progressbar] > div')?.style.width).toBe(
      '100%',
    )
  })
})
