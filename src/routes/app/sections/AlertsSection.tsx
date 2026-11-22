import { useEffect, useState } from 'react'
import { QRCodeSVG } from 'qrcode.react'
import { Check } from 'lucide-react'
import { Switch } from '@/components/ui/switch'
import { cn } from '@/lib/utils'
import {
  connectAider,
  connectClaudeCode,
  connectCodex,
  connectCopilot,
  connectCursor,
  connectGemini,
  connectQwen,
  getAlertsConfig,
  isAiderConnected,
  isClaudeConnected,
  isCodexConnected,
  isCopilotConnected,
  isCursorConnected,
  isGeminiConnected,
  isQwenConnected,
  sendTestAlert,
  setAlertChannel,
  type AlertsConfig,
} from '@/lib/tauri'

const HOOK_TOOLS = [
  { name: 'Claude Code', connect: connectClaudeCode, isConnected: isClaudeConnected },
  { name: 'Codex', connect: connectCodex, isConnected: isCodexConnected },
  { name: 'Gemini CLI', connect: connectGemini, isConnected: isGeminiConnected },
  { name: 'Cursor CLI', connect: connectCursor, isConnected: isCursorConnected },
  { name: 'GitHub Copilot CLI', connect: connectCopilot, isConnected: isCopilotConnected },
  { name: 'Qwen Code', connect: connectQwen, isConnected: isQwenConnected },
  { name: 'Aider', connect: connectAider, isConnected: isAiderConnected },
] as const

function ConnectionRow({
  name,
  connected,
  onConnect,
}: {
  name: string
  connected: boolean
  onConnect: () => void
}) {
  return (
    <div className="flex items-center justify-between rounded-card border border-surface-border bg-surface px-4 py-3">
      <span className="text-sm text-text">{name}</span>
      {connected ? (
        <span className="flex items-center gap-1.5 text-xs text-ok">
          <Check size={13} /> Connected
        </span>
      ) : (
        <button
          onClick={onConnect}
          className="rounded-chip bg-stage/15 px-3 py-1 text-xs font-medium text-stage hover:bg-stage/20"
        >
          Connect
        </button>
      )}
    </div>
  )
}

function ChannelToggle({
  label,
  checked,
  onChange,
}: {
  label: string
  checked: boolean
  onChange: (v: boolean) => void
}) {
  return (
    <div className="flex items-center justify-between rounded-card border border-surface-border bg-surface px-4 py-3">
      <span className="text-sm text-text">{label}</span>
      <Switch checked={checked} onCheckedChange={onChange} />
    </div>
  )
}

export default function AlertsSection() {
  const [config, setConfig] = useState<AlertsConfig | null>(null)
  const [connected, setConnected] = useState<Record<string, boolean>>({})
  const [sent, setSent] = useState(false)

  useEffect(() => {
    getAlertsConfig().then(setConfig)
    Promise.all(HOOK_TOOLS.map((t) => t.isConnected())).then((results) => {
      setConnected(Object.fromEntries(HOOK_TOOLS.map((t, i) => [t.name, results[i]])))
    })
  }, [])

  async function toggle(channel: 'notification' | 'sound' | 'ntfy' | 'uncover_on_alert', value: boolean) {
    const next = await setAlertChannel(channel, value)
    setConfig(next)
  }

  async function handleTestAlert() {
    await sendTestAlert()
    setSent(true)
    setTimeout(() => setSent(false), 2000)
  }

  return (
    <div className="flex flex-col gap-6">
      <header>
        <h1 className="font-display text-xl font-semibold tracking-tight">Alerts</h1>
        <p className="mt-1 text-sm text-text-muted">
          Get pinged the moment an agent needs you — on this Mac and on your phone.
        </p>
      </header>

      <section className="flex flex-col gap-2">
        <h2 className="px-0.5 text-[11px] font-medium uppercase tracking-wide text-text-faint">
          Connections
        </h2>
        {HOOK_TOOLS.map((tool) => (
          <ConnectionRow
            key={tool.name}
            name={tool.name}
            connected={connected[tool.name] ?? false}
            onConnect={() => tool.connect().then(() => setConnected((c) => ({ ...c, [tool.name]: true })))}
          />
        ))}
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="px-0.5 text-[11px] font-medium uppercase tracking-wide text-text-faint">
          Channels
        </h2>
        <ChannelToggle
          label="Native notification"
          checked={config?.notification_enabled ?? false}
          onChange={(v) => toggle('notification', v)}
        />
        <ChannelToggle
          label="Sound"
          checked={config?.sound_enabled ?? false}
          onChange={(v) => toggle('sound', v)}
        />
        <ChannelToggle
          label="Phone push (ntfy)"
          checked={config?.ntfy_enabled ?? false}
          onChange={(v) => toggle('ntfy', v)}
        />
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="px-0.5 text-[11px] font-medium uppercase tracking-wide text-text-faint">
          When an agent is waiting
        </h2>
        <ChannelToggle
          label="Uncover its display automatically"
          checked={config?.uncover_on_alert_enabled ?? false}
          onChange={(v) => toggle('uncover_on_alert', v)}
        />
        <p className="px-0.5 text-xs text-text-faint">
          Lifts the cover only on the display showing that agent's terminal — every other covered
          display stays covered.
        </p>
      </section>

      {config?.ntfy_enabled && config.ntfy_topic && (
        <section className="flex items-center gap-4 rounded-card border border-surface-border bg-surface px-5 py-4">
          <div className="rounded-[10px] bg-white p-2">
            <QRCodeSVG value={`https://ntfy.sh/${config.ntfy_topic}`} size={84} />
          </div>
          <div className="text-xs text-text-muted">
            <p className="text-sm text-text">Scan with the ntfy app</p>
            <p className="mt-1">
              Topic: <span className="font-mono text-text-muted">{config.ntfy_topic}</span>
            </p>
            <p className="mt-1">Free, no account — self-hostable later.</p>
          </div>
        </section>
      )}

      <button
        onClick={handleTestAlert}
        className={cn(
          'self-start rounded-chip px-4 py-2 text-xs font-medium transition-colors',
          sent ? 'bg-ok/15 text-ok' : 'bg-white/5 text-text-muted hover:bg-white/10',
        )}
      >
        {sent ? 'Sent!' : 'Send test alert'}
      </button>
    </div>
  )
}
