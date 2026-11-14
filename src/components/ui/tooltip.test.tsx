import { describe, expect, it } from 'vitest'
import { renderUi } from '@/test/render'
import { Button } from './button'
import { Tooltip } from './tooltip'

describe('Tooltip', () => {
  it('appears on keyboard focus and closes with Esc', async () => {
    const { user, findByRole, queryByRole } = renderUi(
      <Tooltip content="Solo repite lo escrito">
        <Button label="Ayuda" />
      </Tooltip>,
    )
    await user.tab()
    expect(await findByRole('tooltip')).toHaveTextContent('Solo repite lo escrito')
    await user.keyboard('{Escape}')
    expect(queryByRole('tooltip')).not.toBeInTheDocument()
  })
})
