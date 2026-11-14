import type {
  AppInfo,
  GenerateResult,
  GradebookView,
  HistoryEntry,
  NewHistoryEntry,
  OutputDirStatus,
  ProgressEvent,
  RecentFile,
  Settings,
  SheetInfoView,
} from '@/lib/bindings'

export type {
  AppError,
  AppErrorCode,
  AppInfo,
  FailureView,
  GenerateResult,
  GeneratedFile,
  GradebookView,
  HistoryEntry,
  MetadataView,
  NewHistoryEntry,
  OutputDirStatus,
  ProgressEvent,
  RecentFile,
  Settings,
  SheetInfoView,
  StudentView,
  TextSize,
  Theme,
} from '@/lib/bindings'

/** What the window reports while a file is dragged over it or dropped on it. */
export type FileDropEvent =
  | { readonly type: 'hover' }
  | { readonly type: 'leave' }
  | { readonly type: 'drop'; readonly paths: readonly string[] }

/** A newer version announced by the update feed. */
export interface UpdateInfo {
  readonly version: string
  /** Release notes, when the feed has them. */
  readonly notes: string | null
}

export interface GenerateRequest {
  readonly gradebookId: string
  /** Student numbers, or null for everybody. */
  readonly students: readonly number[] | null
  readonly outputDir: string
  /** `YYYY-MM-DD`. */
  readonly dateIso: string
}

/**
 * Everything the screens need from the outside world: the Rust commands plus
 * the native pickers and window integration. Every method rejects with an
 * {@link AppError}.
 */
export interface Api {
  /** `tauri` inside the desktop app, `mock` in a browser and in tests. */
  readonly kind: 'tauri' | 'mock'

  listSheets(path: string): Promise<SheetInfoView[]>
  inspectGradebook(path: string, sheet: string | null): Promise<GradebookView>
  previewStudent(gradebookId: string, studentNumber: number, dateIso: string): Promise<string>
  generateReports(
    request: GenerateRequest,
    onProgress: (event: ProgressEvent) => void,
  ): Promise<GenerateResult>
  cancelGeneration(): Promise<void>
  checkOutputDir(path: string): Promise<OutputDirStatus>
  defaultOutputBase(): Promise<string>
  readPdf(path: string): Promise<ArrayBuffer>
  getSettings(): Promise<Settings>
  saveSettings(settings: Settings): Promise<void>
  getRecentFiles(): Promise<RecentFile[]>
  addRecentFile(path: string): Promise<void>
  getHistory(): Promise<HistoryEntry[]>
  addHistoryEntry(entry: NewHistoryEntry): Promise<HistoryEntry>
  removeHistoryEntry(id: string): Promise<void>
  revealInFolder(path: string): Promise<void>
  openExternally(path: string): Promise<void>
  openLogDir(): Promise<void>
  appInfo(): Promise<AppInfo>
  /** Reads the signed update feed; null when this version is the latest one. */
  checkForUpdate(): Promise<UpdateInfo | null>
  /** Downloads the available update, installs it and restarts the app. */
  installUpdate(): Promise<void>

  /** Native "open file" dialog; null when the person cancels. */
  pickGradebookFile(): Promise<string | null>
  /** Native "choose folder" dialog; null when the person cancels. */
  pickFolder(defaultPath: string | null): Promise<string | null>
  /** Subscribes to files dragged onto the window. Returns the unsubscribe function. */
  onFileDrop(handler: (event: FileDropEvent) => void): Promise<() => void>
  /** Sets the title of the native window (and of the document). */
  setWindowTitle(title: string): Promise<void>
}
