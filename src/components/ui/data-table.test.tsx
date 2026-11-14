import { useState } from 'react'
import { describe, expect, it } from 'vitest'
import { expectNoA11yViolations } from '@/test/axe'
import { renderUi } from '@/test/render'
import { DataTable, type DataTableColumn } from './data-table'

interface Row {
  id: string
  name: string
}
const rows: Row[] = [
  { id: 'a', name: 'ÁLVAREZ, Camila' },
  { id: 'b', name: 'GÓMEZ, Andrés' },
  { id: 'c', name: 'MARTÍNEZ, Ana' },
]
const columns: DataTableColumn<Row>[] = [{ id: 'name', header: 'Nombre', cell: (row) => row.name }]

function Harness({ initial = [] as string[], data = rows }: { initial?: string[]; data?: Row[] }) {
  const [selected, setSelected] = useState<ReadonlySet<string>>(new Set(initial))
  return (
    <DataTable
      caption="Estudiantes"
      columns={columns}
      rows={data}
      getRowId={(row) => row.id}
      getRowLabel={(row) => `Seleccionar a ${row.name}`}
      selectedIds={selected}
      onSelectedIdsChange={setSelected}
      emptyState={<p>No hay estudiantes para mostrar.</p>}
    />
  )
}

describe('DataTable', () => {
  it('renders a named table with column headers and passes axe', async () => {
    const { container, getByRole } = renderUi(<Harness />)
    expect(getByRole('table', { name: 'Estudiantes' })).toBeInTheDocument()
    expect(getByRole('columnheader', { name: 'Nombre' })).toBeInTheDocument()
    expect(container.querySelector('thead')).toHaveClass('sticky')
    await expectNoA11yViolations(container)
  })

  it('select-all is unchecked, indeterminate or checked depending on the rows', async () => {
    const { user, getByRole } = renderUi(<Harness />)
    const all = getByRole('checkbox', { name: 'Seleccionar todos' })
    expect(all).toHaveAttribute('aria-checked', 'false')

    await user.click(getByRole('checkbox', { name: 'Seleccionar a GÓMEZ, Andrés' }))
    expect(all).toHaveAttribute('aria-checked', 'mixed')

    await user.click(all)
    expect(all).toHaveAttribute('aria-checked', 'true')
    for (const box of document.querySelectorAll('tbody [role=checkbox]')) {
      expect(box).toHaveAttribute('aria-checked', 'true')
    }

    await user.click(all)
    expect(all).toHaveAttribute('aria-checked', 'false')
  })

  it('announces the selection count', async () => {
    const { user, getByRole, container } = renderUi(<Harness initial={['a']} />)
    expect(container).toHaveTextContent('1 seleccionado de 3')
    await user.click(getByRole('checkbox', { name: 'Seleccionar a MARTÍNEZ, Ana' }))
    expect(container).toHaveTextContent('2 seleccionados de 3')
  })

  it('is keyboard navigable: Tab order, Space toggles, arrows move between rows', async () => {
    const { user, getByRole } = renderUi(<Harness />)
    await user.tab() // scroll region
    expect(getByRole('region')).toHaveFocus()
    await user.tab() // select all
    expect(getByRole('checkbox', { name: 'Seleccionar todos' })).toHaveFocus()
    await user.tab()
    const first = getByRole('checkbox', { name: 'Seleccionar a ÁLVAREZ, Camila' })
    expect(first).toHaveFocus()
    await user.keyboard(' ')
    expect(first).toHaveAttribute('aria-checked', 'true')
    await user.keyboard('{ArrowDown}')
    expect(getByRole('checkbox', { name: 'Seleccionar a GÓMEZ, Andrés' })).toHaveFocus()
    await user.keyboard('{ArrowUp}')
    expect(first).toHaveFocus()
  })

  it('shows the empty state instead of a table when there are no rows', async () => {
    const { container, queryByRole, getByText } = renderUi(<Harness data={[]} />)
    expect(queryByRole('table')).not.toBeInTheDocument()
    expect(getByText('No hay estudiantes para mostrar.')).toBeInTheDocument()
    await expectNoA11yViolations(container)
  })
})
