/**
 * Looks up a lucide icon component by the kebab-case name stored on an
 * agent profile. Profiles can also store a plain emoji in `icon` (no
 * lucide mapping needed — rendered as text), so this only needs to
 * cover the names Curtain's own presets and the Custom-profile icon
 * picker actually use.
 */
import {
  Bot,
  Code,
  Feather,
  FlaskConical,
  SquareTerminal,
  Hammer,
  MousePointer,
  PencilLine,
  Sparkles,
  Terminal,
  Zap,
  Film,
  Bug,
  Rocket,
  type LucideIcon,
} from 'lucide-react'

export const ICON_MAP: Record<string, LucideIcon> = {
  bot: Bot,
  code: Code,
  feather: Feather,
  'flask-conical': FlaskConical,
  github: SquareTerminal,
  hammer: Hammer,
  'mouse-pointer': MousePointer,
  'pencil-line': PencilLine,
  sparkles: Sparkles,
  terminal: Terminal,
  zap: Zap,
  film: Film,
  bug: Bug,
  rocket: Rocket,
}

/** Icon choices offered in the Custom-profile picker. */
export const ICON_CHOICES = Object.keys(ICON_MAP)

export const COLOR_SWATCHES = [
  '#D97757',
  '#74AA9C',
  '#4C8DF6',
  '#6E6ADE',
  '#8957E5',
  '#6F42C1',
  '#60A5FA',
  '#53B48A',
]

function isEmoji(s: string) {
  return /\p{Emoji}/u.test(s) && !/^[a-z-]+$/.test(s)
}

export function resolveIcon(icon: string): LucideIcon | null {
  return ICON_MAP[icon] ?? null
}

export { isEmoji }
