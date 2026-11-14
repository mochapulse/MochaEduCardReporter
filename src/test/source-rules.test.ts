/**
 * Source rules enforced on the UI code (everything under src/ except tests, the
 * i18n dictionary and test helpers).
 *
 * 1. No hardcoded colors: no hex literals and no rgb()/hsl()/oklch() functions in
 *    .ts/.tsx/.css files, except `styles/tokens.css` and `styles/tokens.ts`,
 *    which define the tokens. Components must use token-based Tailwind classes.
 *
 * 2. No hardcoded user-visible text. Heuristics (parsed with the TypeScript AST,
 *    so comments and imports are ignored):
 *    - any JSX text node that contains a letter is a violation (all text must
 *      come from `t()` / `es.*`);
 *    - string literals in user-facing JSX attributes (aria-label, title,
 *      placeholder, alt, label, description, caption, aria-description) that
 *      contain a letter are violations;
 *    - any other string/template literal that "looks Spanish" is a violation:
 *      it contains an accented vowel, "ñ", "¿" or "¡", or two or more common
 *      Spanish function words (el, la, de, para, con, que, ...).
 *    Fixture data in gallery/sample-data.ts (student names) is exempt.
 *    Known limit: an unaccented one-word Spanish literal outside those
 *    attributes (e.g. `const x = 'Aceptar'`) is not detected; code review covers it.
 */
import { readdirSync, readFileSync, statSync } from 'node:fs'
import path from 'node:path'
import ts from 'typescript'
import { describe, expect, it } from 'vitest'

const srcDir = path.resolve(import.meta.dirname, '..')

function walk(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const full = path.join(dir, entry)
    return statSync(full).isDirectory() ? walk(full) : [full]
  })
}

const rel = (file: string) => path.relative(srcDir, file).split(path.sep).join('/')
const allFiles = walk(srcDir).filter((file) => /\.(tsx?|css)$/.test(file))
const isTest = (file: string) => /\.test\.tsx?$/.test(file)

const colorFiles = allFiles.filter(
  (file) => !isTest(file) && !['styles/tokens.css', 'styles/tokens.ts'].includes(rel(file)),
)
const textFiles = allFiles.filter(
  (file) =>
    /\.tsx?$/.test(file) &&
    !isTest(file) &&
    !rel(file).startsWith('i18n/') &&
    !rel(file).startsWith('test/') &&
    rel(file) !== 'gallery/sample-data.ts' && // fixture names, not UI text
    rel(file) !== 'lib/api/mock-data.ts' && // mock student names, not UI text
    rel(file) !== 'lib/api/mock-api.ts' && // mock file paths with French accents
    rel(file) !== 'app/dev-scenarios.ts' && // dev scenario fixture data
    !rel(file).startsWith('components/pdf-viewer/fake-'), // fake engine test data
)

const colorPattern = /#[0-9a-fA-F]{3,8}\b|\b(?:rgba?|hsla?|oklch|oklab|lab|lch)\(/g

function stripCssAndTsComments(source: string): string {
  return source.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|[^:])\/\/.*$/gm, '$1')
}

const userFacingAttributes = new Set([
  'aria-label',
  'aria-description',
  'title',
  'placeholder',
  'alt',
  'label',
  'description',
  'caption',
])
const spanishWords = new Set([
  'el',
  'la',
  'los',
  'las',
  'de',
  'del',
  'para',
  'con',
  'por',
  'una',
  'un',
  'que',
  'se',
  'en',
  'y',
  'no',
  'es',
  'su',
])

export function looksSpanish(text: string): boolean {
  if (/[áéíóúñüÁÉÍÓÚÑ¿¡]/.test(text)) return true
  const words = text.toLowerCase().match(/[a-z]+/g) ?? []
  return words.filter((word) => spanishWords.has(word)).length >= 2
}

function findTextViolations(file: string): string[] {
  const source = readFileSync(file, 'utf8')
  const sf = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX)
  const found: string[] = []
  const where = (node: ts.Node) => {
    const { line } = sf.getLineAndCharacterOfPosition(node.getStart(sf))
    return `${rel(file)}:${String(line + 1)}`
  }
  const visit = (node: ts.Node): void => {
    if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) return
    if (ts.isJsxText(node) && /\p{L}/u.test(node.text)) {
      found.push(`${where(node)} JSX text: "${node.text.trim()}"`)
    } else if (
      ts.isJsxAttribute(node) &&
      node.initializer &&
      ts.isStringLiteral(node.initializer) &&
      userFacingAttributes.has(node.name.getText(sf)) &&
      /\p{L}/u.test(node.initializer.text)
    ) {
      found.push(`${where(node)} attribute ${node.name.getText(sf)}="${node.initializer.text}"`)
    } else if (
      (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) &&
      !ts.isJsxAttribute(node.parent) &&
      looksSpanish(node.text)
    ) {
      found.push(`${where(node)} Spanish-looking literal: "${node.text}"`)
    } else if (
      (ts.isTemplateHead(node) || ts.isTemplateMiddle(node) || ts.isTemplateTail(node)) &&
      looksSpanish(node.text)
    ) {
      found.push(`${where(node)} Spanish-looking template: "${node.text}"`)
    }
    ts.forEachChild(node, visit)
  }
  visit(sf)
  return found
}

describe('source rules', () => {
  it('scans a meaningful set of files', () => {
    expect(colorFiles.length).toBeGreaterThan(15)
    expect(textFiles.length).toBeGreaterThan(15)
  })

  it('has no hardcoded colors outside the token files', () => {
    const violations = colorFiles.flatMap((file) => {
      const matches = stripCssAndTsComments(readFileSync(file, 'utf8')).match(colorPattern) ?? []
      return matches.map((match) => `${rel(file)}: ${match}`)
    })
    expect(violations).toEqual([])
  })

  it('has no hardcoded Spanish text outside src/i18n', () => {
    expect(textFiles.flatMap(findTextViolations)).toEqual([])
  })

  describe('the heuristics themselves', () => {
    it('detect JSX text, attributes and Spanish-looking literals', () => {
      expect(looksSpanish('Cambiar carpeta…')).toBe(false) // caught by the attribute/JSX rules instead
      expect(looksSpanish('para el archivo')).toBe(true)
      expect(looksSpanish('boletín')).toBe(true)
      expect(looksSpanish('bg-primary text-on-primary items-center')).toBe(false)
      expect(colorPattern.test('color: #0F766E')).toBe(true)
      colorPattern.lastIndex = 0
      expect('rgb(0 0 0)'.match(colorPattern)).not.toBeNull()
    })
  })
})
