import { useEffect, useState } from 'react'
import { cn } from '@/lib/utils'
import { getKeepAwakeMode, setKeepAwakeMode, type KeepAwakeMode } from '@/lib/tauri'

const MODES: { id: KeepAwakeMode; label: string; description: string }[] = [
  { id: 'off', label: 'Off', description: 'Never prevent sleep.' },
  {
    id: 'while_agents_run',
    label: 'While agents run',
    description: 'Keep awake only when Claude Code or Codex is active.',
  },
  { id: 'always', label: 'Always', description: 'Prevent sleep the whole time Curtain runs.' },
  { id: 'timer', label: 'Timer', description: 'Stay awake for a fixed window, then stop.' },
]

const TIMER_OPTIONS = [30, 60, 120, 240]

export default function KeepAwakeSection() {
  const [mode, setMode] = useState<KeepAwakeMode>('while_agents_run')
  const [timerMinutes, setTimerMinutes] = useState(30)

  useEffect(() => {
    getKeepAwakeMode().then(setMode)
  }, [])

  async function selectMode(next: KeepAwakeMode) {
    setMode(next)
    await setKeepAwakeMode(next, next === 'timer' ? timerMinutes : undefined)
  }

  return (
    <div className="flex flex-col gap-6">
      <header>
        <h1 className="font-display text-xl font-semibold tracking-tight">Keep Awake</h1>
        <p className="mt-1 text-sm text-text-muted">
          Prevents system sleep. Screens may still sleep on their own.
        </p>
      </header>

      <div className="flex flex-col gap-2.5">
        {MODES.map((m) => (
          <button
            key={m.id}
            onClick={() => selectMode(m.id)}
            className={cn(
              'flex flex-col items-start gap-0.5 rounded-card border px-4 py-3 text-left transition-colors',
              mode === m.id
                ? 'border-stage/40 bg-stage/10'
                : 'border-surface-border bg-surface hover:bg-surface-hover',
            )}
          >
            <span className={cn('text-sm font-medium', mode === m.id ? 'text-stage' : 'text-text')}>
              {m.label}
            </span>
            <span className="text-xs text-text-muted">{m.description}</span>
          </button>
        ))}
      </div>

      {mode === 'timer' && (
        <div className="flex items-center gap-2">
          {TIMER_OPTIONS.map((min) => (
            <button
              key={min}
              onClick={() => {
                setTimerMinutes(min)
                setKeepAwakeMode('timer', min)
              }}
              className={cn(
                'rounded-chip px-3 py-1.5 text-xs font-medium transition-colors',
                timerMinutes === min
                  ? 'bg-stage/15 text-stage'
                  : 'bg-white/5 text-text-muted hover:bg-white/10',
              )}
            >
              {min < 60 ? `${min}m` : `${min / 60}h`}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
