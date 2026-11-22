import { useEffect, useState } from 'react'
import { motion, AnimatePresence } from 'motion/react'
import NumberFlow from '@number-flow/react'
import { MonitorX, Moon, Settings, Zap } from 'lucide-react'
import { cn } from '@/lib/utils'
import { formatCombo } from '@/lib/hotkey-format'
import {
  coverAll,
  coverDisplay,
  ensureNotificationPermission,
  getAgents,
  getCoverState,
  getHotkeys,
  getKeepAwakeMode,
  onAgentsState,
  onCoverState,
  openAppWindow,
  setKeepAwakeMode,
  uncoverAll,
  uncoverDisplay,
  type Agent,
  type CoverState,
  type KeepAwakeMode,
} from '@/lib/tauri'

const KEEP_AWAKE_LABELS: Record<KeepAwakeMode, string> = {
  off: 'Off',
  while_agents_run: 'While agents run',
  always: 'Always',
  timer: 'Timer',
}

function Tile({
  icon,
  label,
  active,
  onClick,
}: {
  icon: React.ReactNode
  label: string
  active?: boolean
  onClick: () => void
}) {
  return (
    <motion.button
      onClick={onClick}
      whileTap={{ scale: 0.97 }}
      transition={{ type: 'spring', stiffness: 500, damping: 35 }}
      className={cn(
        'flex flex-1 flex-col items-start gap-2 rounded-tile border px-4 py-3.5 text-left transition-colors',
        active
          ? 'border-stage/40 bg-stage/10 text-text'
          : 'border-surface-border bg-surface text-text hover:bg-surface-hover',
      )}
    >
      <span
        className={cn(
          'flex h-7 w-7 items-center justify-center rounded-full',
          active ? 'bg-stage/20 text-stage' : 'bg-white/5 text-text-muted',
        )}
      >
        {icon}
      </span>
      <span className="text-sm font-medium">{label}</span>
    </motion.button>
  )
}

function ScreenRow({
  name,
  covered,
  onToggle,
}: {
  name: string
  covered: boolean
  onToggle: () => void
}) {
  return (
    <div className="flex items-center justify-between rounded-card border border-surface-border bg-surface px-3 py-2.5">
      <div className="flex items-center gap-2.5">
        <span className="text-text-muted">▭</span>
        <span className="text-sm text-text">{name}</span>
      </div>
      <motion.button
        onClick={onToggle}
        whileTap={{ scale: 0.95 }}
        transition={{ type: 'spring', stiffness: 500, damping: 35 }}
        className={cn(
          'rounded-chip px-3 py-1 text-xs font-medium transition-colors',
          covered
            ? 'bg-stage/15 text-stage'
            : 'bg-white/5 text-text-muted hover:bg-white/10',
        )}
      >
        {covered ? 'Covered' : 'Cover'}
      </motion.button>
    </div>
  )
}

function elapsedLabel(startedAtMs: number) {
  const seconds = Math.max(0, Math.floor((Date.now() - startedAtMs) / 1000))
  const m = Math.floor(seconds / 60)
  const s = seconds % 60
  return `${m}:${s.toString().padStart(2, '0')}`
}

function AgentRow({ agent }: { agent: Agent }) {
  const waiting = agent.status === 'waiting'
  const crashed = agent.status === 'crashed'
  const idleMaybe = agent.status === 'idle_maybe'
  return (
    <motion.div
      className={cn(
        'flex items-center justify-between rounded-card border px-3 py-2.5',
        waiting
          ? 'border-stage/30 bg-stage/10'
          : crashed
            ? 'border-destructive/25 bg-destructive/10'
            : 'border-surface-border bg-surface',
      )}
      animate={waiting ? { opacity: [1, 0.6, 1] } : {}}
      transition={waiting ? { duration: 1.6, repeat: Infinity, ease: 'easeInOut' } : {}}
    >
      <div className="flex items-center gap-2.5">
        <span
          className={cn(
            'h-1.5 w-1.5 rounded-full',
            waiting
              ? 'bg-stage'
              : crashed
                ? 'bg-destructive'
                : idleMaybe || agent.status === 'done'
                  ? 'bg-text-faint'
                  : 'bg-ok',
          )}
        />
        <span className="text-sm text-text">{agent.display_name}</span>
      </div>
      <span
        className={cn(
          'text-xs',
          waiting ? 'text-stage' : crashed ? 'text-destructive' : 'text-text-muted',
        )}
      >
        {waiting
          ? 'waiting for you'
          : crashed
            ? `crashed${agent.exit_code != null ? ` (${agent.exit_code})` : ''}`
            : idleMaybe
              ? 'idle — maybe waiting?'
              : agent.status === 'done'
                ? 'done'
                : elapsedLabel(agent.started_at_ms)}
      </span>
    </motion.div>
  )
}

const TASK_PROFILE_IDS = new Set(['build', 'python-task', 'render'])
const AGENTS_CAP = 5

