import { useEffect, useMemo, useState } from 'react'
import { motion, AnimatePresence } from 'motion/react'
import { Bot, Clock, Copy, Lock, Plus, Sparkles, Trash2, XCircle } from 'lucide-react'
import { Switch } from '@/components/ui/switch'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { cn } from '@/lib/utils'
import { ICON_CHOICES, COLOR_SWATCHES, resolveIcon } from '@/lib/icon-map'
import {
  connectClaudeCode,
  connectCodex,
  deleteAgentProfile,
  duplicateAgentProfile,
  getAgentHistory,
  getAgentProfiles,
  getAgents,
  getTimeSavedThisWeekMs,
  listRunningProcesses,
  onAgentsState,
  onProfilesChanged,
  restorePresetAgents,
  setAgentProfileEnabled,
  testMatchRules,
  upsertAgentProfile,
  type Agent,
  type AgentProfile,
  type HistoryEntry,
  type RunningProcess,
} from '@/lib/tauri'

function elapsed(startedAtMs: number) {
  const s = Math.max(0, Math.floor((Date.now() - startedAtMs) / 1000))
  const m = Math.floor(s / 60)
  const h = Math.floor(m / 60)
  if (h > 0) return `${h}h ${m % 60}m`
  return `${m}m ${s % 60}s`
}

function formatDuration(ms: number) {
  const totalMinutes = Math.round(ms / 60000)
  const h = Math.floor(totalMinutes / 60)
  const m = totalMinutes % 60
  if (h > 0) return `${h}h ${m}m`
  if (m > 0) return `${m}m`
  return '< 1m'
}

function relativeTime(ms: number) {
  const diff = Date.now() - ms
  const mins = Math.floor(diff / 60000)
  if (mins < 1) return 'just now'
  if (mins < 60) return `${mins}m ago`
  const hours = Math.floor(mins / 60)
  if (hours < 24) return `${hours}h ago`
  return `${Math.floor(hours / 24)}d ago`
}

function TimeSavedStat({ ms }: { ms: number }) {
  return (
    <div className="flex items-center gap-3 rounded-card border border-surface-border bg-surface px-4 py-3.5">
      <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-stage/15 text-stage">
        <Sparkles size={15} />
      </span>
      <div>
        {ms > 0 ? (
          <p className="text-sm text-text">
            You saved <span className="font-medium text-stage">{formatDuration(ms)}</span> this
            week not babysitting a terminal
          </p>
        ) : (
          <p className="text-sm text-text-muted">No waiting time logged yet this week</p>
        )}
      </div>
    </div>
  )
}

function HistoryRow({ entry, profile }: { entry: HistoryEntry; profile?: AgentProfile }) {
  const crashed = entry.end_status === 'crashed'
  const stillOpen = entry.ended_at_ms === null
  return (
    <div className="flex items-center justify-between rounded-card border border-surface-border bg-surface px-4 py-3">
      <div className="flex items-center gap-2.5">
        {profile ? (
          <ProfileIcon icon={profile.icon} color={profile.color} size={12} />
        ) : (
          <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-white/5 text-text-faint">
            <Bot size={12} />
          </span>
        )}
        <div>
          <p className="text-sm text-text">{entry.display_name}</p>
          <p className="mt-0.5 text-xs text-text-faint">
            {relativeTime(entry.started_at_ms)}
            {entry.waiting_ms > 0 && <> · waited {formatDuration(entry.waiting_ms)}</>}
          </p>
        </div>
      </div>
      {stillOpen ? (
        <span className="rounded-md bg-white/5 px-1.5 py-0.5 text-[10px] text-text-faint">
          running
        </span>
      ) : crashed ? (
        <span className="flex items-center gap-1 rounded-md bg-destructive/15 px-1.5 py-0.5 text-[10px] text-destructive">
          <XCircle size={10} />
          crashed{entry.exit_code !== null ? ` (${entry.exit_code})` : ''}
        </span>
      ) : (
        <span className="flex items-center gap-1 rounded-md bg-ok/15 px-1.5 py-0.5 text-[10px] text-ok">
          <Clock size={10} />
          done
        </span>
      )}
    </div>
  )
}

