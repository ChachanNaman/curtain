/**
 * Shared combo-string <-> display formatting for hotkeys. The backend
 * stores/compares combos in the lowercase, `+`-joined form
 * `tauri-plugin-global-shortcut` uses (e.g. `ctrl+alt+h`); this renders
 * that as platform glyphs and parses a `KeyboardEvent` back into it.
 */

const isMac =
  typeof navigator !== 'undefined' && /mac/i.test(navigator.platform ?? navigator.userAgent ?? '')

const MODIFIER_GLYPHS_MAC: Record<string, string> = {
  ctrl: '⌃',
  alt: '⌥',
  shift: '⇧',
  cmd: '⌘',
  meta: '⌘',
  super: '⌘',
}

const MODIFIER_LABELS_OTHER: Record<string, string> = {
  ctrl: 'Ctrl',
  alt: 'Alt',
  shift: 'Shift',
  cmd: 'Win',
  meta: 'Win',
  super: 'Win',
}

const KEY_GLYPHS: Record<string, string> = {
  escape: 'Esc',
  equal: '=',
  minus: '-',
  space: 'Space',
}

/** Splits a combo string into its ordered display tokens (e.g. `['⌃', '⌥', 'H']`). */
export function comboToTokens(combo: string): string[] {
  const parts = combo.toLowerCase().split('+').filter(Boolean)
  return parts.map((part) => {
    if (isMac && part in MODIFIER_GLYPHS_MAC) return MODIFIER_GLYPHS_MAC[part]
    if (!isMac && part in MODIFIER_LABELS_OTHER) return MODIFIER_LABELS_OTHER[part]
    if (part in KEY_GLYPHS) return KEY_GLYPHS[part]
    return part.length === 1 ? part.toUpperCase() : part.charAt(0).toUpperCase() + part.slice(1)
  })
}

export function formatCombo(combo: string | null): string {
  if (!combo) return ''
  return comboToTokens(combo).join(isMac ? '' : '+')
}

const KEY_NAME_MAP: Record<string, string> = {
  ' ': 'space',
  Escape: 'escape',
  '=': 'equal',
  '-': 'minus',
  ArrowUp: 'up',
  ArrowDown: 'down',
  ArrowLeft: 'left',
  ArrowRight: 'right',
}

/** Builds a combo string from a keydown event, or `null` if the key alone
 * (no modifier) was pressed — callers should treat that as invalid. */
export function eventToCombo(e: KeyboardEvent): string | null {
  const mods: string[] = []
  if (e.ctrlKey) mods.push('ctrl')
  if (e.altKey) mods.push('alt')
  if (e.shiftKey) mods.push('shift')
  if (e.metaKey) mods.push('cmd')

  if (['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) return null
  if (mods.length === 0) return null

  const key = KEY_NAME_MAP[e.key] ?? e.key.toLowerCase()
  return [...mods, key].join('+')
}

export const EMERGENCY_RESTORE_COMBO = isMac ? 'ctrl+alt+cmd+escape' : 'ctrl+alt+shift+escape'
