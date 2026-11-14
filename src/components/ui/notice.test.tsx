import { describe, expect, it } from 'vitest'
import { expectNoA11yViolations } from '@/test/axe'
import { renderUi } from '@/test/render'
import { Notice, type NoticeVariant } from './notice'

const variants: [NoticeVariant, string][] = [
  ['info', 'Información'],
  ['success', 'Listo'],
  ['warning', 'Atención'],
  ['danger', 'Error'],
]

describe('Notice', () => {
  it.each(variants)(
    '%s carries an icon and a text label, not only color',
    async (variant, word) => {
      const { container, getByRole } = renderUi(<Notice variant={variant}>Mensaje</Notice>)
      const notice = getByRole(variant === 'danger' ? 'alert' : 'status')
      expect(notice.querySelector('svg')).toHaveAttribute('aria-hidden', 'true')
      expect(notice).toHaveTextContent(`${word}: Mensaje`)
      await expectNoA11yViolations(container)
    },
  )

  it('shows an optional title', () => {
    const { getByText } = renderUi(
      <Notice variant="warning" title="Revisar">
        Detalle
      </Notice>,
    )
    expect(getByText('Revisar')).toBeInTheDocument()
  })
})