function ProfileIcon({ icon, color, size = 14 }: { icon: string; color: string; size?: number }) {
  // resolveIcon is a static lookup table, not a component factory — the
  // reference is stable across renders, this isn't a dynamic component.
  const Lucide = resolveIcon(icon)
  return (
    <span
      className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full"
      style={{ backgroundColor: `${color}26`, color }}
    >
      {/* oxlint-disable-next-line react/static-components */}
      {Lucide ? <Lucide size={size} /> : <span className="text-xs">{icon}</span>}
    </span>
  )
}

function AgentCard({ agent, profile }: { agent: Agent; profile?: AgentProfile }) {
  const waiting = agent.status === 'waiting'
  const crashed = agent.status === 'crashed'
  const idleMaybe = agent.status === 'idle_maybe'
  return (
    <motion.div
      initial={{ opacity: 0, y: -4 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0 }}
      className={cn(
        'flex items-center justify-between rounded-card border px-4 py-3.5',
        waiting
          ? 'border-stage/30 bg-stage/10'
          : crashed
            ? 'border-destructive/25 bg-destructive/10'
            : 'border-surface-border bg-surface',
      )}
    >
      <div className="flex items-center gap-3">
        <ProfileIcon icon={agent.icon} color={agent.color} />
        <div>
          <div className="text-sm font-medium text-text">{agent.display_name}</div>
          <div className="text-xs text-text-muted">pid {agent.pid || '—'}</div>
        </div>
      </div>
      <div className="flex items-center gap-2">
        <span
          className={cn(
            'text-xs',
            waiting ? 'text-stage' : crashed ? 'text-destructive' : 'text-text-muted',
          )}
        >
          {waiting
            ? 'waiting for you'
            : crashed
              ? `crashed${agent.exit_code != null ? ` (exit ${agent.exit_code})` : ''}`
              : idleMaybe
                ? 'idle — maybe waiting?'
                : agent.status === 'done'
                  ? 'done'
                  : elapsed(agent.started_at_ms)}
        </span>
        {profile?.alert_source === 'wrapper' && (
          <WrapperAliasButton profile={profile} compact />
        )}
      </div>
    </motion.div>
  )
}

function WrapperAliasButton({ profile, compact }: { profile: AgentProfile; compact?: boolean }) {
  const [copied, setCopied] = useState(false)
  const command = profile.match_rules[0]?.process_name ?? profile.id
  const alias = `alias ${profile.id}='curtain run --agent ${profile.id} -- ${command}'`
  return (
    <button
      onClick={() => {
        navigator.clipboard?.writeText(alias)
        setCopied(true)
        setTimeout(() => setCopied(false), 1500)
      }}
      className={cn(
        'flex items-center gap-1 rounded-md bg-white/5 px-2 py-1 text-[11px] text-text-faint hover:bg-white/10 hover:text-text-muted',
        compact && 'shrink-0',
      )}
      title={alias}
    >
      <Copy size={11} />
      {copied ? 'Copied' : 'alias'}
    </button>
  )
}

