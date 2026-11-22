import { useEffect, useRef, useState } from 'react'
import { motion } from 'motion/react'
import { Download, Lock, Upload } from 'lucide-react'
import { Switch } from '@/components/ui/switch'
import { Kbd, KbdGroup } from '@/components/ui/kbd'
import { cn } from '@/lib/utils'
import { comboToTokens, eventToCombo, EMERGENCY_RESTORE_COMBO } from '@/lib/hotkey-format'
import {
  clearHotkey,
  exportSettings,
  getHotkeys,
  importSettings,
  onHotkeysState,
  resetAllHotkeys,
  resetHotkey,
  setHotkey,
  setHotkeyEnabled,
  type HotkeyRow,
} from '@/lib/tauri'

function ComboChip({ combo, recording }: { combo: string | null; recording: boolean }) {
  if (recording) {
    return (
      <span className="rounded-md border border-stage/40 bg-stage/10 px-2.5 py-1 text-xs font-medium text-stage">
        Press keys…
      </span>
    )
  }
  if (!combo) {
    return (
      <span className="rounded-md border border-dashed border-surface-border px-2.5 py-1 text-xs text-text-faint">
        Unbound
      </span>
    )
  }
  return (
    <KbdGroup>
      {comboToTokens(combo).map((t, i) => (
        <Kbd key={i}>{t}</Kbd>
      ))}
    </KbdGroup>
  )
}

function HotkeyRowView({ row, onChange }: { row: HotkeyRow; onChange: () => void }) {
  const [recording, setRecording] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!recording) return
    function onKeyDown(e: KeyboardEvent) {
      e.preventDefault()
      if (e.key === 'Escape') {
        setRecording(false)
        setError(null)
        return
      }
      if (e.key === 'Backspace') {
        clearHotkey(row.id).then(onChange)
        setRecording(false)
        setError(null)
        return
      }
      const combo = eventToCombo(e)
      if (!combo) {
        setError('Needs at least one modifier')
        return
      }
      setHotkey(row.id, combo).then((result) => {
        if (result.ok) {
          setRecording(false)
          setError(null)
          onChange()
        } else if (result.reason === 'conflict') {
          setError(`Already used by ${result.conflicting_action_label}.`)
        } else if (result.reason === 'system') {
          setError('Taken by another app or the system.')
        } else {
          setError('Not a valid shortcut.')
        }
      })
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [recording, row.id, onChange])

  const disconnected = row.is_display && row.display_connected === false

  return (
    <div
      ref={containerRef}
      className={cn(
        'flex flex-col gap-1 rounded-card border px-4 py-3 transition-colors',
        disconnected
          ? 'border-surface-border/50 bg-surface/50 opacity-50'
          : 'border-surface-border bg-surface',
      )}
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-sm text-text">{row.label}</span>
          {disconnected && (
            <span className="rounded-md bg-white/5 px-1.5 py-0.5 text-[10px] text-text-faint">
              Not connected
            </span>
          )}
        </div>
        <div className="flex items-center gap-3">
          {!disconnected && (
            <>
              <button
                onClick={() => resetHotkey(row.id).then(onChange)}
                className="text-[11px] text-text-faint hover:text-text-muted"
              >
                Reset
              </button>
              <motion.button
                onClick={() => {
                  setRecording(true)
                  setError(null)
                }}
                whileTap={{ scale: 0.96 }}
                transition={{ type: 'spring', stiffness: 500, damping: 35 }}
              >
                <ComboChip combo={row.combo} recording={recording} />
              </motion.button>
              <Switch
                size="sm"
                checked={row.enabled}
                onCheckedChange={(v) => setHotkeyEnabled(row.id, v).then(onChange)}
              />
            </>
          )}
        </div>
      </div>
      {error && <p className="text-[11px] text-stage">{error}</p>}
    </div>
  )
}

export default function HotkeysSection() {
  const [rows, setRows] = useState<HotkeyRow[]>([])
  const [settingsNote, setSettingsNote] = useState<string | null>(null)

  const refresh = () => getHotkeys().then(setRows)

  useEffect(() => {
    refresh()
    return onHotkeysState(setRows)
  }, [])

  useEffect(() => {
    if (!settingsNote) return
    const t = setTimeout(() => setSettingsNote(null), 3000)
    return () => clearTimeout(t)
  }, [settingsNote])

  const actionRows = rows.filter((r) => !r.is_display)
  const displayRows = rows.filter((r) => r.is_display)

  async function handleExport() {
    const ok = await exportSettings()
    if (ok) setSettingsNote('Exported shortcuts + agents to JSON.')
  }

  async function handleImport() {
    const ok = await importSettings()
    if (ok) {
      refresh()
      setSettingsNote('Imported — shortcuts + agents restored.')
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <header className="flex items-start justify-between">
        <div>
          <h1 className="font-display text-xl font-semibold tracking-tight">Hotkeys</h1>
          <p className="mt-1 text-sm text-text-muted">
            Click a combo to rebind it. Backspace clears, Esc cancels.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleImport}
            className="flex items-center gap-1.5 rounded-chip bg-white/5 px-3 py-1.5 text-xs font-medium text-text-muted hover:bg-white/10"
          >
            <Upload size={12} />
            Import
          </button>
          <button
            onClick={handleExport}
            className="flex items-center gap-1.5 rounded-chip bg-white/5 px-3 py-1.5 text-xs font-medium text-text-muted hover:bg-white/10"
          >
            <Download size={12} />
            Export
          </button>
          <button
            onClick={() => resetAllHotkeys().then(refresh)}
            className="rounded-chip bg-white/5 px-3 py-1.5 text-xs font-medium text-text-muted hover:bg-white/10"
          >
            Reset all
          </button>
        </div>
      </header>

      {settingsNote && (
        <p className="-mt-2 text-xs text-text-faint">{settingsNote}</p>
      )}

      <section className="flex flex-col gap-2">
        <h2 className="px-0.5 text-[11px] font-medium uppercase tracking-wide text-text-faint">
          Actions
        </h2>
        <div className="flex flex-col gap-1.5">
          {actionRows.map((row) => (
            <HotkeyRowView key={row.id} row={row} onChange={refresh} />
          ))}
        </div>
      </section>

      {displayRows.length > 0 && (
        <section className="flex flex-col gap-2">
          <h2 className="px-0.5 text-[11px] font-medium uppercase tracking-wide text-text-faint">
            Displays
          </h2>
          <div className="flex flex-col gap-1.5">
            {displayRows.map((row) => (
              <HotkeyRowView key={row.id} row={row} onChange={refresh} />
            ))}
          </div>
        </section>
      )}

      <div className="flex items-center justify-between rounded-card border border-surface-border/60 bg-surface/40 px-4 py-3">
        <div className="flex items-center gap-2">
          <Lock size={13} className="text-text-faint" />
          <span className="text-sm text-text-muted">Emergency restore</span>
          <span className="rounded-md bg-white/5 px-1.5 py-0.5 text-[10px] text-text-faint">
            can't be disabled
          </span>
        </div>
        <KbdGroup>
          {comboToTokens(EMERGENCY_RESTORE_COMBO).map((t, i) => (
            <Kbd key={i}>{t}</Kbd>
          ))}
        </KbdGroup>
      </div>
    </div>
  )
}
