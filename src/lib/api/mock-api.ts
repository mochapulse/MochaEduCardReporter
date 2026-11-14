import {
  buildGradebook,
  EMPTY_METADATA,
  FRENCH_I,
  FRENCH_IV_EVENING,
  FRENCH_IV_MORNING,
  type MockSheetSpec,
} from './mock-data'
import { appError } from './errors'
import { buildSamplePdf } from './sample-pdf'
import type {
  Api,
  AppError,
  AppInfo,
  FailureView,
  FileDropEvent,
  GenerateRequest,
  GenerateResult,
  GeneratedFile,
  GradebookView,
  HistoryEntry,
  NewHistoryEntry,
  OutputDirStatus,
  ProgressEvent,
  RecentFile,
  Settings,
  SheetInfoView,
  UpdateInfo,
} from './types'

/** A file the mock "disk" knows about. */
export interface MockFileSpec {
  /** Sheets of the file. For CSV a single sheet. */
  readonly sheets?: readonly MockSheetSpec[]
  /** Makes `listSheets` / `inspectGradebook` reject with this error. */
  readonly error?: AppError
}

export const MOCK_BASE = '/home/profesora/Documentos/Boletines'
const NOTES = '/home/profesora/Documentos/Notas'

export const MOCK_PATHS = {
  workbook: `${NOTES}/NOTAS FRANCÉS 2025-2.xls`,
  single: `${NOTES}/CARNET DE NOTES FRANÇAIS IV 2025.xlsx`,
  csv: `${NOTES}/Francés I.csv`,
  locked: `${NOTES}/En uso.xlsx`,
  empty: `${NOTES}/Vacío.xlsx`,
  missing: `${NOTES}/Borrado.xlsx`,
} as const

const defaultFiles: Readonly<Record<string, MockFileSpec>> = {
  [MOCK_PATHS.workbook]: {
    sheets: [
      {
        name: 'FRANCAIS IV LE MATIN',
        students: 27,
        metadata: FRENCH_IV_MORNING,
        warnings: [
          'Student enumeration corrected from row order: GÓMEZ, Andrés: 8 -> 7; PÉREZ, Julián: 7 -> 8; RODRÍGUEZ, Lucía: 12 -> 11',
        ],
      },
      { name: 'FRANCAIS IV LE SOIR', students: 22, metadata: FRENCH_IV_EVENING },
    ],
  },
  [MOCK_PATHS.single]: {
    sheets: [{ name: 'Carnet', students: 27, metadata: FRENCH_IV_MORNING }],
  },
  [MOCK_PATHS.csv]: { sheets: [{ name: 'Francés I', students: 18, metadata: FRENCH_I }] },
  [MOCK_PATHS.locked]: {
    error: appError('FILE_LOCKED', 'cannot open file', 'os error 32: file is in use'),
  },
  [MOCK_PATHS.empty]: {
    sheets: [{ name: 'Hoja1', students: 0, metadata: EMPTY_METADATA }],
    error: appError('EMPTY_SHEET', 'the sheet has no students', 'sheet "Hoja1"'),
  },
}

export interface MockApiOptions {
  /** Pause between students during generation (ms). Default 120. */
  stepDelayMs?: number
  /** Latency of every other call (ms). Default 0. */
  latencyMs?: number
  /** Extra or replacement files, by path. */
  files?: Readonly<Record<string, MockFileSpec>>
  /** Calls that always reject with this error. */
  failCalls?: Partial<Record<keyof Api, AppError>>
  /** Students (by number) whose first report fails. */
  failStudents?: readonly number[]
  /** When true, the students above fail on every attempt, not just the first. */
  failStudentsAlways?: boolean
  /** Folders that already hold PDFs: path to count. */
  existingPdfs?: Readonly<Record<string, number>>
  settings?: Settings
  recentFiles?: readonly RecentFile[]
  history?: readonly HistoryEntry[]
  /** Answers of `pickGradebookFile`, in order (then the default workbook). */
  filePicks?: readonly (string | null)[]
  /** Answers of `pickFolder`, in order (then null). */
  folderPicks?: readonly (string | null)[]
  /** Generation waits (until `release()` or cancel) once this many students are done. */
  holdAfter?: number
  version?: string
  /** Version the update feed announces, or none. */
  updateAvailable?: string
}