function ProfileRow({
  profile,
  onToggle,
  onEdit,
  onDuplicate,
  onDelete,
}: {
  profile: AgentProfile
  onToggle: (v: boolean) => void
  onEdit: () => void
  onDuplicate: () => void
  onDelete: () => void
}) {
  return (
    <div className="flex items-center justify-between rounded-card border border-surface-border bg-surface px-4 py-3">
      <div className="flex items-center gap-3">
        <ProfileIcon icon={profile.icon} color={profile.color} />
        <div>
          <div className="text-sm text-text">{profile.display_name}</div>
          <div className="text-[11px] text-text-faint">
            {profile.alert_source === 'hooks'
              ? 'official hooks'
              : profile.alert_source === 'wrapper'
                ? 'wrapper CLI'
                : profile.alert_source === 'idle_heuristic'
                  ? 'idle heuristic'
                  : 'no alerts'}
          </div>
        </div>
      </div>
      <div className="flex items-center gap-3">
        {profile.alert_source === 'wrapper' && <WrapperAliasButton profile={profile} />}
        <button onClick={onEdit} className="text-[11px] text-text-faint hover:text-text-muted">
          Edit
        </button>
        <button onClick={onDuplicate} className="text-[11px] text-text-faint hover:text-text-muted">
          Duplicate
        </button>
        {profile.is_preset ? (
          <Lock size={12} className="text-text-faint" />
        ) : (
          <button onClick={onDelete} className="text-text-faint hover:text-destructive">
            <Trash2 size={13} />
          </button>
        )}
        <Switch checked={profile.enabled} onCheckedChange={onToggle} />
      </div>
    </div>
  )
}

