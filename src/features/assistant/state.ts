import type {
  AppError,
  FailureView,
  GeneratedFile,
  GenerateResult,
  GradebookView,
  HistoryEntry,
  OutputDirStatus,
  ProgressEvent,
  SheetInfoView,
  StudentView,
} from '@/lib/api'
import { isValidIsoDate, todayIso } from './dates'
import { joinPath, studentFromFileName } from './paths'

export type View = 'home' | 'review' | 'save' | 'generating' | 'results' | 'settings' | 'help'
export type ReviewStep = 'sheet' | 'students'
export type ExistingChoice = 'replace' | 'new'

/** Key of a student in the selection set. */
export type StudentKey = string

export function studentKey(student: StudentView, index: number): StudentKey {
  return student.number === null ? `row-${String(index)}` : String(student.number)
}

export interface SourceFile {
  readonly path: string
  /** Usable sheets of the file. */
  readonly sheets: readonly SheetInfoView[]
}

export type Busy =
  | { readonly kind: 'opening'; readonly path: string }
  | { readonly kind: 'inspecting'; readonly sheet: string }

export interface SaveChoices {
  /** Base folder; null until the default has been resolved. */
  readonly base: string | null
  /** Folder name below the base (suggested from the file). */
  readonly folderName: string
  readonly remember: boolean
  /** `YYYY-MM-DD`. */
  readonly date: string
  /** What to do when the folder already has PDFs. */
  readonly existing: ExistingChoice | null
  /** Result of `checkOutputDir` for `statusFor`. */
  readonly status: OutputDirStatus | null
  readonly statusFor: string | null
}

export interface RunProgress {
  readonly mode: 'all' | 'retry'
  readonly done: number
  readonly total: number
  readonly currentName: string | null
  /** Files already created before this run (retry only). */
  readonly baseFiles: readonly GeneratedFile[]
  readonly files: readonly GeneratedFile[]
  readonly failures: readonly FailureView[]
  readonly cancelRequested: boolean
}

export interface ResultsState {
  readonly origin: 'generated' | 'history'
  readonly outputDir: string
  readonly files: readonly GeneratedFile[]
  readonly failures: readonly FailureView[]
  readonly cancelled: boolean
  readonly historyId: string | null
  readonly subject: string
  readonly group: string
  /** Number of students the run was asked to handle (generated origin only). */
  readonly requested: number
}

export interface FlowState {
  readonly view: View
  /** Where "Atrás" in Ajustes / Ayuda leads. */
  readonly returnView: View
  readonly busy: Busy | null
  readonly error: AppError | null
  readonly source: SourceFile | null
  readonly reviewStep: ReviewStep
  readonly gradebook: GradebookView | null
  readonly selected: ReadonlySet<StudentKey>
  readonly save: SaveChoices
  readonly run: RunProgress | null
  readonly results: ResultsState | null
}

export function createInitialSave(today: string = todayIso()): SaveChoices {
  return {
    base: null,
    folderName: '',
    remember: true,
    date: today,
    existing: null,
    status: null,
    statusFor: null,
  }
}

export function createInitialState(today: string = todayIso()): FlowState {
  return {
    view: 'home',
    returnView: 'home',
    busy: null,
    error: null,
    source: null,
    reviewStep: 'students',
    gradebook: null,
    selected: new Set(),
    save: createInitialSave(today),
    run: null,
    results: null,
  }
}

export type FlowAction =
  | { type: 'OPEN_STARTED'; path: string }
  | { type: 'SHEET_CHOICE_READY'; source: SourceFile }
  | { type: 'GRADEBOOK_LOADED'; source: SourceFile; gradebook: GradebookView }
  | { type: 'SHEET_CONFIRMED' }
  | { type: 'INSPECT_STARTED'; sheet: string }
  | { type: 'OPERATION_FAILED'; error: AppError }
  | { type: 'DISMISS_ERROR' }
  | { type: 'SET_SELECTION'; selected: ReadonlySet<StudentKey> }
  | { type: 'CONTINUE' }
  | { type: 'BACK' }
  | { type: 'BASE_RESOLVED'; base: string; remember: boolean }
  | { type: 'SET_BASE'; base: string }
  | { type: 'SET_REMEMBER'; remember: boolean }
  | { type: 'SET_DATE'; date: string }
  | { type: 'SET_EXISTING_CHOICE'; choice: ExistingChoice }
  | { type: 'FOLDER_STATUS'; path: string; status: OutputDirStatus }
  | { type: 'GENERATE_STARTED'; mode: 'all' | 'retry'; total: number }
  | { type: 'PROGRESS'; event: ProgressEvent }
  | { type: 'CANCEL_REQUESTED' }
  | { type: 'GENERATE_DONE'; result: GenerateResult }
  | { type: 'GENERATE_FAILED'; error: AppError }
  | { type: 'OPEN_HISTORY'; entry: HistoryEntry }
  | { type: 'OPEN_SETTINGS' }
  | { type: 'OPEN_HELP' }
  | { type: 'RESET' }

