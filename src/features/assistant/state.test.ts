import { describe, expect, it } from 'vitest'
import type { GenerateResult, GradebookView, HistoryEntry, OutputDirStatus } from '@/lib/api'
import { appError } from '@/lib/api'
import {
  createInitialState,
  flowReducer,
  generateBlockReason,
  outputFolder,
  plannedFolder,
  selectedNumbers,
  type FlowAction,
  type FlowState,
} from './state'

const sheets = [
  { name: 'A', student_count: 3, error: null },
  { name: 'B', student_count: 2, error: null },
]
const source = { path: '/n/notas.xls', sheets }

function gradebook(sheet = 'A', count = 3): GradebookView {
  return {
    id: `gb-${sheet}`,
    source_path: source.path,
    source_file_name: 'notas.xls',
    sheet,
    metadata: {
      university: '',
      program: '',
      subject: 'Francés IV',
      group: '2',
      professor: 'P',
      period: '2025-2',
    },
    students: Array.from({ length: count }, (_, i) => ({
      number: i + 1,
      name: `EST ${String(i + 1)}`,
      code: `c${String(i + 1)}`,
      absences: '0',
      definitive_grade: '4.0',
      first_twenty_grades: [],
      component_grades: [],
      weighted_components: [],
    })),
    warnings: [],
    suggested_folder_name: 'Francés IV – Grupo 2 – 2025-2',
  }
}

function run(state: FlowState, ...actions: FlowAction[]): FlowState {
  return actions.reduce(flowReducer, state)
}

const freeStatus: OutputDirStatus = {
  exists: false,
  writable: true,
  pdf_count: 0,
  alternative_path: '/b/x (2)',
  alternative_name: 'x (2)',
}

function atSave(): FlowState {
  return run(
    createInitialState('2026-11-14'),
    { type: 'GRADEBOOK_LOADED', source, gradebook: gradebook() },
    { type: 'CONTINUE' },
    { type: 'BASE_RESOLVED', base: '/b', remember: true },
  )
}

