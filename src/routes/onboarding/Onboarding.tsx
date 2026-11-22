import { useEffect, useState } from 'react'
import { motion, AnimatePresence } from 'motion/react'
import { QRCodeSVG } from 'qrcode.react'
import { MonitorX } from 'lucide-react'
import { Kbd } from '@/components/ui/kbd'
import { cn } from '@/lib/utils'
import { connectClaudeCode, connectCodex, getAlertsConfig, type AlertsConfig } from '@/lib/tauri'

const STEPS = ['Hide any screen', 'Never miss a waiting agent', 'Your hotkeys'] as const

function StepDots({ step }: { step: number }) {
  return (
    <div className="flex items-center justify-center gap-1.5">
      {STEPS.map((_, i) => (
        <span
          key={i}
          className={cn(
            'h-1.5 rounded-full transition-all',
            i === step ? 'w-5 bg-stage' : 'w-1.5 bg-white/15',
          )}
        />
      ))}
    </div>
  )
}

function CurtainDemo() {
  const [covered, setCovered] = useState(false)
  useEffect(() => {
    const id = setInterval(() => setCovered((c) => !c), 1800)
    return () => clearInterval(id)
  }, [])
  return (
    <div className="relative h-40 w-64 overflow-hidden rounded-card border border-surface-border bg-surface">
      <div className="flex h-full w-full items-center justify-center text-xs text-text-faint">
        your screen
      </div>
      <AnimatePresence>
        {covered && (
          <motion.div
            initial={{ y: '-100%' }}
            animate={{ y: 0 }}
            exit={{ y: '-100%' }}
            transition={{ duration: 0.42, ease: [0.32, 0.72, 0, 1] }}
            className="absolute inset-0 bg-black"
          >
            <motion.div
              initial={{ opacity: 1 }}
              animate={{ opacity: 0 }}
              transition={{ duration: 0.3, delay: 0.3 }}
              className="h-0.5 w-full bg-stage"
            />
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  )
}

function Step1() {
  return (
    <div className="flex flex-col items-center gap-6 text-center">
      <MonitorX size={22} className="text-stage" />
      <div>
        <h1 className="font-display text-2xl font-semibold tracking-tight">Hide any screen</h1>
        <p className="mt-2 max-w-xs text-sm text-text-muted">
          One tap covers a display in pure black. Apps keep running underneath.
        </p>
      </div>
      <CurtainDemo />
    </div>
  )
}

function Step2() {
  const [config, setConfig] = useState<AlertsConfig | null>(null)
  const [claudeConnected, setClaudeConnected] = useState(false)
  const [codexConnected, setCodexConnected] = useState(false)

  useEffect(() => {
    getAlertsConfig().then(setConfig)
  }, [])

  return (
    <div className="flex flex-col items-center gap-6 text-center">
      <div>
        <h1 className="font-display text-2xl font-semibold tracking-tight">
          Never miss a waiting agent
        </h1>
        <p className="mt-2 max-w-sm text-sm text-text-muted">
          Connect Claude Code or Codex, then scan the code to get pinged on your phone.
        </p>
      </div>

      <div className="flex gap-2.5">
        <button
          onClick={() => connectClaudeCode().then(() => setClaudeConnected(true))}
          className={cn(
            'rounded-chip px-4 py-2 text-xs font-medium transition-colors',
            claudeConnected ? 'bg-ok/15 text-ok' : 'bg-stage/15 text-stage hover:bg-stage/20',
          )}
        >
          {claudeConnected ? 'Claude Code connected' : 'Connect Claude Code'}
        </button>
        <button
          onClick={() => connectCodex().then(() => setCodexConnected(true))}
          className={cn(
            'rounded-chip px-4 py-2 text-xs font-medium transition-colors',
            codexConnected ? 'bg-ok/15 text-ok' : 'bg-stage/15 text-stage hover:bg-stage/20',
          )}
        >
          {codexConnected ? 'Codex connected' : 'Connect Codex'}
        </button>
      </div>

      {config && (
        <div className="rounded-[10px] bg-white p-2">
          <QRCodeSVG value={`https://ntfy.sh/${config.ntfy_topic}`} size={100} />
        </div>
      )}
      <p className="text-xs text-text-faint">Scan with the ntfy app — free, no account needed.</p>
    </div>
  )
}

function Step3() {
  return (
    <div className="flex flex-col items-center gap-6 text-center">
      <div>
        <h1 className="font-display text-2xl font-semibold tracking-tight">Your hotkeys</h1>
        <p className="mt-2 max-w-xs text-sm text-text-muted">
          This one works anywhere, instantly. Try it right now.
        </p>
      </div>
      <div className="flex items-center gap-2 rounded-card border border-surface-border bg-surface px-5 py-4">
        <Kbd>⌃</Kbd>
        <Kbd>⌥</Kbd>
        <Kbd>H</Kbd>
        <span className="ml-2 text-sm text-text-muted">Panic — covers every screen</span>
      </div>
    </div>
  )
}

export default function Onboarding({ onDone }: { onDone?: () => void }) {
  const [step, setStep] = useState(0)
  const isLast = step === STEPS.length - 1

  return (
    <div className="flex h-screen w-screen flex-col items-center justify-center gap-10 bg-bg px-10 text-text">
      <AnimatePresence mode="wait">
        <motion.div
          key={step}
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -8 }}
          transition={{ duration: 0.2, ease: 'easeOut' }}
        >
          {step === 0 ? <Step1 /> : step === 1 ? <Step2 /> : <Step3 />}
        </motion.div>
      </AnimatePresence>

      <div className="flex flex-col items-center gap-4">
        <StepDots step={step} />
        <div className="flex gap-2.5">
          {step > 0 && (
            <button
              onClick={() => setStep((s) => s - 1)}
              className="rounded-chip bg-white/5 px-4 py-2 text-xs font-medium text-text-muted hover:bg-white/10"
            >
              Back
            </button>
          )}
          <button
            onClick={() => (isLast ? onDone?.() : setStep((s) => s + 1))}
            className="rounded-chip bg-stage px-4 py-2 text-xs font-medium text-[#1a1203] hover:opacity-90"
          >
            {isLast ? 'Done' : 'Next'}
          </button>
        </div>
      </div>
    </div>
  )
}