/** Usable sheets (the ones the engine could inspect). */
export function usableSheets(sheets: readonly SheetInfoView[]): readonly SheetInfoView[] {
  return sheets.filter((sheet) => sheet.error === null)
}

function byNumber(a: GeneratedFile, b: GeneratedFile): number {
  return (
    (a.student_number ?? Number.MAX_SAFE_INTEGER) - (b.student_number ?? Number.MAX_SAFE_INTEGER)
  )
}

export function flowReducer(state: FlowState, action: FlowAction): FlowState {
  switch (action.type) {
    case 'OPEN_STARTED':
      return { ...state, busy: { kind: 'opening', path: action.path }, error: null }

    case 'SHEET_CHOICE_READY':
      // A new file: nothing from the previous one is kept, but the save choices stay.
      return {
        ...state,
        view: 'review',
        reviewStep: 'sheet',
        busy: null,
        error: null,
        source: action.source,
        gradebook: null,
        selected: new Set(),
        run: null,
        results: null,
        save: { ...state.save, existing: null, status: null, statusFor: null, folderName: '' },
      }

    case 'GRADEBOOK_LOADED': {
      const allKeys = action.gradebook.students.map(studentKey)
      return {
        ...state,
        view: 'review',
        reviewStep: 'students',
        busy: null,
        error: null,
        source: action.source,
        gradebook: action.gradebook,
        selected: new Set(allKeys),
        run: null,
        results: null,
        save: {
          ...state.save,
          folderName: action.gradebook.suggested_folder_name,
          existing: null,
          status: null,
          statusFor: null,
        },
      }
    }

    case 'SHEET_CONFIRMED':
      return { ...state, reviewStep: 'students', error: null }

    case 'INSPECT_STARTED':
      return { ...state, busy: { kind: 'inspecting', sheet: action.sheet }, error: null }

    case 'OPERATION_FAILED':
      return { ...state, busy: null, error: action.error }

    case 'DISMISS_ERROR':
      return { ...state, error: null }

    case 'SET_SELECTION':
      return { ...state, selected: action.selected }

    case 'CONTINUE':
      if (state.view !== 'review' || state.reviewStep !== 'students' || state.selected.size === 0) {
        return state
      }
      return { ...state, view: 'save', error: null }

    case 'BACK':
      // A file is still being read: leaving now would be undone when it arrives.
      return state.busy ? state : back(state)

    case 'BASE_RESOLVED':
      if (state.save.base !== null) return state
      return { ...state, save: { ...state.save, base: action.base, remember: action.remember } }

    case 'SET_BASE':
      return {
        ...state,
        save: { ...state.save, base: action.base, existing: null, status: null, statusFor: null },
      }

    case 'SET_REMEMBER':
      return { ...state, save: { ...state.save, remember: action.remember } }

    case 'SET_DATE':
      return { ...state, save: { ...state.save, date: action.date } }

    case 'SET_EXISTING_CHOICE':
      return { ...state, save: { ...state.save, existing: action.choice } }

    case 'FOLDER_STATUS': {
      if (action.path !== plannedFolder(state)) return state // stale answer
      const hasPdfs = action.status.pdf_count > 0
      return {
        ...state,
        save: {
          ...state.save,
          status: action.status,
          statusFor: action.path,
          existing: hasPdfs ? state.save.existing : null,
        },
      }
    }

    case 'GENERATE_STARTED':
      return {
        ...state,
        view: 'generating',
        error: null,
        run: {
          mode: action.mode,
          done: 0,
          total: action.total,
          currentName: null,
          baseFiles: action.mode === 'retry' ? (state.results?.files ?? []) : [],
          files: [],
          failures: [],
          cancelRequested: false,
        },
      }

    case 'PROGRESS': {
      if (!state.run) return state
      const { event } = action
      const files =
        event.pdf_path === null
          ? state.run.files
          : [
              ...state.run.files,
              {
                student_number: event.student_number,
                student_name: event.student_name,
                path: event.pdf_path,
              },
            ]
      const failures =
        event.error === null
          ? state.run.failures
          : [
              ...state.run.failures,
              {
                student_number: event.student_number,
                student_name: event.student_name,
                message: event.error,
                code: 'PDF_FAILED' as const,
              },
            ]
      return {
        ...state,
        run: {
          ...state.run,
          done: event.done,
          total: event.total,
          currentName: event.student_name,
          files,
          failures,
        },
      }
    }

    case 'CANCEL_REQUESTED':
      return state.run ? { ...state, run: { ...state.run, cancelRequested: true } } : state

    case 'GENERATE_DONE': {
      // The folder now holds PDFs: when the person comes back, it is checked again.
      const save = { ...state.save, existing: null, status: null, statusFor: null }
      const baseFiles = state.run?.baseFiles ?? []
      const files = [...baseFiles, ...action.result.files].sort(byNumber)
      return {
        ...state,
        save,
        view: 'results',
        run: null,
        error: null,
        results: {
          origin: 'generated',
          outputDir: action.result.output_dir,
          files,
          failures: action.result.failures,
          cancelled: action.result.cancelled,
          historyId: action.result.history_id ?? state.results?.historyId ?? null,
          subject: state.gradebook?.metadata.subject ?? '',
          group: state.gradebook?.metadata.group ?? '',
          requested:
            state.run?.mode === 'retry' ? (state.results?.requested ?? 0) : (state.run?.total ?? 0),
        },
      }
    }

    case 'GENERATE_FAILED': {
      const wasRetry = state.run?.mode === 'retry'
      return {
        ...state,
        view: wasRetry ? 'results' : 'save',
        run: null,
        error: action.error,
      }
    }

    case 'OPEN_HISTORY': {
      const { entry } = action
      const files: GeneratedFile[] = entry.pdf_files.map((path) => {
        const parsed = studentFromFileName(path)
        return { student_number: parsed.number, student_name: parsed.name, path }
      })
      return {
        ...state,
        view: 'results',
        error: null,
        busy: null,
        run: null,
        results: {
          origin: 'history',
          outputDir: entry.output_folder,
          files,
          failures: [],
          cancelled: false,
          historyId: entry.id,
          subject: entry.subject,
          group: entry.group,
          requested: files.length,
        },
      }
    }

    case 'OPEN_SETTINGS':
      if (state.busy) return state
      return { ...state, view: 'settings', returnView: state.view, error: null }

    case 'OPEN_HELP':
      if (state.busy) return state
      return { ...state, view: 'help', returnView: state.view, error: null }

    case 'RESET':
      return {
        ...createInitialState(),
        save: { ...createInitialSave(), base: state.save.base, remember: state.save.remember },
      }
  }
}