export default function Popover() {
  const [state, setState] = useState<CoverState>({ display_count: 0, covered: [] })
  const [awakeMode, setAwakeMode] = useState<KeepAwakeMode>('while_agents_run')
  const [agents, setAgents] = useState<Agent[]>([])
  const [panicCombo, setPanicCombo] = useState<string | null>(null)

  useEffect(() => {
    ensureNotificationPermission()
    getCoverState().then(setState)
    getKeepAwakeMode().then(setAwakeMode)
    getAgents().then(setAgents)
    getHotkeys().then((rows) => {
      const panic = rows.find((r) => r.id === 'panic')
      setPanicCombo(panic?.combo ?? null)
    })
    const offCover = onCoverState(setState)
    const offAgents = onAgentsState(setAgents)
    return () => {
      offCover()
      offAgents()
    }
  }, [])

  const sortedAgents = [...agents].sort((a, b) => {
    const rank = (s: Agent['status']) => (s === 'waiting' ? 0 : s === 'crashed' ? 1 : 2)
    return rank(a.status) - rank(b.status)
  })
  const visibleAgents = sortedAgents.slice(0, AGENTS_CAP)
  const overflowCount = sortedAgents.length - visibleAgents.length
  const headerLabel = agents.some((a) => TASK_PROFILE_IDS.has(a.key)) ? 'tasks running' : 'agents'

  const allCovered = state.display_count > 0 && state.covered.length === state.display_count

  async function toggleAll() {
    if (allCovered) {
      await uncoverAll()
    } else {
      await coverAll()
    }
  }

  async function toggleAway() {
    await coverAll()
    await setKeepAwakeMode('always')
    setAwakeMode('always')
  }

  async function toggleDisplay(i: number) {
    if (state.covered.includes(i)) {
      await uncoverDisplay(i)
    } else {
      await coverDisplay(i)
    }
  }

  function cycleKeepAwake() {
    const order: KeepAwakeMode[] = ['off', 'while_agents_run', 'always', 'timer']
    const next = order[(order.indexOf(awakeMode) + 1) % order.length]
    setAwakeMode(next)
    setKeepAwakeMode(next)
  }

  return (
    <div className="flex h-full w-full flex-col gap-4 p-4">
      <header className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-lg">◐</span>
          <span className="font-display text-sm font-semibold tracking-tight">Curtain</span>
        </div>
        <div className="flex items-center gap-1.5 text-xs text-text-muted">
          <span className="h-1.5 w-1.5 rounded-full bg-ok" />
          <NumberFlow value={agents.length} /> {headerLabel}
        </div>
      </header>

      <div className="flex gap-2.5">
        <Tile
          icon={<MonitorX size={15} />}
          label={allCovered ? 'Uncover all' : 'Blackout all'}
          active={allCovered}
          onClick={toggleAll}
        />
        <Tile icon={<Moon size={15} />} label="Away Mode" onClick={toggleAway} />
      </div>

      <section className="flex flex-col gap-2">
        <h2 className="px-0.5 text-[11px] font-medium uppercase tracking-wide text-text-faint">
          Screens
        </h2>
        <div className="flex flex-col gap-1.5">
          <AnimatePresence initial={false}>
            {Array.from({ length: state.display_count }).map((_, i) => (
              <motion.div
                key={i}
                initial={{ opacity: 0, y: -4 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.18 }}
              >
                <ScreenRow
                  name={i === 0 ? 'Built-in Display' : `Display ${i + 1}`}
                  covered={state.covered.includes(i)}
                  onToggle={() => toggleDisplay(i)}
                />
              </motion.div>
            ))}
          </AnimatePresence>
        </div>
      </section>

      {agents.length > 0 && (
        <section className="flex flex-col gap-2">
          <h2 className="px-0.5 text-[11px] font-medium uppercase tracking-wide text-text-faint">
            Agents
          </h2>
          <div className="flex flex-col gap-1.5">
            <AnimatePresence initial={false}>
              {visibleAgents.map((agent) => (
                <motion.div
                  key={agent.key}
                  initial={{ opacity: 0, y: -4 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.18 }}
                >
                  <AgentRow agent={agent} />
                </motion.div>
              ))}
            </AnimatePresence>
            {overflowCount > 0 && (
              <p className="px-1 text-[11px] text-text-faint">+{overflowCount} more</p>
            )}
          </div>
        </section>
      )}

      <button
        onClick={cycleKeepAwake}
        className="flex items-center justify-between rounded-card border border-surface-border bg-surface px-3 py-2.5 text-left transition-colors hover:bg-surface-hover"
      >
        <span className="flex items-center gap-2 text-sm text-text">
          <Zap size={14} className="text-text-muted" />
          Keep awake
        </span>
        <span className="text-xs text-text-muted">{KEEP_AWAKE_LABELS[awakeMode]}</span>
      </button>

      <div className="mt-auto flex items-center justify-between border-t border-surface-border pt-3 text-xs text-text-muted">
        <button onClick={openAppWindow} className="hover:text-text">
          Open Curtain
        </button>
        <span className="font-mono">
          {panicCombo ? `${formatCombo(panicCombo)} panic` : 'panic unbound'}
        </span>
        <button onClick={openAppWindow} className="hover:text-text">
          <Settings size={14} />
        </button>
      </div>
    </div>
  )
}