export interface MockApi extends Api {
  /** Every call made, in order (method name and arguments). */
  readonly calls: { readonly method: string; readonly args: readonly unknown[] }[]
  /** Pretends the window received a file drag/drop event. */
  emitFileDrop(event: FileDropEvent): void
  /** Lets a held generation continue. */
  release(): void
  /** Last title passed to `setWindowTitle`. */
  readonly windowTitle: () => string
  /** Current in-memory settings. */
  readonly currentSettings: () => Settings
}

/** Configured errors are plain data; throwing needs an Error. */
function rethrowable(error: AppError): Error {
  return appError(error.code, error.message, error.detail)
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms)
  })
}

function sanitize(name: string): string {
  return name.replace(/[^\p{L}\p{N}]+/gu, '_').replace(/^_+|_+$/g, '')
}

function joinPath(base: string, name: string): string {
  const separator = base.includes('\\') && !base.includes('/') ? '\\' : '/'
  return `${base.replace(/[\\/]+$/, '')}${separator}${name}`
}

function localIso(date = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${String(date.getFullYear())}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
}

/**
 * In-memory implementation of {@link Api}: realistic data, progress with
 * delays, cancellation and configurable failures. Used in the browser dev
 * server (no Tauri window) and injected in tests.
 */
export function createMockApi(options: MockApiOptions = {}): MockApi {
  const stepDelayMs = options.stepDelayMs ?? 120
  const latencyMs = options.latencyMs ?? 0
  const files: Record<string, MockFileSpec> = { ...defaultFiles, ...options.files }
  const existingPdfs: Record<string, number> = { ...options.existingPdfs }
  let settings: Settings = options.settings ?? {
    base_folder: null,
    remember_base_folder: true,
    text_size: 'normal',
    theme: 'claro',
  }
  let recents: RecentFile[] = [...(options.recentFiles ?? [])]
  let history: HistoryEntry[] = [...(options.history ?? [])]
  const filePicks = [...(options.filePicks ?? [])]
  const folderPicks = [...(options.folderPicks ?? [])]
  const gradebooks = new Map<string, { view: GradebookView; seed: number }>()
  const attempted = new Set<number>()
  const calls: MockApi['calls'] = []
  const dropHandlers = new Set<(event: FileDropEvent) => void>()
  let cancelRequested = false
  const isCancelled = () => cancelRequested
  let gate: { promise: Promise<void>; open: () => void } | null = null
  let held = false
  let nextId = 1
  let title = ''

  const record = (method: string, ...args: unknown[]) => {
    calls.push({ method, args })
  }
  const guard = async (method: keyof Api): Promise<void> => {
    if (latencyMs > 0) await sleep(latencyMs)
    else await Promise.resolve()
    const failure = options.failCalls?.[method]
    if (failure) throw rethrowable(failure)
  }
  const fileSpec = (path: string): MockFileSpec => {
    const spec = files[path]
    if (!spec) throw appError('FILE_NOT_FOUND', 'file not found', path)
    return spec
  }
  const isExcel = (path: string) => /\.xlsx?$/i.test(path)

  const api: MockApi = {
    kind: 'mock',
    calls,
    emitFileDrop: (event) => {
      for (const handler of dropHandlers) handler(event)
    },
    release: () => {
      gate?.open()
    },
    windowTitle: () => title,
    currentSettings: () => settings,

    async listSheets(path) {
      record('listSheets', path)
      await guard('listSheets')
      const spec = fileSpec(path)
      if (spec.error?.code === 'FILE_LOCKED') throw rethrowable(spec.error)
      // Like the engine: a CSV (or an unreadable workbook) has no sheet list.
      if (!isExcel(path) || spec.error) return []
      return (spec.sheets ?? []).map((sheet): SheetInfoView => ({
        name: sheet.name,
        student_count: sheet.students,
        error: null,
      }))
    },

    async inspectGradebook(path, sheet) {
      record('inspectGradebook', path, sheet)
      await guard('inspectGradebook')
      const spec = fileSpec(path)
      if (spec.error) throw rethrowable(spec.error)
      const sheets = spec.sheets ?? []
      const index = sheet === null ? 0 : sheets.findIndex((candidate) => candidate.name === sheet)
      const chosen = sheets[index]
      if (!chosen) throw appError('NO_GRADEBOOK_FOUND', 'no gradebook found', path)
      const id = `gb-${String(nextId)}`
      nextId += 1
      const view = buildGradebook(id, path, chosen, isExcel(path), index)
      gradebooks.set(id, { view, seed: index })
      return view
    },

    async previewStudent(gradebookId, studentNumber, dateIso) {
      record('previewStudent', gradebookId, studentNumber, dateIso)
      await guard('previewStudent')
      if (!gradebooks.has(gradebookId)) throw appError('INTERNAL', 'unknown gradebook id')
      return `/tmp/mocha-preview/${String(studentNumber).padStart(2, '0')}_preview.pdf`
    },

    async generateReports(request: GenerateRequest, onProgress) {
      record('generateReports', request)
      await guard('generateReports')
      const book = gradebooks.get(request.gradebookId)
      if (!book) throw appError('INTERNAL', 'unknown gradebook id')
      cancelRequested = false
      held = false
      const wanted = request.students
      const students = book.view.students.filter(
        (student) =>
          wanted === null || (student.number !== null && wanted.includes(student.number)),
      )
      if (students.length === 0) throw appError('NO_GRADEBOOK_FOUND', 'nothing to generate')
      const written: GeneratedFile[] = []
      const failures: FailureView[] = []
      let cancelled = false
      for (const [index, student] of students.entries()) {
        if (stepDelayMs > 0) await sleep(stepDelayMs)
        else await Promise.resolve()
        if (options.holdAfter !== undefined && index === options.holdAfter && !held) {
          held = true
          let open: () => void = () => undefined
          const promise = new Promise<void>((resolve) => {
            open = resolve
          })
          gate = { promise, open }
          await Promise.race([
            gate.promise,
            (async () => {
              while (!isCancelled()) await sleep(5)
            })(),
          ])
        }
        if (isCancelled()) {
          cancelled = true
          break
        }
        const number = student.number
        const shouldFail =
          number !== null &&
          options.failStudents?.includes(number) === true &&
          (options.failStudentsAlways === true || !attempted.has(number))
        if (number !== null) attempted.add(number)
        let event: ProgressEvent
        if (shouldFail) {
          failures.push({
            student_number: number,
            student_name: student.name,
            message: 'could not lay out the report',
            code: 'PDF_FAILED',
          })
          event = {
            done: index + 1,
            total: students.length,
            student_number: number,
            student_name: student.name,
            pdf_path: null,
            error: 'could not lay out the report',
          }
        } else {
          const path = joinPath(
            request.outputDir,
            `${String(number ?? index + 1).padStart(2, '0')}_${sanitize(student.name)}.pdf`,
          )
          written.push({ student_number: number, student_name: student.name, path })
          event = {
            done: index + 1,
            total: students.length,
            student_number: number,
            student_name: student.name,
            pdf_path: path,
            error: null,
          }
        }
        onProgress(event)
      }
      existingPdfs[request.outputDir] = (existingPdfs[request.outputDir] ?? 0) + written.length
      let historyId: string | null = null
      if (written.length > 0) {
        const entry: HistoryEntry = {
          id: `h-${String(nextId)}`,
          created_at: localIso(),
          report_date: request.dateIso,
          source_file_name: book.view.source_file_name,
          source_path: book.view.source_path,
          sheet: book.view.sheet,
          subject: book.view.metadata.subject,
          group: book.view.metadata.group,
          period: book.view.metadata.period,
          output_folder: request.outputDir,
          pdf_files: written.map((file) => file.path),
          generated_count: written.length,
          failed_count: failures.length,
          folder_exists: true,
        }
        nextId += 1
        history = [entry, ...history]
        historyId = entry.id
      }
      return {
        output_dir: request.outputDir,
        files: written,
        failures,
        cancelled,
        history_id: historyId,
      } satisfies GenerateResult
    },

    async cancelGeneration() {
      record('cancelGeneration')
      cancelRequested = true
      await Promise.resolve()
    },

    async checkOutputDir(path): Promise<OutputDirStatus> {
      record('checkOutputDir', path)
      await guard('checkOutputDir')
      const count = existingPdfs[path] ?? 0
      let n = 2
      while ((existingPdfs[`${path} (${String(n)})`] ?? 0) > 0) n += 1
      const alternative = `${path} (${String(n)})`
      return {
        exists: count > 0,
        writable: true,
        pdf_count: count,
        alternative_path: alternative,
        alternative_name: alternative.split(/[\\/]/).pop() ?? alternative,
      }
    },

    async defaultOutputBase() {
      record('defaultOutputBase')
      await guard('defaultOutputBase')
      return settings.base_folder ?? MOCK_BASE
    },

    async readPdf(path) {
      record('readPdf', path)
      await guard('readPdf')
      const name = path.split(/[\\/]/).pop() ?? path
      return buildSamplePdf(name)
    },

    async getSettings() {
      record('getSettings')
      await guard('getSettings')
      return settings
    },
    async saveSettings(next) {
      record('saveSettings', next)
      await guard('saveSettings')
      settings = next
    },

    async getRecentFiles() {
      record('getRecentFiles')
      await guard('getRecentFiles')
      return recents.map((file) => ({ ...file, exists: file.path in files }))
    },
    async addRecentFile(path) {
      record('addRecentFile', path)
      await guard('addRecentFile')
      const file_name = path.split(/[\\/]/).pop() ?? path
      recents = [
        { path, file_name, last_used: localIso(), exists: true },
        ...recents.filter((file) => file.path !== path),
      ].slice(0, 5)
    },

    async getHistory() {
      record('getHistory')
      await guard('getHistory')
      return history
    },
    async addHistoryEntry(entry: NewHistoryEntry) {
      record('addHistoryEntry', entry)
      await guard('addHistoryEntry')
      const created: HistoryEntry = {
        ...entry,
        id: `h-${String(nextId)}`,
        created_at: localIso(),
        generated_count: entry.pdf_files.length,
        folder_exists: true,
      }
      nextId += 1
      history = [created, ...history]
      return created
    },
    async removeHistoryEntry(id) {
      record('removeHistoryEntry', id)
      await guard('removeHistoryEntry')
      history = history.filter((entry) => entry.id !== id)
    },

    async revealInFolder(path) {
      record('revealInFolder', path)
      await guard('revealInFolder')
    },
    async openExternally(path) {
      record('openExternally', path)
      await guard('openExternally')
    },
    async openLogDir() {
      record('openLogDir')
      await guard('openLogDir')
    },
    async appInfo(): Promise<AppInfo> {
      record('appInfo')
      await guard('appInfo')
      return {
        version: options.version ?? '1.0.0',
        engine_version: '1.0.0',
        log_dir: '/home/profesora/.local/share/boletines/logs',
      }
    },
    async checkForUpdate(): Promise<UpdateInfo | null> {
      record('checkForUpdate')
      await guard('checkForUpdate')
      if (!options.updateAvailable) return null
      return { version: options.updateAvailable, notes: null }
    },
    async installUpdate(): Promise<void> {
      record('installUpdate')
      await guard('installUpdate')
      await sleep(stepDelayMs)
    },

    async pickGradebookFile() {
      record('pickGradebookFile')
      await guard('pickGradebookFile')
      return filePicks.length > 0 ? (filePicks.shift() ?? null) : MOCK_PATHS.workbook
    },
    async pickFolder(defaultPath) {
      record('pickFolder', defaultPath)
      await guard('pickFolder')
      return folderPicks.length > 0 ? (folderPicks.shift() ?? null) : null
    },
    onFileDrop(handler) {
      dropHandlers.add(handler)
      return Promise.resolve(() => {
        dropHandlers.delete(handler)
      })
    },
    setWindowTitle(next) {
      title = next
      document.title = next
      return Promise.resolve()
    },
  }
  return api
}