function AddAgentSheet({
  open,
  onOpenChange,
  editing,
  onSaved,
}: {
  open: boolean
  onOpenChange: (v: boolean) => void
  editing: AgentProfile | null
  onSaved: () => void
}) {
  const [mode, setMode] = useState<'custom' | 'pick'>('custom')
  const [name, setName] = useState('')
  const [icon, setIcon] = useState(ICON_CHOICES[0])
  const [color, setColor] = useState(COLOR_SWATCHES[0])
  const [processName, setProcessName] = useState('')
  const [search, setSearch] = useState('')
  const [processes, setProcesses] = useState<RunningProcess[]>([])
  const [matchCount, setMatchCount] = useState<number | null>(null)

  // Resetting the form when the sheet opens (or swaps which profile
  // it's editing) is the actual external event here — there's no
  // single prior render to derive this from.
  useEffect(() => {
    if (!open) return
    listRunningProcesses().then(setProcesses)
    if (editing) {
      // oxlint-disable-next-line react/set-state-in-effect
      setName(editing.display_name)
      setIcon(editing.icon)
      setColor(editing.color)
      setProcessName(editing.match_rules[0]?.process_name ?? '')
    } else {
      setName('')
      setIcon(ICON_CHOICES[0])
      setColor(COLOR_SWATCHES[0])
      setProcessName('')
    }
    setMatchCount(null)
  }, [open, editing])

  const filteredProcesses = useMemo(
    () =>
      processes
        .filter((p) => p.name.toLowerCase().includes(search.toLowerCase()))
        .slice(0, 8),
    [processes, search],
  )

  async function handleTest() {
    const count = await testMatchRules([{ process_name: processName }])
    setMatchCount(count)
  }

  async function handleSave() {
    const profile: AgentProfile = {
      id: editing?.id ?? crypto.randomUUID(),
      display_name: name || processName || 'Custom agent',
      icon,
      color,
      match_rules: [{ process_name: processName || null }],
      alert_source: editing?.alert_source ?? 'idle_heuristic',
      keep_awake: editing?.keep_awake ?? true,
      notify: editing?.notify ?? { on_waiting: true, on_done: true, on_crash: true },
      enabled: true,
      is_preset: editing?.is_preset ?? false,
    }
    await upsertAgentProfile(profile)
    onSaved()
    onOpenChange(false)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md bg-bg ring-0 border border-surface-border">
        <DialogHeader>
          <DialogTitle className="font-display text-text">
            {editing ? 'Edit agent' : 'Add agent'}
          </DialogTitle>
          <DialogDescription className="text-text-muted">
            Pick a running process to match on, or fill in details by hand.
          </DialogDescription>
        </DialogHeader>

        {!editing && (
          <div className="flex gap-1.5 rounded-chip bg-white/5 p-1">
            {(['custom', 'pick'] as const).map((m) => (
              <button
                key={m}
                onClick={() => setMode(m)}
                className={cn(
                  'flex-1 rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors',
                  mode === m ? 'bg-stage/15 text-stage' : 'text-text-muted hover:text-text',
                )}
              >
                {m === 'pick' ? 'Pick a process' : 'Custom'}
              </button>
            ))}
          </div>
        )}

        {mode === 'pick' && !editing && (
          <div className="flex flex-col gap-2">
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search running processes…"
              className="rounded-md border border-surface-border bg-surface px-2.5 py-1.5 text-xs text-text outline-none focus:border-stage/40"
            />
            <div className="flex max-h-40 flex-col gap-1 overflow-y-auto">
              {filteredProcesses.map((p) => (
                <button
                  key={p.pid}
                  onClick={() => {
                    setProcessName(p.name)
                    setName(p.name)
                    setMode('custom')
                  }}
                  className="rounded-md border border-surface-border bg-surface px-2.5 py-1.5 text-left text-xs hover:bg-surface-hover"
                >
                  <div className="text-text">{p.name}</div>
                  <div className="truncate text-[10px] text-text-faint">{p.cmd}</div>
                </button>
              ))}
            </div>
          </div>
        )}

        {mode === 'custom' && (
          <div className="flex flex-col gap-3">
            <label className="flex flex-col gap-1 text-xs text-text-muted">
              Name
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="rounded-md border border-surface-border bg-surface px-2.5 py-1.5 text-sm text-text outline-none focus:border-stage/40"
              />
            </label>
            <label className="flex flex-col gap-1 text-xs text-text-muted">
              Process name
              <input
                value={processName}
                onChange={(e) => setProcessName(e.target.value)}
                placeholder="e.g. python"
                className="rounded-md border border-surface-border bg-surface px-2.5 py-1.5 text-sm text-text outline-none focus:border-stage/40"
              />
            </label>

            <div className="flex items-center gap-2">
              {ICON_CHOICES.slice(0, 8).map((name) => {
                const Lucide = resolveIcon(name)
                return (
                  <button
                    key={name}
                    onClick={() => setIcon(name)}
                    className={cn(
                      'flex h-7 w-7 items-center justify-center rounded-md border',
                      icon === name
                        ? 'border-stage/50 bg-stage/10 text-stage'
                        : 'border-surface-border bg-surface text-text-muted',
                    )}
                  >
                    {Lucide && <Lucide size={14} />}
                  </button>
                )
              })}
            </div>
            <div className="flex items-center gap-2">
              {COLOR_SWATCHES.map((c) => (
                <button
                  key={c}
                  onClick={() => setColor(c)}
                  className={cn(
                    'h-6 w-6 rounded-full border-2',
                    color === c ? 'border-text' : 'border-transparent',
                  )}
                  style={{ backgroundColor: c }}
                />
              ))}
            </div>

            <div className="flex items-center gap-2">
              <button
                onClick={handleTest}
                className="rounded-chip bg-white/5 px-3 py-1.5 text-xs text-text-muted hover:bg-white/10"
              >
                Test
              </button>
              {matchCount !== null && (
                <span className={cn('text-xs', matchCount > 0 ? 'text-ok' : 'text-text-faint')}>
                  {matchCount > 0
                    ? `Matches ${matchCount} running process${matchCount === 1 ? '' : 'es'} ✓`
                    : 'Matches 0 — check your rules'}
                </span>
              )}
            </div>
          </div>
        )}

        <DialogFooter>
          <button
            onClick={handleSave}
            className="rounded-chip bg-stage px-4 py-2 text-xs font-medium text-[#1a1203] hover:opacity-90"
          >
            Save
          </button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

export default function AgentsSection() {
  const [agents, setAgents] = useState<Agent[]>([])
  const [profiles, setProfiles] = useState<AgentProfile[]>([])
  const [history, setHistory] = useState<HistoryEntry[]>([])
  const [timeSavedMs, setTimeSavedMs] = useState(0)
  const [sheetOpen, setSheetOpen] = useState(false)
  const [editing, setEditing] = useState<AgentProfile | null>(null)

  const refreshProfiles = () => getAgentProfiles().then(setProfiles)
  const refreshHistory = () => {
    getAgentHistory().then(setHistory)
    getTimeSavedThisWeekMs().then(setTimeSavedMs)
  }

  useEffect(() => {
    getAgents().then(setAgents)
    refreshProfiles()
    refreshHistory()
    const unAgents = onAgentsState((a) => {
      setAgents(a)
      refreshHistory()
    })
    const unProfiles = onProfilesChanged(refreshProfiles)
    return () => {
      unAgents()
      unProfiles()
    }
  }, [])

  const profileById = useMemo(() => new Map(profiles.map((p) => [p.id, p])), [profiles])

  return (
    <div className="flex flex-col gap-6">
      <header className="flex items-start justify-between">
        <div>
          <h1 className="font-display text-xl font-semibold tracking-tight">Agents</h1>
          <p className="mt-1 text-sm text-text-muted">
            Any agent or long-running task — not just Claude Code and Codex.
          </p>
        </div>
        <button
          onClick={() => {
            setEditing(null)
            setSheetOpen(true)
          }}
          className="flex items-center gap-1.5 rounded-chip bg-stage/15 px-3 py-1.5 text-xs font-medium text-stage hover:bg-stage/20"
        >
          <Plus size={13} /> Add agent
        </button>
      </header>

      <TimeSavedStat ms={timeSavedMs} />

      {agents.length === 0 ? (
        <div className="flex flex-col items-center gap-4 rounded-card border border-dashed border-surface-border px-6 py-12 text-center">
          <Bot size={22} className="text-text-faint" />
          <div>
            <p className="text-sm text-text">No agents detected right now</p>
            <p className="mt-1 text-xs text-text-muted">
              Connect a hook so Curtain knows the moment one needs you.
            </p>
          </div>
          <div className="flex flex-wrap justify-center gap-2.5">
            <button
              onClick={() => connectClaudeCode()}
              className="rounded-chip bg-stage/15 px-3.5 py-1.5 text-xs font-medium text-stage hover:bg-stage/20"
            >
              Connect Claude Code
            </button>
            <button
              onClick={() => connectCodex()}
              className="rounded-chip bg-white/5 px-3.5 py-1.5 text-xs font-medium text-text-muted hover:bg-white/10"
            >
              Connect Codex
            </button>
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-2.5">
          <AnimatePresence initial={false}>
            {agents.map((a) => (
              <AgentCard key={a.key} agent={a} profile={profileById.get(a.key)} />
            ))}
          </AnimatePresence>
        </div>
      )}

      <section className="flex flex-col gap-2">
        <div className="flex items-center justify-between px-0.5">
          <h2 className="text-[11px] font-medium uppercase tracking-wide text-text-faint">
            Profiles
          </h2>
          <button
            onClick={() => restorePresetAgents().then(refreshProfiles)}
            className="text-[11px] text-text-faint hover:text-text-muted"
          >
            Restore presets
          </button>
        </div>
        <div className="flex flex-col gap-1.5">
          {profiles.map((p) => (
            <ProfileRow
              key={p.id}
              profile={p}
              onToggle={(v) => setAgentProfileEnabled(p.id, v).then(refreshProfiles)}
              onEdit={() => {
                setEditing(p)
                setSheetOpen(true)
              }}
              onDuplicate={() => duplicateAgentProfile(p.id).then(refreshProfiles)}
              onDelete={() => deleteAgentProfile(p.id).then(refreshProfiles).catch(() => {})}
            />
          ))}
        </div>
      </section>

      {history.length > 0 && (
        <section className="flex flex-col gap-2">
          <h2 className="px-0.5 text-[11px] font-medium uppercase tracking-wide text-text-faint">
            History
          </h2>
          <div className="flex flex-col gap-1.5">
            {history.slice(0, 20).map((h) => (
              <HistoryRow key={h.id} entry={h} profile={profileById.get(h.agent_key)} />
            ))}
          </div>
        </section>
      )}

      <AddAgentSheet
        open={sheetOpen}
        onOpenChange={setSheetOpen}
        editing={editing}
        onSaved={refreshProfiles}
      />
    </div>
  )
}
