import { describe, expect, it } from 'vitest'
import { expectNoA11yViolations } from '@/test/axe'
import { renderUi } from '@/test/render'
import { Card, CardContent, CardTitle } from './card'

describe('Card', () => {
  it('renders heading at the requested level and content', async () => {
    const { container, getByRole } = renderUi(
      <Card>
        <CardTitle level={2}>Francés IV</CardTitle>
        <CardContent>27 estudiantes</CardContent>
      </Card>,
    )
    expect(getByRole('heading', { level: 2, name: 'Francés IV' })).toBeInTheDocument()
    expect(container).toHaveTextContent('27 estudiantes')
    await expectNoA11yViolations(container)
  })
})
