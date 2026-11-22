import { useEffect, useState } from 'react'
import { motion } from 'motion/react'
import { MonitorX } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Slider } from '@/components/ui/slider'
import {
  coverDisplay,
  getCoverState,
  getDimState,
  getDisplays,
  onCoverState,
  setDim,
  uncoverDisplay,
  type CoverState,
  type DisplayInfo,
} from '@/lib/tauri'

const MAP_WIDTH = 480
const MAP_HEIGHT = 220

function useLayout(displays: DisplayInfo[]) {
  if (displays.length === 0) return { scale: 1, offsetX: 0, offsetY: 0 }
  const minX = Math.min(...displays.map((d) => d.x))
  const minY = Math.min(...displays.map((d) => d.y))
  const maxX = Math.max(...displays.map((d) => d.x + d.width))
  const maxY = Math.max(...displays.map((d) => d.y + d.height))
  const spanX = Math.max(maxX - minX, 1)
  const spanY = Math.max(maxY - minY, 1)
  const padding = 24
  const scale = Math.min(
    (MAP_WIDTH - padding * 2) / spanX,
    (MAP_HEIGHT - padding * 2) / spanY,
  )
  const offsetX = padding + (MAP_WIDTH - padding * 2 - spanX * scale) / 2 - minX * scale
  const offsetY = padding + (MAP_HEIGHT - padding * 2 - spanY * scale) / 2 - minY * scale
  return { scale, offsetX, offsetY }
}

export default function DisplaysSection() {
  const [displays, setDisplays] = useState<DisplayInfo[]>([])
  const [cover, setCover] = useState<CoverState>({ display_count: 0, covered: [] })
  const [dim, setDimLocal] = useState<Record<number, number>>({})
  const [selected, setSelected] = useState<number | null>(null)

  useEffect(() => {
    getDisplays().then((d) => {
      setDisplays(d)
      setSelected((cur) => cur ?? d[0]?.index ?? null)
    })
    getCoverState().then(setCover)
    getDimState().then(setDimLocal)
    return onCoverState(setCover)
  }, [])

  const { scale, offsetX, offsetY } = useLayout(displays)

  async function toggleCover(index: number) {
    if (cover.covered.includes(index)) {
      await uncoverDisplay(index)
    } else {
      await coverDisplay(index)
    }
  }

  async function handleDim(index: number, percent: number) {
    setDimLocal((prev) => ({ ...prev, [index]: percent }))
    await setDim(index, percent)
  }

  return (
    <div className="flex flex-col gap-6">
      <header>
        <h1 className="font-display text-xl font-semibold tracking-tight">Displays</h1>
        <p className="mt-1 text-sm text-text-muted">
          Click a display to cover it. Drag the slider to dim it instead.
        </p>
      </header>

      <div
        className="relative rounded-card border border-surface-border bg-surface"
        style={{ width: MAP_WIDTH, height: MAP_HEIGHT }}
      >
        {displays.map((d) => {
          const covered = cover.covered.includes(d.index)
          const isSelected = selected === d.index
          return (
            <motion.button
              key={d.index}
              onClick={() => {
                setSelected(d.index)
                toggleCover(d.index)
              }}
              whileTap={{ scale: 0.97 }}
              transition={{ type: 'spring', stiffness: 500, damping: 35 }}
              className={cn(
                'absolute flex items-center justify-center overflow-hidden rounded-[6px] border text-[10px] font-medium transition-colors',
                covered
                  ? 'border-stage/50 bg-stage/15 text-stage'
                  : isSelected
                    ? 'border-text-muted/60 bg-surface-hover text-text'
                    : 'border-surface-border bg-white/[0.03] text-text-muted hover:bg-surface-hover',
              )}
              style={{
                left: d.x * scale + offsetX,
                top: d.y * scale + offsetY,
                width: Math.max(d.width * scale, 36),
                height: Math.max(d.height * scale, 24),
              }}
            >
              {covered ? <MonitorX size={13} /> : d.name}
            </motion.button>
          )
        })}
      </div>

      <div className="flex flex-col gap-3">
        {displays.map((d) => {
          const covered = cover.covered.includes(d.index)
          return (
            <div
              key={d.index}
              className={cn(
                'flex flex-col gap-2.5 rounded-card border px-4 py-3.5 transition-colors',
                d.index === selected
                  ? 'border-surface-border bg-surface'
                  : 'border-transparent',
              )}
            >
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-sm font-medium text-text">
                    {d.name}
                    {d.is_main && (
                      <span className="ml-1.5 text-[10px] text-text-faint">main</span>
                    )}
                  </div>
                  <div className="text-xs text-text-muted">
                    {Math.round(d.width)} × {Math.round(d.height)} · {d.scale_factor}×
                  </div>
                </div>
                <button
                  onClick={() => toggleCover(d.index)}
                  className={cn(
                    'rounded-chip px-3 py-1 text-xs font-medium transition-colors',
                    covered ? 'bg-stage/15 text-stage' : 'bg-white/5 text-text-muted hover:bg-white/10',
                  )}
                >
                  {covered ? 'Covered' : 'Cover'}
                </button>
              </div>
              <div className="flex items-center gap-3">
                <span className="w-7 text-[10px] text-text-faint">Dim</span>
                <Slider
                  value={[dim[d.index] ?? 0]}
                  max={100}
                  step={1}
                  onValueChange={(v) => handleDim(d.index, Array.isArray(v) ? v[0] : v)}
                  className="flex-1"
                />
                <span className="w-8 text-right text-[10px] text-text-faint">
                  {dim[d.index] ?? 0}%
                </span>
              </div>
            </div>
          )
        })}
      </div>

    </div>
  )
}
