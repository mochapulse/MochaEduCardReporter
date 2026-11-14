import { render, type RenderResult } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactElement } from 'react'
import { TooltipProvider } from '@/components/ui/tooltip'

/** Renders with the providers every component may need and returns a user-event instance. */
export function renderUi(
  ui: ReactElement,
): RenderResult & { user: ReturnType<typeof userEvent.setup> } {
  const user = userEvent.setup()
  return { user, ...render(<TooltipProvider delayDuration={0}>{ui}</TooltipProvider>) }
}
