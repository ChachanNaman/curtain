import { useEffect, useState } from 'react'
import { Switch } from '@/components/ui/switch'
import { Input } from '@/components/ui/input'
import {
  getIdleAutoCover,
  isAutostartEnabled,
  isUnlockRequired,
  setAutostart,
  setIdleAutoCover,
  setUnlockRequired,
} from '@/lib/tauri'

function Row({
  label,
  description,
  checked,
  onChange,
  disabled,
  badge,
  trailing,
}: {
  label: string
  description: string
  checked: boolean
  onChange: (v: boolean) => void
  disabled?: boolean
  badge?: string
  trailing?: React.ReactNode
}) {
  return (
    <div className="flex items-center justify-between rounded-card border border-surface-border bg-surface px-4 py-3.5">
      <div className="pr-4">
        <div className="flex items-center gap-2 text-sm text-text">
          {label}
          {badge && (
            <span className="rounded-md bg-white/5 px-1.5 py-0.5 text-[10px] text-text-faint">
              {badge}
            </span>
          )}
        </div>
        <p className="mt-0.5 text-xs text-text-muted">{description}</p>
      </div>
      <div className="flex items-center gap-3">
        {trailing}
        <Switch checked={checked} onCheckedChange={onChange} disabled={disabled} />
      </div>
    </div>
  )
}

export default function PrivacySection() {
  const [lockOnAway, setLockOnAway] = useState(false)
  const [unlockRequired, setUnlockRequiredState] = useState(false)
  const [idleEnabled, setIdleEnabled] = useState(false)
  const [idleMinutes, setIdleMinutes] = useState(10)
  const [autostart, setAutostartState] = useState(false)

  useEffect(() => {
    isUnlockRequired().then(setUnlockRequiredState)
    getIdleAutoCover().then(({ enabled, minutes }) => {
      setIdleEnabled(enabled)
      setIdleMinutes(minutes)
    })
    isAutostartEnabled().then(setAutostartState)
  }, [])

  async function toggleUnlock(v: boolean) {
    setUnlockRequiredState(v)
    await setUnlockRequired(v)
  }

  async function toggleIdle(v: boolean) {
    setIdleEnabled(v)
    await setIdleAutoCover(v, idleMinutes)
  }

  async function changeIdleMinutes(v: number) {
    setIdleMinutes(v)
    if (idleEnabled) await setIdleAutoCover(true, v)
  }

  async function toggleAutostart(v: boolean) {
    setAutostartState(v)
    await setAutostart(v)
  }

  return (
    <div className="flex flex-col gap-6">
      <header>
        <h1 className="font-display text-xl font-semibold tracking-tight">Privacy</h1>
        <p className="mt-1 text-sm text-text-muted">
          The cover is privacy, not security — it doesn't lock your Mac unless you turn that on
          below.
        </p>
      </header>

      <div className="flex flex-col gap-2.5">
        <Row
          label="Unlock with Touch ID"
          description="Require biometrics or your password to restore covers."
          checked={unlockRequired}
          onChange={toggleUnlock}
        />
        <Row
          label="Idle auto-cover"
          description="Cover all screens after a few minutes of no input."
          checked={idleEnabled}
          onChange={toggleIdle}
          trailing={
            idleEnabled && (
              <div className="flex items-center gap-1.5">
                <Input
                  type="number"
                  min={1}
                  max={120}
                  value={idleMinutes}
                  onChange={(e) => changeIdleMinutes(Number(e.target.value) || 1)}
                  className="h-7 w-14 text-xs"
                />
                <span className="text-xs text-text-faint">min</span>
              </div>
            )
          }
        />
        <Row
          label="Lock machine on Away Mode"
          description="In addition to covering screens and keeping the Mac awake."
          checked={lockOnAway}
          onChange={setLockOnAway}
        />
        <Row
          label="Launch at login"
          description="Start Curtain automatically when you sign in."
          checked={autostart}
          onChange={toggleAutostart}
        />
        <Row
          label="Walk-away mode"
          description="Cover screens when your phone moves out of Bluetooth range."
          checked={false}
          onChange={() => {}}
          disabled
          badge="Experimental, later"
        />
      </div>
    </div>
  )
}
