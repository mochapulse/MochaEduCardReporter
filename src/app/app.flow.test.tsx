import { act, fireEvent, screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { createMockApi, MOCK_BASE, MOCK_PATHS } from '@/lib/api'
import { continueToSave, openFirstSheet, waitForGenerateEnabled } from '@/test/flow-steps'
import { renderApp } from '@/test/render-app'

vi.mock('@/components/pdf-viewer', () => import('@/test/fake-pdf-viewer'))

const FOLDER = `${MOCK_BASE}/Francés IV – Grupo 2 – 2025-2`

describe('assistant: happy path', () => {
  it('goes from picking a file to the results, and back home', async () => {
    const app = renderApp()
    const { user, api } = app

    // Inicio
    expect(screen.getByRole('heading', { name: 'Crear boletines de notas' })).toHaveFocus()
    await user.click(screen.getByRole('button', { name: 'Elegir archivo de notas' }))

    // Several usable sheets: one card per sheet with its student count.
    const sheetsHeading = await screen.findByRole('heading', {
      name: 'Elegir la hoja con las notas',
    })
    expect(sheetsHeading).toHaveFocus()
    expect(screen.getByText('27 estudiantes')).toBeInTheDocument()
    expect(screen.getByText('22 estudiantes')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Usar esta hoja: FRANCAIS IV LE MATIN' }))

    // Paso 1
    const review = await screen.findByRole('heading', { name: 'Paso 1 — Revisar' })
    expect(review).toHaveFocus()
    expect(screen.getByText('Paso 1 de 3')).toBeInTheDocument()
    expect(api.windowTitle()).toBe('Paso 1 — Revisar — Generador de boletines')
    const summary = screen.getByText('Materia').closest('dl') as HTMLElement
    expect(within(summary).getByText('Francés IV')).toBeInTheDocument()
    expect(within(summary).getByText('Mariela Duarte')).toBeInTheDocument()
    expect(within(summary).getByText('2025-2')).toBeInTheDocument()
    expect(
      screen.getByText(
        'La numeración de 3 estudiantes estaba incorrecta en el archivo y se corrigió automáticamente.',
      ),
    ).toBeInTheDocument()
    expect(screen.getByText('27 estudiantes seleccionados de 27')).toBeInTheDocument()
    await continueToSave(app)

    // Paso 2
    expect(screen.getByText('Los boletines se guardarán en:')).toBeInTheDocument()
    expect(screen.getByText(FOLDER, { selector: '.sr-only' })).toBeInTheDocument()
    const generate = await waitForGenerateEnabled('Generar 27 boletines')
    expect(screen.getByText(`Se crearán 27 boletines en ${FOLDER}.`)).toBeInTheDocument()
    await user.click(generate)

    // Resultados
    expect(await screen.findByText('Listo. Se crearon 27 boletines.')).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'Resultados' })).toHaveFocus()
    expect(await screen.findByTestId('viewer-doc')).toHaveTextContent(
      `${FOLDER}/01_ÁLVAREZ_Camila.pdf`,
    )
    await user.click(screen.getByRole('button', { name: 'next' }))
    await waitFor(() => {
      expect(screen.getByTestId('viewer-doc')).toHaveTextContent('02_BERNAL_Sebastián.pdf')
    })
    await user.click(screen.getByRole('button', { name: 'prev' }))
    await waitFor(() => {
      expect(screen.getByTestId('viewer-doc')).toHaveTextContent('01_ÁLVAREZ_Camila.pdf')
    })

    // The request the screens made.
    const call = api.calls.find((c) => c.method === 'generateReports')
    expect(call?.args[0]).toMatchObject({ students: null, outputDir: FOLDER })

    await user.click(screen.getByRole('button', { name: 'Abrir carpeta' }))
    expect(api.calls.some((c) => c.method === 'revealInFolder' && c.args[0] === FOLDER)).toBe(true)
    await user.click(screen.getByRole('button', { name: 'Imprimir' }))
    expect(api.calls.filter((c) => c.method === 'openExternally')).toHaveLength(1)

    await user.click(screen.getByRole('button', { name: 'Generar otros boletines' }))
    expect(
      await screen.findByRole('heading', { name: 'Crear boletines de notas' }),
    ).toBeInTheDocument()
    // The file shows up under "Archivos recientes".
    expect(
      await screen.findByRole('button', { name: /NOTAS FRANCÉS 2025-2\.xls/ }),
    ).toBeInTheDocument()
  })

  it('skips the sheet choice when the file has only one usable sheet', async () => {
    const api = createMockApi({ stepDelayMs: 0, filePicks: [MOCK_PATHS.single] })
    const { user } = renderApp({ api })
    await user.click(screen.getByRole('button', { name: 'Elegir archivo de notas' }))
    expect(await screen.findByRole('heading', { name: 'Paso 1 — Revisar' })).toBeInTheDocument()
    expect(screen.queryByText('Elegir la hoja con las notas')).not.toBeInTheDocument()
    // Back leaves straight to home (no sheet step to return to).
    await user.click(screen.getByRole('button', { name: 'Atrás' }))
    expect(
      await screen.findByRole('heading', { name: 'Crear boletines de notas' }),
    ).toBeInTheDocument()
  })

  it('reads a CSV (no sheet list) directly', async () => {
    const api = createMockApi({ stepDelayMs: 0, filePicks: [MOCK_PATHS.csv] })
    const { user } = renderApp({ api })
    await user.click(screen.getByRole('button', { name: 'Elegir archivo de notas' }))
    expect(await screen.findByText('18 estudiantes seleccionados de 18')).toBeInTheDocument()
    const call = api.calls.find((c) => c.method === 'inspectGradebook')
    expect(call?.args).toEqual([MOCK_PATHS.csv, null])
  })

  it('does nothing when the file picker is cancelled', async () => {
    const api = createMockApi({ stepDelayMs: 0, filePicks: [null] })
    const { user } = renderApp({ api })
    await user.click(screen.getByRole('button', { name: 'Elegir archivo de notas' }))
    expect(screen.getByRole('heading', { name: 'Crear boletines de notas' })).toBeInTheDocument()
    expect(api.calls.some((c) => c.method === 'listSheets')).toBe(false)
  })

  it('opens a file dropped on the window and rejects an unsupported one', async () => {
    const api = createMockApi({ stepDelayMs: 0 })
    renderApp({ api })
    // The window subscription is made on mount.
    await screen.findByRole('heading', { name: 'Crear boletines de notas' })
    act(() => {
      api.emitFileDrop({ type: 'drop', paths: ['/x/documento.pdf'] })
    })
    expect(
      await screen.findByText(
        'Ese archivo no se puede usar. Elegir un archivo de Excel (.xlsx o .xls) o CSV.',
      ),
    ).toBeInTheDocument()
    act(() => {
      api.emitFileDrop({ type: 'drop', paths: [MOCK_PATHS.single] })
    })
    expect(await screen.findByRole('heading', { name: 'Paso 1 — Revisar' })).toBeInTheDocument()
  })
})

