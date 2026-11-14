import { readdirSync, readFileSync, statSync } from 'node:fs'
import path from 'node:path'
import { describe, expect, it } from 'vitest'

const here = import.meta.dirname

function walk(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const full = path.join(dir, entry)
    return statSync(full).isDirectory() ? walk(full) : [full]
  })
}

const textFile = /\.(tsx?|css|html|json|md)$/

describe('offline guarantee', () => {
  it('has no http(s) URL anywhere in the viewer folder', () => {
    const remote = new RegExp(String.raw`https?:` + '//', 'i')
    const offenders = walk(here)
      .filter((file) => textFile.test(file) && file !== import.meta.filename)
      .filter((file) => remote.test(readFileSync(file, 'utf8')))
      .map((file) => path.relative(here, file))
    expect(offenders).toEqual([])
  })

  it('ships the standard font files of the installed pdf.js, locally', () => {
    const root = path.resolve(here, '../../..')
    const installed = path.join(root, 'node_modules/pdfjs-dist/standard_fonts')
    const shipped = path.join(root, 'public/pdfjs/standard_fonts')
    for (const file of readdirSync(installed)) {
      expect(statSync(path.join(shipped, file)).size, file).toBe(
        statSync(path.join(installed, file)).size,
      )
    }
  })
})