describe('flow state machine', () => {
  it('starts at home with nothing chosen and today as the date', () => {
    const state = createInitialState('2026-11-14')
    expect(state.view).toBe('home')
    expect(state.save.date).toBe('2026-11-14')
    expect(state.selected.size).toBe(0)
  })

  it('shows the sheet choice for several usable sheets, with nothing selected yet', () => {
    const state = run(createInitialState(), { type: 'OPEN_STARTED', path: source.path })
    expect(state.busy).toEqual({ kind: 'opening', path: source.path })
    const ready = flowReducer(state, { type: 'SHEET_CHOICE_READY', source })
    expect(ready).toMatchObject({ view: 'review', reviewStep: 'sheet', busy: null })
  })

  it('loads a gradebook with every student selected and the suggested folder name', () => {
    const state = run(createInitialState(), {
      type: 'GRADEBOOK_LOADED',
      source,
      gradebook: gradebook(),
    })
    expect(state).toMatchObject({ view: 'review', reviewStep: 'students' })
    expect([...state.selected]).toEqual(['1', '2', '3'])
    expect(state.save.folderName).toBe('Francés IV – Grupo 2 – 2025-2')
  })

  it('does not continue without students, and continues with some', () => {
    const loaded = run(createInitialState(), {
      type: 'GRADEBOOK_LOADED',
      source,
      gradebook: gradebook(),
    })
    const none = run(loaded, { type: 'SET_SELECTION', selected: new Set() }, { type: 'CONTINUE' })
    expect(none.view).toBe('review')
    const some = run(
      loaded,
      { type: 'SET_SELECTION', selected: new Set(['2']) },
      { type: 'CONTINUE' },
    )
    expect(some.view).toBe('save')
  })

  it('goes back step by step without losing choices', () => {
    let state = atSave()
    state = run(
      state,
      { type: 'SET_SELECTION', selected: new Set(['1', '3']) },
      { type: 'SET_BASE', base: '/otra' },
      { type: 'SET_DATE', date: '2026-12-01' },
      { type: 'SET_REMEMBER', remember: false },
    )
    expect(plannedFolder(state)).toBe('/otra/Francés IV – Grupo 2 – 2025-2')
    state = flowReducer(state, { type: 'BACK' })
    expect(state).toMatchObject({ view: 'review', reviewStep: 'students' })
    // Several sheets: one more step back shows the sheet choice, keeping the gradebook.
    state = flowReducer(state, { type: 'BACK' })
    expect(state).toMatchObject({ view: 'review', reviewStep: 'sheet' })
    expect(state.gradebook?.id).toBe('gb-A')
    expect([...state.selected]).toEqual(['1', '3'])
    // Choosing the same sheet again keeps the selection.
    state = flowReducer(state, { type: 'SHEET_CONFIRMED' })
    state = flowReducer(state, { type: 'CONTINUE' })
    expect(state.view).toBe('save')
    expect(state.save).toMatchObject({ base: '/otra', date: '2026-12-01', remember: false })
    expect([...state.selected]).toEqual(['1', '3'])
  })

  it('goes home from the sheet choice and, with one sheet, from the student review', () => {
    const choosing = run(createInitialState(), { type: 'SHEET_CHOICE_READY', source })
    expect(flowReducer(choosing, { type: 'BACK' }).view).toBe('home')
    const single = run(createInitialState(), {
      type: 'GRADEBOOK_LOADED',
      source: { path: '/n/a.csv', sheets: [] },
      gradebook: gradebook(),
    })
    expect(flowReducer(single, { type: 'BACK' }).view).toBe('home')
  })

  it('ignores Back and Settings while a file is being read', () => {
    const busy = run(createInitialState(), { type: 'OPEN_STARTED', path: '/x' })
    expect(flowReducer(busy, { type: 'OPEN_SETTINGS' }).view).toBe('home')
    const inspecting = run(
      createInitialState(),
      { type: 'SHEET_CHOICE_READY', source },
      { type: 'INSPECT_STARTED', sheet: 'A' },
    )
    expect(flowReducer(inspecting, { type: 'BACK' }).view).toBe('review')
  })

  it('keeps an error until it is dismissed or the flow moves on', () => {
    const error = appError('FILE_LOCKED', 'locked')
    const failed = run(
      createInitialState(),
      { type: 'OPEN_STARTED', path: '/x' },
      { type: 'OPERATION_FAILED', error },
    )
    expect(failed).toMatchObject({ view: 'home', busy: null })
    expect(failed.error?.code).toBe('FILE_LOCKED')
    expect(flowReducer(failed, { type: 'DISMISS_ERROR' }).error).toBeNull()
    expect(flowReducer(failed, { type: 'OPEN_STARTED', path: '/y' }).error).toBeNull()
  })

  it('opens Ajustes / Ayuda from home and returns to where it was', () => {
    const settings = run(createInitialState(), { type: 'OPEN_SETTINGS' })
    expect(settings.view).toBe('settings')
    expect(flowReducer(settings, { type: 'BACK' }).view).toBe('home')
    const help = run(createInitialState(), { type: 'OPEN_HELP' })
    expect(flowReducer(help, { type: 'BACK' }).view).toBe('home')
  })

  describe('step 2 rules', () => {
    it('blocks generation until the folder was checked', () => {
      const state = atSave()
      expect(generateBlockReason(state)).toBe('checking')
      const checked = flowReducer(state, {
        type: 'FOLDER_STATUS',
        path: plannedFolder(state) ?? '',
        status: freeStatus,
      })
      expect(generateBlockReason(checked)).toBeNull()
    })

    it('ignores a folder check that answers for an old folder', () => {
      const state = atSave()
      const stale = flowReducer(state, {
        type: 'FOLDER_STATUS',
        path: '/otra/carpeta',
        status: freeStatus,
      })
      expect(stale.save.status).toBeNull()
    })

    it('asks what to do when the folder already has PDFs, and writes to the alternative on "new"', () => {
      const state = atSave()
      const path = plannedFolder(state) ?? ''
      const existing = flowReducer(state, {
        type: 'FOLDER_STATUS',
        path,
        status: { ...freeStatus, exists: true, pdf_count: 5 },
      })
      expect(generateBlockReason(existing)).toBe('chooseExisting')
      const replace = flowReducer(existing, { type: 'SET_EXISTING_CHOICE', choice: 'replace' })
      expect(generateBlockReason(replace)).toBeNull()
      expect(outputFolder(replace)).toBe(path)
      const fresh = flowReducer(existing, { type: 'SET_EXISTING_CHOICE', choice: 'new' })
      expect(outputFolder(fresh)).toBe('/b/x (2)')
    })

    it('forgets the choice when the folder changes', () => {
      const state = atSave()
      const path = plannedFolder(state) ?? ''
      const chosen = run(
        state,
        { type: 'FOLDER_STATUS', path, status: { ...freeStatus, pdf_count: 2 } },
        { type: 'SET_EXISTING_CHOICE', choice: 'replace' },
        { type: 'SET_BASE', base: '/otra' },
      )
      expect(chosen.save.existing).toBeNull()
      expect(chosen.save.status).toBeNull()
    })

    it('blocks an invalid date and a folder that cannot be written', () => {
      const state = atSave()
      const path = plannedFolder(state) ?? ''
      const checked = flowReducer(state, {
        type: 'FOLDER_STATUS',
        path,
        status: { ...freeStatus, writable: false },
      })
      expect(generateBlockReason(checked)).toBe('notWritable')
      const badDate = run(checked, { type: 'SET_DATE', date: '' })
      expect(generateBlockReason(badDate)).toBe('badDate')
    })

    it('sends null for "everybody" and numbers for a subset', () => {
      const state = atSave()
      expect(selectedNumbers(state)).toBeNull()
      const subset = flowReducer(state, { type: 'SET_SELECTION', selected: new Set(['3', '1']) })
      expect(selectedNumbers(subset)).toEqual([1, 3])
    })
  })

  describe('generation', () => {
    const result = (overrides: Partial<GenerateResult> = {}): GenerateResult => ({
      output_dir: '/b/x',
      files: [
        { student_number: 1, student_name: 'EST 1', path: '/b/x/01.pdf' },
        { student_number: 3, student_name: 'EST 3', path: '/b/x/03.pdf' },
      ],
      failures: [{ student_number: 2, student_name: 'EST 2', message: 'boom', code: 'PDF_FAILED' }],
      cancelled: false,
      history_id: 'h1',
      ...overrides,
    })

    it('tracks progress, files and failures live', () => {
      let state = run(atSave(), { type: 'GENERATE_STARTED', mode: 'all', total: 3 })
      expect(state.view).toBe('generating')
      state = run(
        state,
        {
          type: 'PROGRESS',
          event: {
            done: 1,
            total: 3,
            student_number: 1,
            student_name: 'EST 1',
            pdf_path: '/b/x/01.pdf',
            error: null,
          },
        },
        {
          type: 'PROGRESS',
          event: {
            done: 2,
            total: 3,
            student_number: 2,
            student_name: 'EST 2',
            pdf_path: null,
            error: 'boom',
          },
        },
      )
      expect(state.run).toMatchObject({ done: 2, total: 3, currentName: 'EST 2' })
      expect(state.run?.files).toHaveLength(1)
      expect(state.run?.failures).toHaveLength(1)
    })

    it('finishes in results, clearing the folder check so Back re-checks it', () => {
      const state = run(
        atSave(),
        { type: 'GENERATE_STARTED', mode: 'all', total: 3 },
        { type: 'GENERATE_DONE', result: result() },
      )
      expect(state.view).toBe('results')
      expect(state.run).toBeNull()
      expect(state.results).toMatchObject({ origin: 'generated', requested: 3, cancelled: false })
      expect(state.save.statusFor).toBeNull()
      const back = flowReducer(state, { type: 'BACK' })
      expect(back.view).toBe('save')
      expect(back.save.date).toBe(state.save.date)
    })

    it('marks a cancelled run and keeps the reports already created', () => {
      const cancelling = run(
        atSave(),
        { type: 'GENERATE_STARTED', mode: 'all', total: 3 },
        { type: 'CANCEL_REQUESTED' },
      )
      expect(cancelling.run?.cancelRequested).toBe(true)
      const done = flowReducer(cancelling, {
        type: 'GENERATE_DONE',
        result: result({ cancelled: true, failures: [] }),
      })
      expect(done.results).toMatchObject({ cancelled: true })
      expect(done.results?.files).toHaveLength(2)
    })

    it('returns to step 2 with the error when generation itself fails', () => {
      const state = run(
        atSave(),
        { type: 'GENERATE_STARTED', mode: 'all', total: 3 },
        { type: 'GENERATE_FAILED', error: appError('OUTPUT_NOT_WRITABLE', 'denied') },
      )
      expect(state).toMatchObject({ view: 'save', run: null })
      expect(state.error?.code).toBe('OUTPUT_NOT_WRITABLE')
      expect(state.selected.size).toBe(3)
    })

    it('retries only the failures and merges the new reports in student order', () => {
      const first = run(
        atSave(),
        { type: 'GENERATE_STARTED', mode: 'all', total: 3 },
        { type: 'GENERATE_DONE', result: result() },
      )
      const retrying = flowReducer(first, { type: 'GENERATE_STARTED', mode: 'retry', total: 1 })
      expect(retrying.run?.baseFiles).toHaveLength(2)
      const done = flowReducer(retrying, {
        type: 'GENERATE_DONE',
        result: result({
          files: [{ student_number: 2, student_name: 'EST 2', path: '/b/x/02.pdf' }],
          failures: [],
        }),
      })
      expect(done.results?.files.map((f) => f.student_number)).toEqual([1, 2, 3])
      expect(done.results?.failures).toEqual([])
      expect(done.results?.requested).toBe(3)
    })

    it('stays in results when the retry itself fails', () => {
      const first = run(
        atSave(),
        { type: 'GENERATE_STARTED', mode: 'all', total: 3 },
        { type: 'GENERATE_DONE', result: result() },
        { type: 'GENERATE_STARTED', mode: 'retry', total: 1 },
        { type: 'GENERATE_FAILED', error: appError('INTERNAL', 'x') },
      )
      expect(first.view).toBe('results')
      expect(first.results?.files).toHaveLength(2)
      expect(first.error?.code).toBe('INTERNAL')
    })

    it('"Generar otros boletines" goes home but keeps the base folder choice', () => {
      const done = run(
        atSave(),
        { type: 'GENERATE_STARTED', mode: 'all', total: 3 },
        { type: 'GENERATE_DONE', result: result() },
        { type: 'RESET' },
      )
      expect(done).toMatchObject({ view: 'home', results: null, gradebook: null })
      expect(done.save.base).toBe('/b')
    })
  })

  describe('history', () => {
    const entry: HistoryEntry = {
      id: 'h9',
      created_at: '2026-11-12T09:41:00',
      report_date: '2026-11-12',
      source_file_name: 'n.xls',
      source_path: '/n/n.xls',
      sheet: null,
      subject: 'Francés I',
      group: '1',
      period: '2026-1',
      output_folder: '/b/h',
      pdf_files: ['/b/h/07_MARTINEZ_Ana.pdf', '/b/h/08_PEREZ_Julian.pdf'],
      generated_count: 2,
      failed_count: 0,
    }

    it('reopens an entry in results and goes home on Back', () => {
      const state = run(createInitialState(), { type: 'OPEN_HISTORY', entry })
      expect(state.view).toBe('results')
      expect(state.results).toMatchObject({ origin: 'history', outputDir: '/b/h', historyId: 'h9' })
      expect(state.results?.files).toEqual([
        { student_number: 7, student_name: 'MARTINEZ Ana', path: '/b/h/07_MARTINEZ_Ana.pdf' },
        { student_number: 8, student_name: 'PEREZ Julian', path: '/b/h/08_PEREZ_Julian.pdf' },
      ])
      expect(flowReducer(state, { type: 'BACK' }).view).toBe('home')
    })
  })
})
