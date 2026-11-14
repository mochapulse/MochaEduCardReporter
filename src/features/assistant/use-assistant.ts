import { useEffect, useMemo, useReducer, useRef } from 'react'
import { appError, toAppError, type Api, type HistoryEntry, type SheetInfoView } from '@/lib/api'
import type { SettingsController } from '@/lib/preferences'
import {
  createInitialState,
  flowReducer,
  generateBlockReason,
  outputFolder,
  plannedFolder,
  selectedNumbers,
  usableSheets,
  type ExistingChoice,
  type FlowState,
  type StudentKey,
} from './state'

export interface AssistantActions {
  openFile: (path: string) => Promise<void>
  pickFile: () => Promise<void>
  chooseSheet: (name: string) => Promise<void>
  setSelection: (selected: ReadonlySet<StudentKey>) => void
  continueToSave: () => void
  back: () => void
  changeFolder: () => Promise<void>
  setRemember: (remember: boolean) => void
  setDate: (date: string) => void
  setExistingChoice: (choice: ExistingChoice) => void
  generate: () => Promise<void>
  retryFailures: () => Promise<void>
  cancelGeneration: () => Promise<void>
  openHistory: (entry: HistoryEntry) => void
  dismissError: () => void
  openSettings: () => void
  openHelp: () => void
  reset: () => void
}

function sheetErrors(sheets: readonly SheetInfoView[]): string | null {
  const lines = sheets.flatMap((sheet) => (sheet.error ? [`${sheet.name}: ${sheet.error}`] : []))
  return lines.length > 0 ? lines.join('\n') : null
}

/**
 * Connects the pure flow reducer to the outside world (the API and the
 * settings). All side effects of the assistant live here; screens only read
 * the state and call these actions.
 */
