import axe from 'axe-core'
import { expect } from 'vitest'

/**
 * Runs axe-core on a container and fails with a readable list of violations.
 * `color-contrast` is disabled because jsdom has no layout/CSS cascade; contrast
 * is verified from the tokens by contrast.test.ts instead.
 */
export async function expectNoA11yViolations(container: Element): Promise<void> {
  const results = await axe.run(container, {
    rules: { 'color-contrast': { enabled: false } },
  })
  const summary = results.violations.map(
    (violation) =>
      `${violation.id}: ${violation.help} (${violation.nodes.map((node) => node.target.join(' ')).join(', ')})`,
  )
  expect(summary).toEqual([])
}
