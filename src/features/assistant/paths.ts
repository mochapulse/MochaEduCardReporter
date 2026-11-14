/** Joins a folder and a name with the separator the folder already uses. */
export function joinPath(base: string, name: string): string {
  const separator = base.includes('\\') && !base.includes('/') ? '\\' : '/'
  return `${base.replace(/[\\/]+$/, '')}${separator}${name}`
}

/** Last component of a path. */
export function baseName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path
}

/** True when the file name has an extension the app can read. */
export function isSupportedGradebook(path: string): boolean {
  return /\.(xlsx|xls|csv)$/i.test(path)
}

/**
 * Number and name of a student from a generated file name such as
 * `07_MARTINEZ_Ana.pdf` (used when a history entry is reopened).
 */
export function studentFromFileName(path: string): { number: number | null; name: string } {
  const stem = baseName(path).replace(/\.pdf$/i, '')
  const match = /^(\d+)_(.*)$/.exec(stem)
  const rawName = match?.[2] ?? stem
  return {
    number: match?.[1] ? Number(match[1]) : null,
    name: rawName.replace(/_+/g, ' ').trim(),
  }
}