export function useAssistant(
  api: Api,
  settings: SettingsController,
  initial?: FlowState,
): { state: FlowState; actions: AssistantActions } {
  const [state, dispatch] = useReducer(flowReducer, initial, (seed) => seed ?? createInitialState())
  const stateRef = useRef(state)
  useEffect(() => {
    stateRef.current = state
  }, [state])

  // Step 2 needs a base folder: the remembered one, or the default.
  const needsBase = state.view === 'save' && state.save.base === null
  useEffect(() => {
    if (!needsBase) return
    let alive = true
    api
      .defaultOutputBase()
      .then((base) => {
        if (alive) {
          dispatch({
            type: 'BASE_RESOLVED',
            base,
            remember: settings.current().remember_base_folder ?? true,
          })
        }
      })
      .catch((error: unknown) => {
        if (alive) dispatch({ type: 'OPERATION_FAILED', error: toAppError(error) })
      })
    return () => {
      alive = false
    }
  }, [api, settings, needsBase])

  // ...and the check for PDFs already in the folder, BEFORE anything is written.
  const planned = state.view === 'save' ? plannedFolder(state) : null
  const checked = state.save.statusFor
  useEffect(() => {
    if (planned === null || checked === planned) return
    let alive = true
    api
      .checkOutputDir(planned)
      .then((status) => {
        if (alive) dispatch({ type: 'FOLDER_STATUS', path: planned, status })
      })
      .catch((error: unknown) => {
        if (alive) dispatch({ type: 'OPERATION_FAILED', error: toAppError(error) })
      })
    return () => {
      alive = false
    }
  }, [api, planned, checked])

  const actions = useMemo<AssistantActions>(() => {
    const fail = (error: unknown) => {
      dispatch({ type: 'OPERATION_FAILED', error: toAppError(error) })
    }

    const loadSheet = async (
      path: string,
      sheets: readonly SheetInfoView[],
      sheet: string | null,
    ) => {
      const gradebook = await api.inspectGradebook(path, sheet)
      dispatch({ type: 'GRADEBOOK_LOADED', source: { path, sheets }, gradebook })
      // The "recent files" list is a convenience: never let it break the flow.
      api.addRecentFile(path).catch(() => undefined)
    }

    const openFile = async (path: string) => {
      if (stateRef.current.busy) return
      dispatch({ type: 'OPEN_STARTED', path })
      try {
        const sheets = await api.listSheets(path)
        const usable = usableSheets(sheets)
        if (usable.length > 1) {
          dispatch({ type: 'SHEET_CHOICE_READY', source: { path, sheets: usable } })
          api.addRecentFile(path).catch(() => undefined)
        } else if (usable.length === 1 && usable[0]) {
          await loadSheet(path, usable, usable[0].name)
        } else if (sheets.length > 0) {
          throw appError('NO_GRADEBOOK_FOUND', 'no usable sheet in the file', sheetErrors(sheets))
        } else {
          // CSV files (and unreadable workbooks) have no sheet list: the engine reads the file itself.
          await loadSheet(path, [], null)
        }
      } catch (error) {
        fail(error)
      }
    }

    const run = async (mode: 'all' | 'retry') => {
      const current = stateRef.current
      const gradebook = current.gradebook
      if (!gradebook) return
      let students: number[] | null
      let outputDir: string | null
      if (mode === 'retry') {
        const failed = current.results?.failures ?? []
        students = failed.flatMap((f) => (f.student_number === null ? [] : [f.student_number]))
        outputDir = current.results?.outputDir ?? null
      } else {
        if (generateBlockReason(current) !== null) return
        students = selectedNumbers(current)
        outputDir = outputFolder(current)
      }
      if (outputDir === null) return
      const total =
        mode === 'retry' ? (students?.length ?? 0) : (students?.length ?? gradebook.students.length)
      if (mode === 'all') {
        // "Recordar esta carpeta base" is applied the moment generation starts.
        const base = current.save.base
        if (current.save.remember && base !== null) {
          settings.update({ base_folder: base, remember_base_folder: true })
        } else {
          settings.update({ remember_base_folder: current.save.remember })
        }
      }
      dispatch({ type: 'GENERATE_STARTED', mode, total })
      try {
        const result = await api.generateReports(
          {
            gradebookId: gradebook.id,
            students,
            outputDir,
            dateIso: current.save.date,
          },
          (event) => {
            dispatch({ type: 'PROGRESS', event })
          },
        )
        dispatch({ type: 'GENERATE_DONE', result })
      } catch (error) {
        dispatch({ type: 'GENERATE_FAILED', error: toAppError(error) })
      }
    }

    return {
      openFile,
      pickFile: async () => {
        try {
          const path = await api.pickGradebookFile()
          if (path !== null) await openFile(path)
        } catch (error) {
          fail(error)
        }
      },
      chooseSheet: async (name) => {
        const current = stateRef.current
        if (current.busy || !current.source) return
        if (current.gradebook?.sheet === name) {
          dispatch({ type: 'SHEET_CONFIRMED' })
          return
        }
        dispatch({ type: 'INSPECT_STARTED', sheet: name })
        try {
          await loadSheet(current.source.path, current.source.sheets, name)
        } catch (error) {
          fail(error)
        }
      },
      setSelection: (selected) => {
        dispatch({ type: 'SET_SELECTION', selected })
      },
      continueToSave: () => {
        dispatch({ type: 'CONTINUE' })
      },
      back: () => {
        dispatch({ type: 'BACK' })
      },
      changeFolder: async () => {
        try {
          const picked = await api.pickFolder(stateRef.current.save.base)
          if (picked !== null) dispatch({ type: 'SET_BASE', base: picked })
        } catch (error) {
          fail(error)
        }
      },
      setRemember: (remember) => {
        dispatch({ type: 'SET_REMEMBER', remember })
      },
      setDate: (date) => {
        dispatch({ type: 'SET_DATE', date })
      },
      setExistingChoice: (choice) => {
        dispatch({ type: 'SET_EXISTING_CHOICE', choice })
      },
      generate: () => run('all'),
      retryFailures: () => run('retry'),
      cancelGeneration: async () => {
        dispatch({ type: 'CANCEL_REQUESTED' })
        try {
          await api.cancelGeneration()
        } catch (error) {
          fail(error)
        }
      },
      openHistory: (entry) => {
        dispatch({ type: 'OPEN_HISTORY', entry })
      },
      dismissError: () => {
        dispatch({ type: 'DISMISS_ERROR' })
      },
      openSettings: () => {
        dispatch({ type: 'OPEN_SETTINGS' })
      },
      openHelp: () => {
        dispatch({ type: 'OPEN_HELP' })
      },
      reset: () => {
        dispatch({ type: 'RESET' })
      },
    }
  }, [api, settings])

  return { state, actions }
}
