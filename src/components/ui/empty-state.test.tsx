import { FileText } from 'lucide-react'
import { describe, expect, it } from 'vitest'
import { expectNoA11yViolations } from '@/test/axe'
import { renderUi } from '@/test/render'
import { Button } from './button'
import { EmptyState } from './empty-state'

describe('EmptyState', () => {
  it('shows title, description and next step', async () => {
    const { container, getByRole, getByText } = renderUi(
      <EmptyState
        icon={FileText}
        title="No hay estudiantes"
        description="Elegir otra hoja."
        action={<Button label="Elegir otra hoja" />}
      />,
    )
    expect(getByRole('heading', { name: 'No hay estudiantes' })).toBeInTheDocument()
    expect(getByText('Elegir otra hoja.')).toBeInTheDocument()
    expect(getByRole('button', { name: 'Elegir otra hoja' })).toBeInTheDocument()
    await expectNoA11yViolations(container)
  })
})
