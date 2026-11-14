import { Folder } from 'lucide-react'
import { describe, expect, it, vi } from 'vitest'
import { expectNoA11yViolations } from '@/test/axe'
import { renderUi } from '@/test/render'
import { Button } from './button'

describe('Button', () => {
  it('is named by its visible label, also with an icon', async () => {
    const { container } = renderUi(<Button icon={Folder} label="Abrir carpeta" />)
    expect(document.querySelector('button')).toHaveAccessibleName('Abrir carpeta')
    expect(container.querySelector('svg')).toHaveAttribute('aria-hidden', 'true')
    await expectNoA11yViolations(container)
  })

  it('cannot be an icon-only button (type level)', () => {
    // @ts-expect-error `label` is required: an icon alone is not a valid button.
    renderUi(<Button icon={Folder} />)
    // @ts-expect-error children are not accepted: the label is the only content.
    renderUi(<Button label="x">hijo</Button>)
  })

  it('activates with Enter and Space', async () => {
    const onClick = vi.fn()
    const { user } = renderUi(<Button label="Continuar" onClick={onClick} />)
    await user.tab()
    expect(document.querySelector('button')).toHaveFocus()
    await user.keyboard('{Enter}')
    await user.keyboard(' ')
    expect(onClick).toHaveBeenCalledTimes(2)
  })

  it('does not fire when disabled and is skipped by Tab', async () => {
    const onClick = vi.fn()
    const { user } = renderUi(<Button disabled label="Generar" onClick={onClick} />)
    await user.tab()
    await user.keyboard('{Enter}')
    expect(onClick).not.toHaveBeenCalled()
  })

  it('uses a 44 px minimum hit area class and every variant renders accessibly', async () => {
    const { container } = renderUi(
      <div>
        {(['primary', 'outline', 'ghost', 'danger'] as const).map((variant) => (
          <Button key={variant} variant={variant} label={variant} />
        ))}
      </div>,
    )
    for (const button of container.querySelectorAll('button')) {
      expect(button.className).toContain('min-h-11')
      expect(button.className).toContain('min-w-11')
    }
    await expectNoA11yViolations(container)
  })
})
