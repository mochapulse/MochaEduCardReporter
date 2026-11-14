import { errorCodes } from '@/i18n/errors'
import type { AppError, AppErrorCode } from './types'

function isKnownCode(value: unknown): value is AppErrorCode {
  return typeof value === 'string' && (errorCodes as readonly string[]).includes(value)
}

/** An {@link AppError} that can be thrown (the backend rejects with a plain object of this shape). */
export class AppFailure extends Error implements AppError {
  readonly code: AppErrorCode
  readonly detail: string | null

  constructor(code: AppErrorCode, message: string, detail: string | null = null) {
    super(message)
    this.name = 'AppFailure'
    this.code = code
    this.detail = detail
  }
}

/** Builds an {@link AppError} that can also be thrown. */
export function appError(
  code: AppErrorCode,
  message: string,
  detail: string | null = null,
): AppFailure {
  return new AppFailure(code, message, detail)
}

/** True for objects shaped like the backend's `AppError`. */
export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === 'object' &&
    value !== null &&
    'code' in value &&
    'message' in value &&
    typeof value.code === 'string' &&
    typeof value.message === 'string'
  )
}

/**
 * Normalizes anything a rejected promise can carry into an {@link AppError}.
 * Unknown codes become `INTERNAL` (the original code is kept in the detail).
 */
export function toAppError(error: unknown): AppError {
  if (isAppError(error)) {
    if (isKnownCode(error.code)) {
      return { code: error.code, message: error.message, detail: error.detail }
    }
    const unknownCode: string = error.code
    return appError(
      'INTERNAL',
      error.message,
      [`code: ${unknownCode}`, error.detail].filter(Boolean).join('\n'),
    )
  }
  if (error instanceof Error) return appError('INTERNAL', error.message, error.stack ?? null)
  if (typeof error === 'string') return appError('INTERNAL', error)
  return appError('INTERNAL', 'Unknown error', safeStringify(error))
}

function safeStringify(value: unknown): string | null {
  try {
    return JSON.stringify(value)
  } catch {
    return null
  }
}