describe('Paso 1', () => {
  it('lets the person pick students, search, and shows why Continuar is disabled', async () => {
    const app = renderApp()
    const { user } = app
    await openFirstSheet(app)

    await user.click(screen.getByRole('button', { name: 'Quitar la selección' }))
    expect(screen.getByText('0 estudiantes seleccionados de 27')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Continuar' })).toBeDisabled()
    expect(screen.getByText('Elegir al menos un estudiante para continuar.')).toBeInTheDocument()

    await user.type(screen.getByRole('searchbox', { name: 'Buscar estudiante' }), 'martinez')
    const row = screen.getByRole('checkbox', { name: 'Seleccionar a MARTÍNEZ, Santiago' })
    await user.click(row)
    expect(screen.getByText('1 estudiante seleccionado de 27')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Continuar' })).toBeEnabled()
    expect(screen.queryByRole('checkbox', { name: 'Seleccionar a ÁLVAREZ, Camila' })).toBeNull()

    // The selection of rows hidden by the search is kept.
    await user.clear(screen.getByRole('searchbox', { name: 'Buscar estudiante' }))
    expect(screen.getByRole('checkbox', { name: 'Seleccionar a MARTÍNEZ, Santiago' })).toBeChecked()
    expect(
      screen.getByRole('checkbox', { name: 'Seleccionar a ÁLVAREZ, Camila' }),
    ).not.toBeChecked()

    await user.type(screen.getByRole('searchbox', { name: 'Buscar estudiante' }), 'zzzz')
    expect(screen.getByText('No se encontró nada con «zzzz».')).toBeInTheDocument()
  })

  it('shows "—" for the values the file does not have', async () => {
    const api = createMockApi({ stepDelayMs: 0, filePicks: [MOCK_PATHS.single] })
    const app = renderApp({ api })
    await app.user.click(screen.getByRole('button', { name: 'Elegir archivo de notas' }))
    await screen.findByRole('heading', { name: 'Paso 1 — Revisar' })
    // An empty metadata value renders the dash (checked through the helper's contract in the dl).
    const summary = screen.getByText('Materia').closest('dl') as HTMLElement
    expect(within(summary).getAllByText(/./).length).toBeGreaterThan(3)
  })

  it('"Ver ejemplo" creates one report for the first selected student and shows it', async () => {
    const app = renderApp()
    const { user, api } = app
    await openFirstSheet(app)
    await user.click(screen.getByRole('checkbox', { name: 'Seleccionar a ÁLVAREZ, Camila' }))
    await user.click(screen.getByRole('button', { name: 'Ver ejemplo' }))
    const dialog = await screen.findByRole('dialog', { name: 'Ejemplo de boletín' })
    expect(
      within(dialog).getByText('Ejemplo con los datos de BERNAL, Sebastián.'),
    ).toBeInTheDocument()
    await waitFor(() => {
      expect(within(dialog).getByTestId('viewer-doc')).toHaveTextContent('02_preview.pdf')
    })
    const preview = api.calls.find((c) => c.method === 'previewStudent')
    expect(preview?.args[1]).toBe(2)
    await user.keyboard('{Escape}')
    await waitFor(() => {
      expect(screen.queryByRole('dialog')).toBeNull()
    })
    // Esc closed the dialog only; the screen is still Paso 1.
    expect(screen.getByRole('heading', { name: 'Paso 1 — Revisar' })).toBeInTheDocument()
  })
})

describe('Paso 2', () => {
  it('uses the right plural, the chosen date and writes only the selected students', async () => {
    const app = renderApp()
    const { user, api } = app
    await openFirstSheet(app)
    await user.click(screen.getByRole('button', { name: 'Quitar la selección' }))
    await user.click(screen.getByRole('checkbox', { name: 'Seleccionar a GÓMEZ, Sofía' }))
    await continueToSave(app)

    const generate = await waitForGenerateEnabled('Generar 1 boletín')
    const date = screen.getByLabelText('Fecha del boletín')
    expect(date).toHaveValue(new Date().toLocaleDateString('en-CA'))
    fireEvent.change(date, { target: { value: '2026-12-01' } })
    expect(screen.getByText(/Se imprimirá como 01\/12\/2026\./)).toBeInTheDocument()

    fireEvent.change(date, { target: { value: '' } })
    expect(generate).toBeDisabled()
    expect(screen.getAllByText('Para generar, escribir una fecha válida.').length).toBeGreaterThan(
      0,
    )
    fireEvent.change(date, { target: { value: '2026-12-01' } })
    await user.click(generate)

    expect(await screen.findByText('Listo. Se creó 1 boletín.')).toBeInTheDocument()
    const call = api.calls.find((c) => c.method === 'generateReports')
    expect(call?.args[0]).toMatchObject({ students: [7], dateIso: '2026-12-01' })
  })

  it('changes the folder with the native picker and remembers the base folder', async () => {
    const api = createMockApi({ stepDelayMs: 0, folderPicks: ['/media/usb/Notas'] })
    const app = renderApp({ api })
    const { user, settings } = app
    await openFirstSheet(app)
    await continueToSave(app)
    await waitForGenerateEnabled('Generar 27 boletines')
    await user.click(screen.getByRole('button', { name: 'Cambiar carpeta…' }))
    await waitFor(() => {
      expect(
        screen.getByText('/media/usb/Notas/Francés IV – Grupo 2 – 2025-2', {
          selector: '.sr-only',
        }),
      ).toBeInTheDocument()
    })
    await user.click(await waitForGenerateEnabled('Generar 27 boletines'))
    await screen.findByText('Listo. Se crearon 27 boletines.')
    await settings.flush()
    expect(api.currentSettings()).toMatchObject({
      base_folder: '/media/usb/Notas',
      remember_base_folder: true,
    })
  })

  it('does not remember the base folder when the box is unchecked', async () => {
    const api = createMockApi({ stepDelayMs: 0, folderPicks: ['/media/usb/Notas'] })
    const app = renderApp({ api })
    const { user, settings } = app
    await openFirstSheet(app)
    await continueToSave(app)
    await user.click(screen.getByRole('button', { name: 'Cambiar carpeta…' }))
    await user.click(screen.getByRole('checkbox', { name: 'Recordar esta carpeta base' }))
    await user.click(await waitForGenerateEnabled('Generar 27 boletines'))
    await screen.findByText('Listo. Se crearon 27 boletines.')
    await settings.flush()
    expect(api.currentSettings().base_folder).toBeNull()
    expect(api.currentSettings().remember_base_folder).toBe(false)
  })

  describe('a folder that already has report cards', () => {
    it('asks before writing and replaces in the same folder on "Reemplazar"', async () => {
      const api = createMockApi({ stepDelayMs: 0, existingPdfs: { [FOLDER]: 27 } })
      const app = renderApp({ api })
      const { user } = app
      await openFirstSheet(app)
      await continueToSave(app)

      expect(await screen.findByText('Ya hay boletines en esta carpeta')).toBeInTheDocument()
      expect(screen.getByText('La carpeta ya tiene 27 boletines.')).toBeInTheDocument()
      const generate = screen.getByRole('button', { name: 'Generar 27 boletines' })
      expect(generate).toBeDisabled()
      expect(
        screen.getByText(
          'Para generar, elegir primero qué hacer con los boletines que ya existen.',
        ),
      ).toBeInTheDocument()
      // Nothing is written before the person chooses.
      expect(api.calls.some((c) => c.method === 'generateReports')).toBe(false)

      await user.click(screen.getByRole('radio', { name: /Reemplazar/ }))
      expect(generate).toBeEnabled()
      await user.click(generate)
      await screen.findByText('Listo. Se crearon 27 boletines.')
      const call = api.calls.find((c) => c.method === 'generateReports')
      expect(call?.args[0]).toMatchObject({ outputDir: FOLDER })
    })

    it('writes to a new folder on "Guardar en una carpeta nueva"', async () => {
      const api = createMockApi({ stepDelayMs: 0, existingPdfs: { [FOLDER]: 27 } })
      const app = renderApp({ api })
      const { user } = app
      await openFirstSheet(app)
      await continueToSave(app)
      await user.click(await screen.findByRole('radio', { name: /Guardar en una carpeta nueva/ }))
      expect(screen.getByText(`${FOLDER} (2)`, { selector: '.sr-only' })).toBeInTheDocument()
      await user.click(screen.getByRole('button', { name: 'Generar 27 boletines' }))
      await screen.findByText('Listo. Se crearon 27 boletines.')
      const call = api.calls.find((c) => c.method === 'generateReports')
      expect(call?.args[0]).toMatchObject({ outputDir: `${FOLDER} (2)` })
    })

    it('asks again when coming back after generating (the folder now has reports)', async () => {
      const app = renderApp()
      const { user } = app
      await openFirstSheet(app)
      await continueToSave(app)
      await user.click(await waitForGenerateEnabled('Generar 27 boletines'))
      await screen.findByText('Listo. Se crearon 27 boletines.')
      await user.click(screen.getByRole('button', { name: 'Atrás' }))
      expect(await screen.findByText('Ya hay boletines en esta carpeta')).toBeInTheDocument()
      expect(screen.getByRole('button', { name: 'Generar 27 boletines' })).toBeDisabled()
    })
  })

  it('goes back keeping every choice', async () => {
    const app = renderApp()
    const { user } = app
    await openFirstSheet(app)
    await user.click(screen.getByRole('button', { name: 'Quitar la selección' }))
    await user.click(screen.getByRole('checkbox', { name: 'Seleccionar a ÁLVAREZ, Camila' }))
    await continueToSave(app)
    fireEvent.change(screen.getByLabelText('Fecha del boletín'), {
      target: { value: '2026-12-01' },
    })
    await user.click(screen.getByRole('button', { name: 'Atrás' }))
    expect(await screen.findByText('1 estudiante seleccionado de 27')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Atrás' }))
    // Several sheets: back to the sheet choice, then the same sheet again.
    await user.click(
      await screen.findByRole('button', { name: 'Usar esta hoja: FRANCAIS IV LE MATIN' }),
    )
    expect(await screen.findByText('1 estudiante seleccionado de 27')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Continuar' }))
    expect(await screen.findByLabelText('Fecha del boletín')).toHaveValue('2026-12-01')
    // Choosing the other sheet loads it fresh.
    await user.click(screen.getByRole('button', { name: 'Atrás' }))
    await user.click(screen.getByRole('button', { name: 'Atrás' }))
    await user.click(
      await screen.findByRole('button', { name: 'Usar esta hoja: FRANCAIS IV LE SOIR' }),
    )
    expect(await screen.findByText('22 estudiantes seleccionados de 22')).toBeInTheDocument()
  })
})

describe('Generando', () => {
  it('shows real progress and lets the person cancel, keeping what was created', async () => {
    const api = createMockApi({ stepDelayMs: 0, holdAfter: 2 })
    const app = renderApp({ api })
    const { user } = app
    await openFirstSheet(app)
    await continueToSave(app)
    await user.click(await waitForGenerateEnabled('Generar 27 boletines'))

    expect(await screen.findByRole('heading', { name: 'Generando boletines' })).toBeInTheDocument()
    expect(await screen.findByText('2 de 27 — BERNAL, Sebastián')).toBeInTheDocument()
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '2')

    await user.click(screen.getByRole('button', { name: 'Cancelar' }))
    const dialog = await screen.findByRole('dialog', { name: '¿Cancelar la generación?' })
    // Focus starts on the safe option.
    expect(within(dialog).getByRole('button', { name: 'No, seguir generando' })).toHaveFocus()
    await user.click(within(dialog).getByRole('button', { name: 'Sí, cancelar' }))

    expect(
      await screen.findByText('Se canceló. Se conservan 2 boletines ya creados.'),
    ).toBeInTheDocument()
    expect(screen.getAllByRole('listitem').length).toBeGreaterThanOrEqual(2)
    expect(api.calls.some((c) => c.method === 'cancelGeneration')).toBe(true)
  })

  it('keeps generating when the person answers "No, seguir generando"', async () => {
    const api = createMockApi({ stepDelayMs: 0, holdAfter: 2 })
    const app = renderApp({ api })
    const { user } = app
    await openFirstSheet(app)
    await continueToSave(app)
    await user.click(await waitForGenerateEnabled('Generar 27 boletines'))
    await screen.findByText('2 de 27 — BERNAL, Sebastián')
    await user.click(screen.getByRole('button', { name: 'Cancelar' }))
    await user.click(await screen.findByRole('button', { name: 'No, seguir generando' }))
    expect(api.calls.some((c) => c.method === 'cancelGeneration')).toBe(false)
    api.release()
    expect(await screen.findByText('Listo. Se crearon 27 boletines.')).toBeInTheDocument()
  })

  it('lists a failing student at the end and retries only that one', async () => {
    const api = createMockApi({ stepDelayMs: 0, failStudents: [4] })
    const app = renderApp({ api })
    const { user } = app
    await openFirstSheet(app)
    await continueToSave(app)
    await user.click(await waitForGenerateEnabled('Generar 27 boletines'))

    expect(await screen.findByText('Listo. Se crearon 26 boletines.')).toBeInTheDocument()
    expect(screen.getByText('No se pudo crear 1 boletín')).toBeInTheDocument()
    expect(screen.getByText(/DÍAZ, Valentina: No se pudo crear un boletín/)).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Reintentar los que fallaron' }))
    expect(await screen.findByText('Listo. Se crearon 27 boletines.')).toBeInTheDocument()
    expect(screen.queryByText('No se pudo crear 1 boletín')).toBeNull()
    const retry = api.calls.filter((c) => c.method === 'generateReports')[1]
    expect(retry?.args[0]).toMatchObject({ students: [4], outputDir: FOLDER })
  })
})

describe('Resultados', () => {
  it('searches the list and navigates among the visible reports', async () => {
    const app = renderApp()
    const { user } = app
    await openFirstSheet(app)
    await continueToSave(app)
    await user.click(await waitForGenerateEnabled('Generar 27 boletines'))
    await screen.findByText('Listo. Se crearon 27 boletines.')

    await user.type(screen.getByRole('searchbox', { name: 'Buscar boletín' }), 'ospina')
    expect(screen.getAllByRole('button', { name: /OSPINA/ })).toHaveLength(1)
    await user.click(screen.getByRole('button', { name: /OSPINA/ }))
    await waitFor(() => {
      expect(screen.getByTestId('viewer-doc')).toHaveTextContent('OSPINA')
    })
    expect(screen.getByRole('button', { name: 'next' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'prev' })).toBeDisabled()
    await user.type(screen.getByRole('searchbox', { name: 'Buscar boletín' }), 'zz')
    expect(screen.getByText('Ningún boletín coincide con la búsqueda.')).toBeInTheDocument()
  })
})
