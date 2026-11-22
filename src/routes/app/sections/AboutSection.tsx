import { useState } from 'react'
import { Check, Download, Loader2 } from 'lucide-react'
import { checkForUpdate, installUpdateAndRelaunch } from '@/lib/tauri'

const VERSION = '0.1.0'
const REPO_URL = 'https://github.com/chachannaman/curtain'

type UpdateState =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'up-to-date' }
  | { kind: 'available'; version: string }
  | { kind: 'installing'; pct: number }
  | { kind: 'error' }

export default function AboutSection() {
  const [update, setUpdate] = useState<UpdateState>({ kind: 'idle' })

  async function handleCheck() {
    setUpdate({ kind: 'checking' })
    try {
      const info = await checkForUpdate()
      setUpdate(info.available ? { kind: 'available', version: info.version ?? '' } : { kind: 'up-to-date' })
    } catch {
      setUpdate({ kind: 'error' })
    }
  }

  async function handleInstall() {
    setUpdate({ kind: 'installing', pct: 0 })
    try {
      await installUpdateAndRelaunch((pct) => setUpdate({ kind: 'installing', pct }))
    } catch {
      setUpdate({ kind: 'error' })
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <header>
        <h1 className="font-display text-xl font-semibold tracking-tight">About</h1>
      </header>

      <div className="flex flex-col items-center gap-2 rounded-card border border-surface-border bg-surface px-6 py-10 text-center">
        <span className="text-3xl">◐</span>
        <p className="font-display text-sm font-semibold text-text">Curtain</p>
        <p className="text-xs text-text-muted">Version {VERSION}</p>
      </div>

      <div className="flex items-center justify-between rounded-card border border-surface-border bg-surface px-4 py-3">
        <div>
          <p className="text-sm text-text">Updates</p>
          <p className="mt-0.5 text-xs text-text-faint">
            {update.kind === 'up-to-date' && "You're on the latest version"}
            {update.kind === 'available' && `Version ${update.version} is available`}
            {update.kind === 'installing' && `Installing… ${update.pct}%`}
            {update.kind === 'error' && "Couldn't check for updates"}
            {(update.kind === 'idle' || update.kind === 'checking') && 'Checked only when you ask'}
          </p>
        </div>
        {update.kind === 'available' ? (
          <button
            onClick={handleInstall}
            className="flex items-center gap-1.5 rounded-chip bg-stage/15 px-3 py-1.5 text-xs font-medium text-stage hover:bg-stage/20"
          >
            <Download size={12} /> Install & restart
          </button>
        ) : update.kind === 'up-to-date' ? (
          <span className="flex items-center gap-1 text-xs text-ok">
            <Check size={13} />
          </span>
        ) : (
          <button
            onClick={handleCheck}
            disabled={update.kind === 'checking' || update.kind === 'installing'}
            className="flex items-center gap-1.5 rounded-chip bg-white/5 px-3 py-1.5 text-xs font-medium text-text-muted hover:bg-white/10 disabled:opacity-50"
          >
            {update.kind === 'checking' && <Loader2 size={12} className="animate-spin" />}
            Check for updates
          </button>
        )}
      </div>

      <div className="flex flex-col gap-2.5 text-sm">
        <a
          href={REPO_URL}
          target="_blank"
          rel="noreferrer"
          className="flex items-center justify-between rounded-card border border-surface-border bg-surface px-4 py-3 text-text-muted transition-colors hover:text-text"
        >
          View on GitHub
          <span className="text-text-faint">↗</span>
        </a>
        <a
          href={`${REPO_URL}/issues`}
          target="_blank"
          rel="noreferrer"
          className="flex items-center justify-between rounded-card border border-surface-border bg-surface px-4 py-3 text-text-muted transition-colors hover:text-text"
        >
          Report an issue
          <span className="text-text-faint">↗</span>
        </a>
      </div>

      <p className="text-xs leading-relaxed text-text-faint">
        Curtain hides what's on screen — it is not a replacement for locking your Mac. Anyone with
        physical access can still use the keyboard to reach what's underneath unless Away Mode's
        lock option is on. Free, open source, no accounts, no telemetry.
      </p>
    </div>
  )
}
