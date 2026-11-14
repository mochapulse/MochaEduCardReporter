import { fireEvent } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { expectNoA11yViolations } from '@/test/axe'
import { renderUi } from '@/test/render'
import { DropZone } from './drop-zone'

describe('DropZone', () => {
  it('always has the visible button and it works from the keyboard', async () => {
    const onChooseFile = vi.fn()
    const { container, user, getByRole } = renderUi(<DropZone onChooseFile={onChooseFile} />)
    const button = getByRole('button', { name: 'Elegir archivo de notas' })
    await user.tab()
    expect(button).toHaveFocus()
    await user.keyboard('{Enter}')
    await user.keyboard(' ')
    expect(onChooseFile).toHaveBeenCalledTimes(2)
    await expectNoA11yViolations(container)
  })

  it('shows the drag-over state and leaves it after the drop', () => {
    const onDropFiles = vi.fn()
    const { getByText, getByRole } = renderUi(
      <DropZone onChooseFile={() => undefined} onDropFiles={onDropFiles} />,
    )
    const zone = getByRole('region')
    expect(zone).toHaveAttribute('data-drag-over', 'false')
    fireEvent.dragEnter(zone)
    expect(zone).toHaveAttribute('data-drag-over', 'true')
    expect(getByText('Suelta el archivo aquí')).toBeInTheDocument()

    const good = new File(['x'], 'NOTAS.xlsx')
    const bad = new File(['x'], 'foto.png')
    fireEvent.drop(zone, { dataTransfer: { files: [good, bad] } })
    expect(zone).toHaveAttribute('data-drag-over', 'false')
    expect(onDropFiles).toHaveBeenCalledWith([good], [bad])
  })

  it('can render forced drag-over for the gallery and stays accessible', async () => {
    const { container } = renderUi(<DropZone forceDragOver onChooseFile={() => undefined} />)
    expect(container.querySelector('section')).toHaveAttribute('data-drag-over', 'true')
    await expectNoA11yViolations(container)
  })
})