function back(state: FlowState): FlowState {
  switch (state.view) {
    case 'review': {
      const multiple = state.source ? usableSheets(state.source.sheets).length > 1 : false
      if (state.reviewStep === 'students' && multiple) {
        return { ...state, reviewStep: 'sheet', error: null }
      }
      return { ...state, view: 'home', error: null }
    }
    case 'save':
      return { ...state, view: 'review', reviewStep: 'students', error: null }
    case 'results':
      return state.results?.origin === 'history'
        ? { ...state, view: 'home', error: null }
        : { ...state, view: 'save', error: null }
    case 'settings':
    case 'help':
      return { ...state, view: state.returnView, error: null }
    case 'home':
    case 'generating':
      return state
  }
}

/** The folder the reports would be written to, ignoring the "new folder" choice. */
export function plannedFolder(state: FlowState): string | null {
  const { base, folderName } = state.save
  if (base === null || folderName === '') return null
  return joinPath(base, folderName)
}

/** The folder generation will write to (honors the "new folder" choice). */
export function outputFolder(state: FlowState): string | null {
  const planned = plannedFolder(state)
  if (planned === null) return null
  const { existing, status, statusFor } = state.save
  if (existing === 'new' && status !== null && statusFor === planned) return status.alternative_path
  return planned
}

export type BlockReason = 'noFolder' | 'checking' | 'chooseExisting' | 'notWritable' | 'badDate'

/** Why "Generar" is disabled, or null when it can run. */
export function generateBlockReason(state: FlowState): BlockReason | null {
  const planned = plannedFolder(state)
  if (planned === null) return 'noFolder'
  if (!isValidIsoDate(state.save.date)) return 'badDate'
  if (state.save.statusFor !== planned || state.save.status === null) return 'checking'
  if (!state.save.status.writable) return 'notWritable'
  if (state.save.status.pdf_count > 0 && state.save.existing === null) return 'chooseExisting'
  return null
}

/** Numbers of the selected students (null for "everybody"). */
export function selectedNumbers(state: FlowState): number[] | null {
  const students = state.gradebook?.students ?? []
  const picked = students.filter((student, index) => state.selected.has(studentKey(student, index)))
  if (picked.length === students.length) return null
  return picked.flatMap((student) => (student.number === null ? [] : [student.number]))
}
