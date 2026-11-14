/**
 * Development scenarios: `?demo=<name>` opens a screen directly, in a given
 * state, with the in-memory mock API. Optional `size=` (normal|grande|muy-grande)
 * and `theme=` (claro|oscuro|sistema). Without `?demo` the app runs normally,
 * against the real backend. Never part of a production build.
 */
import {
  createInitialState,
  flowReducer,
  type FlowAction,
  type FlowState,
} from '@/features/assistant/state'
import type { AssistantActions } from '@/features/assistant/use-assistant'
import {
  appError,
  createMockApi,
  MOCK_BASE,
  MOCK_PATHS,
  type HistoryEntry,
  type MockApi,
  type MockApiOptions,
  type RecentFile,
  type Settings,
} from '@/lib/api'

export interface Scenario {
  api: MockApi
  initial?: FlowState
  autoplay?: (actions: AssistantActions) => void
}

const RECENTS: RecentFile[] = [
  {
    path: MOCK_PATHS.workbook,
    file_name: 'NOTAS FRANCÉS 2025-2.xls',
    last_used: '2026-11-12T09:30:00',
    exists: true,
  },
  {
    path: MOCK_PATHS.csv,
    file_name: 'Francés I.csv',
    last_used: '2026-09-02T16:05:00',
    exists: true,
  },
  {
    path: MOCK_PATHS.missing,
    file_name: 'Borrado.xlsx',
    last_used: '2026-06-20T11:00:00',
    exists: false,
  },
]

function historyEntry(id: string, overrides: Partial<HistoryEntry>): HistoryEntry {
  return {
    id,
    created_at: '2026-11-12T09:41:00',
    report_date: '2026-11-12',
    source_file_name: 'NOTAS FRANCÉS 2025-2.xls',
    source_path: MOCK_PATHS.workbook,
    sheet: 'FRANCAIS IV LE MATIN',
    subject: 'Francés IV',
    group: '2',
    period: '2025-2',
    output_folder: `${MOCK_BASE}/Francés IV – Grupo 2 – 2025-2`,
    pdf_files: [
      `${MOCK_BASE}/Francés IV – Grupo 2 – 2025-2/01_ALVAREZ_Camila.pdf`,
      `${MOCK_BASE}/Francés IV – Grupo 2 – 2025-2/02_BERNAL_Andres.pdf`,
      `${MOCK_BASE}/Francés IV – Grupo 2 – 2025-2/03_CASTRO_Ana.pdf`,
    ],
    generated_count: 27,
    failed_count: 0,
    folder_exists: true,
    ...overrides,
  }
}

const HISTORY: HistoryEntry[] = [
  historyEntry('h-1', {}),
  historyEntry('h-2', {
    created_at: '2026-06-18T17:20:00',
    subject: 'Francés I',
    group: '1',
    period: '2026-1',
    generated_count: 18,
  }),
  historyEntry('h-3', {
    created_at: '2025-12-01T10:00:00',
    subject: 'Francés III',
    group: '4',
    period: '2025-2',
    generated_count: 22,
    folder_exists: false,
  }),
]

function apply(state: FlowState, ...actions: FlowAction[]): FlowState {
  return actions.reduce(flowReducer, state)
}

export async function createScenario(params: URLSearchParams): Promise<Scenario | null> {
  // No `?demo` means a normal development run: the real Tauri backend.
  if (!params.has('demo')) return null
  const demo = params.get('demo') ?? 'home'
  const size = params.get('size')
  const theme = params.get('theme')
  const settings: Settings = {
    base_folder: null,
    remember_base_folder: true,
    text_size: size === 'grande' || size === 'muy-grande' ? size : 'normal',
    theme: theme === 'oscuro' || theme === 'sistema' ? theme : 'claro',
  }
  const options: MockApiOptions = {
    settings,
    recentFiles: RECENTS,
    history: HISTORY,
    stepDelayMs: 200,
  }
  if (demo === 'existing') {
    options.existingPdfs = { [`${MOCK_BASE}/Francés IV – Grupo 2 – 2025-2`]: 27 }
  }
  if (demo === 'failures') options.failStudents = [4, 11]
  if (demo === 'generating') options.stepDelayMs = 1500
  if (demo === 'results' || demo === 'failures') options.stepDelayMs = 0

  const api = createMockApi(options)
  if (['home', 'settings', 'help'].includes(demo)) {
    const initial = createInitialState()
    if (demo === 'settings') return { api, initial: apply(initial, { type: 'OPEN_SETTINGS' }) }
    if (demo === 'help') return { api, initial: apply(initial, { type: 'OPEN_HELP' }) }
    return { api }
  }
  if (demo === 'error') {
    return {
      api,
      initial: apply(createInitialState(), {
        type: 'OPERATION_FAILED',
        error: appError('FILE_LOCKED', 'cannot open file', 'os error 32: file is in use'),
      }),
    }
  }

  const path = demo === 'single' ? MOCK_PATHS.single : MOCK_PATHS.workbook
  const sheets = await api.listSheets(path)
  let state = createInitialState()
  if (demo === 'sheets') {
    return {
      api,
      initial: apply(state, { type: 'SHEET_CHOICE_READY', source: { path, sheets } }),
    }
  }
  const sheet = sheets[0]?.name ?? null
  const gradebook = await api.inspectGradebook(path, sheet)
  state = apply(state, {
    type: 'GRADEBOOK_LOADED',
    source: { path, sheets },
    gradebook,
  })
  if (demo === 'review' || demo === 'single') return { api, initial: state }
  if (demo === 'review-none') {
    return { api, initial: apply(state, { type: 'SET_SELECTION', selected: new Set() }) }
  }
  state = apply(state, { type: 'CONTINUE' })
  if (demo === 'save' || demo === 'existing') return { api, initial: state }
  if (demo === 'generating') {
    return {
      api,
      initial: state,
      autoplay: (actions) => {
        // Wait for the folder check, then press "Generar".
        setTimeout(() => void actions.generate(), 300)
      },
    }
  }
  if (demo === 'results' || demo === 'failures') {
    const base = await api.defaultOutputBase()
    const result = await api.generateReports(
      {
        gradebookId: gradebook.id,
        students: null,
        outputDir: `${base}/${gradebook.suggested_folder_name}`,
        dateIso: state.save.date,
      },
      () => undefined,
    )
    return {
      api,
      initial: apply(
        state,
        { type: 'GENERATE_STARTED', mode: 'all', total: gradebook.students.length },
        { type: 'GENERATE_DONE', result },
      ),
    }
  }
  return { api, initial: state }
}
