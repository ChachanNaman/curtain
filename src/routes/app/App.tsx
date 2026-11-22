import { useEffect, useState } from 'react'
import { AnimatePresence, motion } from 'motion/react'
import {
  Bell,
  Bot,
  Info,
  Keyboard,
  Monitor,
  Shield,
  Zap,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { onNavigate } from '@/lib/tauri'
import Onboarding from '@/routes/onboarding/Onboarding'
import DisplaysSection from './sections/DisplaysSection'
import AgentsSection from './sections/AgentsSection'
import AlertsSection from './sections/AlertsSection'
import PrivacySection from './sections/PrivacySection'
import KeepAwakeSection from './sections/KeepAwakeSection'
import HotkeysSection from './sections/HotkeysSection'
import AboutSection from './sections/AboutSection'

const ONBOARDED_KEY = 'curtain-onboarded'

const TABS = [
  { id: 'displays', label: 'Displays', icon: Monitor, Component: DisplaysSection },
  { id: 'agents', label: 'Agents', icon: Bot, Component: AgentsSection },
  { id: 'alerts', label: 'Alerts', icon: Bell, Component: AlertsSection },
  { id: 'privacy', label: 'Privacy', icon: Shield, Component: PrivacySection },
  { id: 'keep-awake', label: 'Keep Awake', icon: Zap, Component: KeepAwakeSection },
  { id: 'hotkeys', label: 'Hotkeys', icon: Keyboard, Component: HotkeysSection },
  { id: 'about', label: 'About', icon: Info, Component: AboutSection },
] as const

type TabId = (typeof TABS)[number]['id']

export default function App() {
  const [active, setActive] = useState<TabId>('displays')
  const [onboarded, setOnboarded] = useState(() => {
    try {
      return localStorage.getItem(ONBOARDED_KEY) === '1'
    } catch {
      return true
    }
  })
  const ActiveComponent = TABS.find((t) => t.id === active)?.Component ?? DisplaysSection

  useEffect(() => onNavigate((tab) => setActive(tab as TabId)), [])

  if (!onboarded) {
    return (
      <Onboarding
        onDone={() => {
          try {
            localStorage.setItem(ONBOARDED_KEY, '1')
          } catch {
            // localStorage unavailable (private mode, etc.) — just proceed
          }
          setOnboarded(true)
        }}
      />
    )
  }

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-bg text-text">
      <nav className="flex w-[196px] shrink-0 flex-col gap-0.5 border-r border-surface-border px-3 pt-14 pb-4">
        <div className="mb-3 flex items-center gap-2 px-2">
          <span className="text-lg">◐</span>
          <span className="font-display text-sm font-semibold tracking-tight">Curtain</span>
        </div>
        {TABS.map((tab) => {
          const Icon = tab.icon
          const isActive = tab.id === active
          return (
            <button
              key={tab.id}
              onClick={() => setActive(tab.id)}
              className={cn(
                'relative flex items-center gap-2.5 rounded-card px-2.5 py-2 text-left text-sm transition-colors',
                isActive ? 'text-text' : 'text-text-muted hover:text-text',
              )}
            >
              {isActive && (
                <motion.div
                  layoutId="sidebar-active"
                  className="absolute inset-0 rounded-card bg-surface-hover"
                  transition={{ type: 'spring', stiffness: 500, damping: 35 }}
                />
              )}
              <Icon size={15} className="relative z-10" />
              <span className="relative z-10">{tab.label}</span>
            </button>
          )
        })}
      </nav>

      <main className="relative flex-1 overflow-y-auto">
        <AnimatePresence mode="wait">
          <motion.div
            key={active}
            initial={{ opacity: 0, y: 4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.18, ease: 'easeOut' }}
            className="mx-auto max-w-2xl px-10 py-12"
          >
            <ActiveComponent />
          </motion.div>
        </AnimatePresence>
      </main>
    </div>
  )
}
