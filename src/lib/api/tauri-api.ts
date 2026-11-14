import { Channel } from '@tauri-apps/api/core'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-dialog'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { t } from '@/i18n/es'
import { commands, type ProgressEvent } from '@/lib/bindings'
import { appError, toAppError } from './errors'
import type { Api } from './types'

/** Runs a command and normalizes its rejection into an `AppError`. */
async function call<T>(promise: Promise<T>): Promise<T> {
  try {
    return await promise
  } catch (error) {
    const normalized = toAppError(error)
    throw appError(normalized.code, normalized.message, normalized.detail)
  }
}

const GRADEBOOK_EXTENSIONS = ['xlsx', 'xls', 'csv']

/** The real implementation: typed Tauri commands, native pickers and webview drag and drop. */
export function createTauriApi(): Api {
  // Kept between check and install: the plugin's Update carries a resource id.
  let pendingUpdate: Awaited<ReturnType<typeof check>> = null
  return {
    kind: 'tauri',
    listSheets: (path) => call(commands.listSheets(path)),
    inspectGradebook: (path, sheet) => call(commands.inspectGradebook(path, sheet)),
    previewStudent: (id, number, dateIso) => call(commands.previewStudent(id, number, dateIso)),
    generateReports: (request, onProgress) => {
      const channel = new Channel<ProgressEvent>()
      channel.onmessage = onProgress
      return call(
        commands.generateReports(
          request.gradebookId,
          request.students ? [...request.students] : null,
          request.outputDir,
          request.dateIso,
          channel,
        ),
      )
    },
    cancelGeneration: () => call(commands.cancelGeneration()),
    checkOutputDir: (path) => call(commands.checkOutputDir(path)),
    defaultOutputBase: () => call(commands.defaultOutputBase()),
    readPdf: (path) => call(commands.readPdf(path)),
    getSettings: () => call(commands.getSettings()),
    saveSettings: async (settings) => {
      await call(commands.saveSettings(settings))
    },
    getRecentFiles: () => call(commands.getRecentFiles()),
    addRecentFile: async (path) => {
      await call(commands.addRecentFile(path))
    },
    getHistory: () => call(commands.getHistory()),
    addHistoryEntry: (entry) => call(commands.addHistoryEntry(entry)),
    removeHistoryEntry: async (id) => {
      await call(commands.removeHistoryEntry(id))
    },
    revealInFolder: async (path) => {
      await call(commands.revealInFolder(path))
    },
    openExternally: async (path) => {
      await call(commands.openExternally(path))
    },
    openLogDir: async () => {
      await call(commands.openLogDir())
    },
    appInfo: () => call(commands.appInfo()),
    checkForUpdate: async () => {
      const update = await call(check())
      pendingUpdate = update
      if (!update) return null
      return { version: update.version, notes: update.body ?? null }
    },
    installUpdate: async () => {
      const update = pendingUpdate ?? (await call(check()))
      if (!update) {
        throw appError('INTERNAL', 'no update available to install')
      }
      await call(update.downloadAndInstall())
      await relaunch()
    },

    pickGradebookFile: async () =>
      call(
        open({
          multiple: false,
          directory: false,
          title: t('picker.chooseFile'),
          filters: [{ name: t('picker.filterName'), extensions: GRADEBOOK_EXTENSIONS }],
        }),
      ),
    pickFolder: async (defaultPath) =>
      call(
        open({
          multiple: false,
          directory: true,
          title: t('picker.chooseFolder'),
          ...(defaultPath ? { defaultPath } : {}),
        }),
      ),
    // In a webview, HTML5 drops do not carry file paths; Tauri's own drag-drop
    // event (enabled by default on the window) does.
    onFileDrop: (handler) =>
      call(
        getCurrentWebview().onDragDropEvent((event) => {
          const payload = event.payload
          if (payload.type === 'enter') handler({ type: 'hover' })
          else if (payload.type === 'leave') handler({ type: 'leave' })
          else if (payload.type === 'drop') handler({ type: 'drop', paths: payload.paths })
        }),
      ),
    setWindowTitle: async (title) => {
      document.title = title
      try {
        await getCurrentWindow().setTitle(title)
      } catch {
        // The window title is cosmetic; the capability may not grant it.
      }
    },
  }
}
