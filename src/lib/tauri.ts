/**
 * Typed wrappers around Tauri commands/events. In the browser (no
 * __TAURI_INTERNALS__), these fall back to an in-memory mock so the UI can
 * be built and screenshotted with `npm run dev` outside the native shell.
 */
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification'
import { check as checkUpdate } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

export type CoverState = {
  display_count: number
  covered: number[]
}

export type KeepAwakeMode = 'off' | 'while_agents_run' | 'always' | 'timer'

export type AgentStatus = 'running' | 'waiting' | 'idle_maybe' | 'done' | 'crashed'

export type Agent = {
  key: string
  display_name: string
  icon: string
  color: string
  pid: number
  started_at_ms: number
  status: AgentStatus
  exit_code: number | null
}

export type MatchRule = {
  process_name?: string | null
  argv_contains?: string | null
  cwd_contains?: string | null
}

export type AlertSource = 'hooks' | 'wrapper' | 'idle_heuristic' | 'none'

export type NotifyConfig = {
  on_waiting: boolean
  on_done: boolean
  on_crash: boolean
}

export type AgentProfile = {
  id: string
  display_name: string
  icon: string
  color: string
  match_rules: MatchRule[]
  alert_source: AlertSource
  keep_awake: boolean
  notify: NotifyConfig
  enabled: boolean
  is_preset: boolean
}

export type RunningProcess = {
  pid: number
  name: string
  cmd: string
  cwd: string | null
}

export type HistoryEntry = {
  id: string
  agent_key: string
  display_name: string
  started_at_ms: number
  waiting_ms: number
  waiting_started_at_ms: number | null
  ended_at_ms: number | null
  end_status: 'done' | 'crashed' | null
  exit_code: number | null
}

export type HotkeyRow = {
  id: string
  label: string
  combo: string | null
  default_combo: string | null
  enabled: boolean
  is_display: boolean
  display_name: string | null
  display_connected: boolean | null
}

export type SetHotkeyResult = {
  ok: boolean
  reason: string | null
  conflicting_action_label: string | null
}

export type AlertsConfig = {
  token: string
  ntfy_topic: string
  notification_enabled: boolean
  sound_enabled: boolean
  ntfy_enabled: boolean
  uncover_on_alert_enabled: boolean
}

export type DisplayInfo = {
  index: number
  name: string
  x: number
  y: number
  width: number
  height: number
  scale_factor: number
  is_main: boolean
}

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

/**
 * macOS/Windows notification banners require the OS-level consent flow
 * to run through this JS-side plugin API at least once — firing
 * `.show()` from the Rust side alone never triggers the permission
 * prompt and the notification is silently dropped. Safe to call on
 * every launch: it's a no-op once already granted or denied.
 */
export async function ensureNotificationPermission(): Promise<void> {
  if (!isTauri) return
  if (await isPermissionGranted()) return
  await requestPermission()
}

// --- Mock backend (dev/Playwright only) -----------------------------------

const mock = {
  state: {
    display_count: 2,
    covered: [] as number[],
  } as CoverState,
  awakeMode: 'while_agents_run' as KeepAwakeMode,
  listeners: new Set<(s: CoverState) => void>(),
  agents: [
    { key: 'claude', display_name: 'Claude Code', icon: 'terminal', color: '#D97757', pid: 4821, started_at_ms: Date.now() - 23 * 60_000, status: 'running', exit_code: null },
    { key: 'codex', display_name: 'Codex', icon: 'terminal', color: '#74AA9C', pid: 4902, started_at_ms: Date.now() - 4 * 60_000, status: 'waiting', exit_code: null },
    { key: 'python-task', display_name: 'Python script / ML training', icon: 'flask-conical', color: '#A1A1AA', pid: 5110, started_at_ms: Date.now() - 55 * 60_000, status: 'idle_maybe', exit_code: null },
  ] as Agent[],
  agentListeners: new Set<(a: Agent[]) => void>(),
  agentProfiles: [
    { id: 'claude', display_name: 'Claude Code', icon: 'terminal', color: '#D97757', match_rules: [{ process_name: 'claude' }], alert_source: 'hooks', keep_awake: true, notify: { on_waiting: true, on_done: true, on_crash: true }, enabled: true, is_preset: true },
    { id: 'codex', display_name: 'Codex', icon: 'terminal', color: '#74AA9C', match_rules: [{ process_name: 'codex' }], alert_source: 'hooks', keep_awake: true, notify: { on_waiting: true, on_done: true, on_crash: true }, enabled: true, is_preset: true },
    { id: 'gemini', display_name: 'Gemini CLI', icon: 'sparkles', color: '#4C8DF6', match_rules: [{ process_name: 'gemini' }], alert_source: 'hooks', keep_awake: true, notify: { on_waiting: true, on_done: true, on_crash: true }, enabled: true, is_preset: true },
    { id: 'cursor', display_name: 'Cursor CLI', icon: 'mouse-pointer', color: '#6E6ADE', match_rules: [{ process_name: 'cursor-agent' }], alert_source: 'hooks', keep_awake: true, notify: { on_waiting: true, on_done: true, on_crash: true }, enabled: true, is_preset: true },
    { id: 'copilot', display_name: 'GitHub Copilot CLI', icon: 'github', color: '#8957E5', match_rules: [{ process_name: 'copilot' }], alert_source: 'hooks', keep_awake: true, notify: { on_waiting: true, on_done: true, on_crash: true }, enabled: true, is_preset: true },
    { id: 'qwen', display_name: 'Qwen Code', icon: 'bot', color: '#6F42C1', match_rules: [{ process_name: 'qwen' }], alert_source: 'hooks', keep_awake: true, notify: { on_waiting: true, on_done: true, on_crash: true }, enabled: true, is_preset: true },
    { id: 'aider', display_name: 'Aider', icon: 'pencil-line', color: '#F2A33C', match_rules: [{ process_name: 'aider' }], alert_source: 'hooks', keep_awake: true, notify: { on_waiting: true, on_done: true, on_crash: true }, enabled: true, is_preset: true },
    { id: 'opencode', display_name: 'OpenCode', icon: 'code', color: '#53B48A', match_rules: [{ process_name: 'opencode' }], alert_source: 'wrapper', keep_awake: true, notify: { on_waiting: true, on_done: true, on_crash: true }, enabled: true, is_preset: true },
    { id: 'amp', display_name: 'Amp', icon: 'zap', color: '#D97757', match_rules: [{ process_name: 'amp' }], alert_source: 'wrapper', keep_awake: true, notify: { on_waiting: true, on_done: true, on_crash: true }, enabled: true, is_preset: true },
    { id: 'goose', display_name: 'Goose', icon: 'feather', color: '#60A5FA', match_rules: [{ process_name: 'goose' }], alert_source: 'wrapper', keep_awake: true, notify: { on_waiting: true, on_done: true, on_crash: true }, enabled: true, is_preset: true },
    { id: 'build', display_name: 'Build', icon: 'hammer', color: '#A1A1AA', match_rules: [{ process_name: 'npm' }, { process_name: 'cargo' }], alert_source: 'wrapper', keep_awake: true, notify: { on_waiting: true, on_done: true, on_crash: true }, enabled: false, is_preset: true },
    { id: 'python-task', display_name: 'Python script / ML training', icon: 'flask-conical', color: '#A1A1AA', match_rules: [{ process_name: 'python' }], alert_source: 'wrapper', keep_awake: true, notify: { on_waiting: true, on_done: true, on_crash: true }, enabled: false, is_preset: true },
    { id: 'render', display_name: 'Render / export', icon: 'film', color: '#A1A1AA', match_rules: [{ process_name: 'ffmpeg' }], alert_source: 'wrapper', keep_awake: true, notify: { on_waiting: true, on_done: true, on_crash: true }, enabled: false, is_preset: true },
  ] as AgentProfile[],
  runningProcesses: [
    { pid: 123, name: 'python3', cmd: 'python3 train.py --epochs 50', cwd: '/Users/me/ml-project' },
    { pid: 456, name: 'node', cmd: 'node server.js', cwd: '/Users/me/app' },
    { pid: 789, name: 'ffmpeg', cmd: 'ffmpeg -i in.mov -c:v libx264 out.mp4', cwd: '/Users/me/videos' },
  ] as RunningProcess[],
  history: [
    {
      id: 'h1',
      agent_key: 'claude',
      display_name: 'Claude Code',
      started_at_ms: Date.now() - 1000 * 60 * 42,
      waiting_ms: 1000 * 60 * 3,
      waiting_started_at_ms: null,
      ended_at_ms: Date.now() - 1000 * 60 * 20,
      end_status: 'done',
      exit_code: 0,
    },
    {
      id: 'h2',
      agent_key: 'codex',
      display_name: 'Codex',
      started_at_ms: Date.now() - 1000 * 60 * 90,
      waiting_ms: 1000 * 60 * 6,
      waiting_started_at_ms: null,
      ended_at_ms: Date.now() - 1000 * 60 * 61,
      end_status: 'crashed',
      exit_code: 1,
    },
  ] as HistoryEntry[],
  hotkeys: [
    { id: 'panic', label: 'Panic: cover all', combo: 'ctrl+alt+h', default_combo: 'ctrl+alt+h', enabled: true, is_display: false, display_name: null, display_connected: null },
    { id: 'restore_all', label: 'Restore all', combo: 'ctrl+alt+r', default_combo: 'ctrl+alt+r', enabled: true, is_display: false, display_name: null, display_connected: null },
    { id: 'away_mode', label: 'Away Mode', combo: 'ctrl+alt+a', default_combo: 'ctrl+alt+a', enabled: true, is_display: false, display_name: null, display_connected: null },
    { id: 'toggle_keep_awake', label: 'Toggle keep-awake', combo: 'ctrl+alt+k', default_combo: 'ctrl+alt+k', enabled: true, is_display: false, display_name: null, display_connected: null },
    { id: 'open_popover', label: 'Open popover', combo: 'ctrl+alt+c', default_combo: 'ctrl+alt+c', enabled: true, is_display: false, display_name: null, display_connected: null },
    { id: 'open_app_window', label: 'Open app window', combo: 'ctrl+alt+shift+c', default_combo: 'ctrl+alt+shift+c', enabled: true, is_display: false, display_name: null, display_connected: null },
    { id: 'jump_to_waiting_agent', label: 'Jump to waiting agent', combo: 'ctrl+alt+j', default_combo: 'ctrl+alt+j', enabled: true, is_display: false, display_name: null, display_connected: null },
    { id: 'dim_up', label: 'Dim up (focused display)', combo: 'ctrl+alt+equal', default_combo: 'ctrl+alt+equal', enabled: true, is_display: false, display_name: null, display_connected: null },
    { id: 'dim_down', label: 'Dim down (focused display)', combo: 'ctrl+alt+minus', default_combo: 'ctrl+alt+minus', enabled: true, is_display: false, display_name: null, display_connected: null },
    { id: 'display:mock-uuid-1', label: 'Built-in Display', combo: 'ctrl+alt+1', default_combo: 'ctrl+alt+1', enabled: true, is_display: true, display_name: 'Built-in Display', display_connected: true },
    { id: 'display:mock-uuid-2', label: 'Dell U2720Q', combo: 'ctrl+alt+2', default_combo: 'ctrl+alt+2', enabled: true, is_display: true, display_name: 'Dell U2720Q', display_connected: true },
    { id: 'display:mock-uuid-3', label: 'Old Monitor', combo: 'ctrl+alt+3', default_combo: 'ctrl+alt+3', enabled: true, is_display: true, display_name: 'Not connected', display_connected: false },
  ] as HotkeyRow[],
  hotkeysListeners: new Set<(rows: HotkeyRow[]) => void>(),
  alertsConfig: {
    token: 'mock-token',
    ntfy_topic: 'curtain-mock-topic',
    notification_enabled: true,
    sound_enabled: true,
    ntfy_enabled: true,
    uncover_on_alert_enabled: true,
  } as AlertsConfig,
  claudeConnected: false,
  codexConnected: false,
  displays: [
    { index: 0, name: 'Built-in Display', x: 0, y: 0, width: 1512, height: 982, scale_factor: 2, is_main: true },
    { index: 1, name: 'Dell U2720Q', x: 1512, y: -120, width: 2560, height: 1440, scale_factor: 1, is_main: false },
  ] as DisplayInfo[],
  dim: new Map<number, number>(),
}

function mockEmit() {
  mock.listeners.forEach((fn) => fn({ ...mock.state, covered: [...mock.state.covered] }))
}

// --- Cover (F1) -------------------------------------------------------------

export async function coverDisplay(index: number): Promise<void> {
  if (!isTauri) {
    if (!mock.state.covered.includes(index)) mock.state.covered.push(index)
    mockEmit()
    return
  }
  await invoke('cover_display', { index })
}

export async function uncoverDisplay(index: number): Promise<void> {
  if (!isTauri) {
    mock.state.covered = mock.state.covered.filter((i) => i !== index)
    mockEmit()
    return
  }
  await invoke('uncover_display', { index })
}

export async function coverAll(): Promise<void> {
  if (!isTauri) {
    mock.state.covered = Array.from({ length: mock.state.display_count }, (_, i) => i)
    mockEmit()
    return
  }
  await invoke('cover_all')
}

export async function uncoverAll(): Promise<void> {
  if (!isTauri) {
    mock.state.covered = []
    mockEmit()
    return
  }
  await invoke('uncover_all')
}

export async function getCoverState(): Promise<CoverState> {
  if (!isTauri) return { ...mock.state, covered: [...mock.state.covered] }
  return invoke('cover_state')
}

export function onCoverState(cb: (state: CoverState) => void): () => void {
  if (!isTauri) {
    mock.listeners.add(cb)
    return () => mock.listeners.delete(cb)
  }
  let unlisten: (() => void) | undefined
  listen<CoverState>('cover://state', (e) => cb(e.payload)).then((fn) => {
    unlisten = fn
  })
  return () => unlisten?.()
}

// --- Keep awake (F3) --------------------------------------------------------

export async function setKeepAwakeMode(mode: KeepAwakeMode, timerMinutes?: number): Promise<void> {
  if (!isTauri) {
    mock.awakeMode = mode
    return
  }
  await invoke('set_keep_awake_mode', { mode, timerMinutes })
}

export async function getKeepAwakeMode(): Promise<KeepAwakeMode> {
  if (!isTauri) return mock.awakeMode
  return invoke('get_keep_awake_mode')
}

// --- Agents (F4) -------------------------------------------------------------

export async function getAgents(): Promise<Agent[]> {
  if (!isTauri) return mock.agents
  return invoke('get_agents')
}

export function onAgentsState(cb: (agents: Agent[]) => void): () => void {
  if (!isTauri) {
    mock.agentListeners.add(cb)
    return () => mock.agentListeners.delete(cb)
  }
  let unlisten: (() => void) | undefined
  listen<Agent[]>('agents://state', (e) => cb(e.payload)).then((fn) => {
    unlisten = fn
  })
  return () => unlisten?.()
}

// --- Alerts (F5) -------------------------------------------------------------

export type AlertPayload = { agent_key: string; agent_name: string; message: string }

export async function getAlertsConfig(): Promise<AlertsConfig> {
  if (!isTauri) return mock.alertsConfig
  return invoke('get_alerts_config')
}

export async function setAlertChannel(
  channel: 'notification' | 'sound' | 'ntfy' | 'uncover_on_alert',
  enabled: boolean,
): Promise<AlertsConfig> {
  if (!isTauri) {
    mock.alertsConfig = { ...mock.alertsConfig, [`${channel}_enabled`]: enabled }
    return mock.alertsConfig
  }
  return invoke('set_alert_channel', { channel, enabled })
}

export async function sendTestAlert(): Promise<void> {
  if (!isTauri) return
  await invoke('send_test_alert')
}

export function onAlertNotify(cb: (payload: AlertPayload) => void): () => void {
  if (!isTauri) return () => {}
  let unlisten: (() => void) | undefined
  listen<AlertPayload>('alerts://notify', (e) => cb(e.payload)).then((fn) => {
    unlisten = fn
  })
  return () => unlisten?.()
}

export async function connectClaudeCode(): Promise<void> {
  if (!isTauri) {
    mock.claudeConnected = true
    return
  }
  await invoke('connect_claude_code')
}

export async function disconnectClaudeCode(): Promise<void> {
  if (!isTauri) {
    mock.claudeConnected = false
    return
  }
  await invoke('disconnect_claude_code')
}

export async function isClaudeConnected(): Promise<boolean> {
  if (!isTauri) return mock.claudeConnected
  return invoke('claude_connected')
}

export async function connectCodex(): Promise<void> {
  if (!isTauri) {
    mock.codexConnected = true
    return
  }
  await invoke('connect_codex')
}

export async function disconnectCodex(): Promise<void> {
  if (!isTauri) {
    mock.codexConnected = false
    return
  }
  await invoke('disconnect_codex')
}

export async function isCodexConnected(): Promise<boolean> {
  if (!isTauri) return mock.codexConnected
  return invoke('codex_connected')
}

// --- Displays + dim (F2) -----------------------------------------------------

export async function getDisplays(): Promise<DisplayInfo[]> {
  if (!isTauri) return mock.displays
  return invoke('get_displays')
}

export async function setDim(index: number, percent: number): Promise<void> {
  if (!isTauri) {
    mock.dim.set(index, percent)
    return
  }
  await invoke('set_dim', { index, percent })
}

export async function getDimState(): Promise<Record<number, number>> {
  if (!isTauri) return Object.fromEntries(mock.dim)
  const pairs = await invoke<[number, number][]>('dim_state')
  return Object.fromEntries(pairs)
}

// --- App window ---------------------------------------------------------------

export async function openAppWindow(): Promise<void> {
  if (!isTauri) return
  await invoke('open_app_window')
}

// --- Extra agent hook connections --------------------------------------------

function mockConnection(flagKey: 'gemini' | 'cursor' | 'copilot' | 'qwen' | 'aider') {
  const flags: Record<string, boolean> = (mock as unknown as { hookFlags: Record<string, boolean> }).hookFlags ??
    ((mock as unknown as { hookFlags: Record<string, boolean> }).hookFlags = {})
  return {
    connect: async () => {
      flags[flagKey] = true
    },
    disconnect: async () => {
      flags[flagKey] = false
    },
    isConnected: async () => flags[flagKey] ?? false,
  }
}

export const connectGemini = () => (isTauri ? invoke('connect_gemini') : mockConnection('gemini').connect())
export const disconnectGemini = () => (isTauri ? invoke('disconnect_gemini') : mockConnection('gemini').disconnect())
export const isGeminiConnected = (): Promise<boolean> =>
  isTauri ? invoke('gemini_connected') : mockConnection('gemini').isConnected()

export const connectCursor = () => (isTauri ? invoke('connect_cursor') : mockConnection('cursor').connect())
export const disconnectCursor = () => (isTauri ? invoke('disconnect_cursor') : mockConnection('cursor').disconnect())
export const isCursorConnected = (): Promise<boolean> =>
  isTauri ? invoke('cursor_connected') : mockConnection('cursor').isConnected()

export const connectCopilot = () => (isTauri ? invoke('connect_copilot') : mockConnection('copilot').connect())
export const disconnectCopilot = () => (isTauri ? invoke('disconnect_copilot') : mockConnection('copilot').disconnect())
export const isCopilotConnected = (): Promise<boolean> =>
  isTauri ? invoke('copilot_connected') : mockConnection('copilot').isConnected()

export const connectQwen = () => (isTauri ? invoke('connect_qwen') : mockConnection('qwen').connect())
export const disconnectQwen = () => (isTauri ? invoke('disconnect_qwen') : mockConnection('qwen').disconnect())
export const isQwenConnected = (): Promise<boolean> =>
  isTauri ? invoke('qwen_connected') : mockConnection('qwen').isConnected()

export const connectAider = () => (isTauri ? invoke('connect_aider') : mockConnection('aider').connect())
export const disconnectAider = () => (isTauri ? invoke('disconnect_aider') : mockConnection('aider').disconnect())
export const isAiderConnected = (): Promise<boolean> =>
  isTauri ? invoke('aider_connected') : mockConnection('aider').isConnected()

// --- Agent profiles (F4) ------------------------------------------------------

export async function getAgentProfiles(): Promise<AgentProfile[]> {
  if (!isTauri) return mock.agentProfiles
  return invoke('get_agent_profiles')
}

export async function upsertAgentProfile(profile: AgentProfile): Promise<void> {
  if (!isTauri) {
    const i = mock.agentProfiles.findIndex((p) => p.id === profile.id)
    if (i >= 0) mock.agentProfiles[i] = profile
    else mock.agentProfiles.push(profile)
    return
  }
  await invoke('upsert_agent_profile', { profile })
}

export async function deleteAgentProfile(id: string): Promise<void> {
  if (!isTauri) {
    const p = mock.agentProfiles.find((p) => p.id === id)
    if (p?.is_preset) throw 'Presets can only be disabled, not deleted.'
    mock.agentProfiles = mock.agentProfiles.filter((p) => p.id !== id)
    return
  }
  await invoke('delete_agent_profile', { id })
}

export async function setAgentProfileEnabled(id: string, enabled: boolean): Promise<void> {
  if (!isTauri) {
    const p = mock.agentProfiles.find((p) => p.id === id)
    if (p) p.enabled = enabled
    return
  }
  await invoke('set_agent_profile_enabled', { id, enabled })
}

export async function duplicateAgentProfile(id: string): Promise<void> {
  if (!isTauri) {
    const src = mock.agentProfiles.find((p) => p.id === id)
    if (src) {
      mock.agentProfiles.push({
        ...src,
        id: `${src.id}-copy-${Math.random().toString(36).slice(2, 8)}`,
        display_name: `${src.display_name} (copy)`,
        is_preset: false,
      })
    }
    return
  }
  await invoke('duplicate_agent_profile', { id })
}

export async function restorePresetAgents(): Promise<void> {
  if (!isTauri) return
  await invoke('restore_preset_agents')
}

export async function listRunningProcesses(): Promise<RunningProcess[]> {
  if (!isTauri) return mock.runningProcesses
  return invoke('list_running_processes')
}

export async function testMatchRules(rules: MatchRule[]): Promise<number> {
  if (!isTauri) {
    return mock.runningProcesses.filter((p) =>
      rules.some((r) => {
        const name = p.name.toLowerCase()
        const cmd = p.cmd.toLowerCase()
        const cwd = (p.cwd ?? '').toLowerCase()
        let any = false
        if (r.process_name) {
          any = true
          if (!name.includes(r.process_name.toLowerCase())) return false
        }
        if (r.argv_contains) {
          any = true
          if (!cmd.includes(r.argv_contains.toLowerCase())) return false
        }
        if (r.cwd_contains) {
          any = true
          if (!cwd.includes(r.cwd_contains.toLowerCase())) return false
        }
        return any
      }),
    ).length
  }
  return invoke('test_match_rules', { rules })
}

// --- Agent history + time-saved stat (F11) ---------------------------------

export async function getAgentHistory(agentKey?: string): Promise<HistoryEntry[]> {
  if (!isTauri) {
    return agentKey ? mock.history.filter((h) => h.agent_key === agentKey) : mock.history
  }
  return invoke('get_agent_history', { agentKey: agentKey ?? null })
}

export async function getTimeSavedThisWeekMs(): Promise<number> {
  if (!isTauri) {
    return mock.history.reduce((sum, h) => sum + h.waiting_ms, 0)
  }
  return invoke('get_time_saved_this_week_ms')
}

// --- Hotkeys (F7) --------------------------------------------------------------

function mockHotkeysEmit() {
  mock.hotkeysListeners.forEach((fn) => fn(mock.hotkeys.map((h) => ({ ...h }))))
}

export async function getHotkeys(): Promise<HotkeyRow[]> {
  if (!isTauri) return mock.hotkeys
  return invoke('get_hotkeys')
}

export async function setHotkey(actionId: string, combo: string): Promise<SetHotkeyResult> {
  if (!isTauri) {
    const conflict = mock.hotkeys.find((h) => h.id !== actionId && h.enabled && h.combo === combo)
    if (conflict) {
      return { ok: false, reason: 'conflict', conflicting_action_label: conflict.label }
    }
    const row = mock.hotkeys.find((h) => h.id === actionId)
    if (row) row.combo = combo
    mockHotkeysEmit()
    return { ok: true, reason: null, conflicting_action_label: null }
  }
  return invoke('set_hotkey', { actionId, combo })
}

export async function clearHotkey(actionId: string): Promise<void> {
  if (!isTauri) {
    const row = mock.hotkeys.find((h) => h.id === actionId)
    if (row) row.combo = null
    mockHotkeysEmit()
    return
  }
  await invoke('clear_hotkey', { actionId })
}

export async function setHotkeyEnabled(actionId: string, enabled: boolean): Promise<void> {
  if (!isTauri) {
    const row = mock.hotkeys.find((h) => h.id === actionId)
    if (row) row.enabled = enabled
    mockHotkeysEmit()
    return
  }
  await invoke('set_hotkey_enabled', { actionId, enabled })
}

export async function resetHotkey(actionId: string): Promise<void> {
  if (!isTauri) {
    const row = mock.hotkeys.find((h) => h.id === actionId)
    if (row) row.combo = row.default_combo
    mockHotkeysEmit()
    return
  }
  await invoke('reset_hotkey', { actionId })
}

export async function resetAllHotkeys(): Promise<void> {
  if (!isTauri) {
    mock.hotkeys.forEach((h) => {
      h.combo = h.default_combo
      h.enabled = true
    })
    mockHotkeysEmit()
    return
  }
  await invoke('reset_all_hotkeys')
}

export function onHotkeysState(cb: (rows: HotkeyRow[]) => void): () => void {
  if (!isTauri) {
    mock.hotkeysListeners.add(cb)
    return () => mock.hotkeysListeners.delete(cb)
  }
  let unlisten: (() => void) | undefined
  listen<HotkeyRow[]>('hotkeys://state', (e) => cb(e.payload)).then((fn) => {
    unlisten = fn
  })
  return () => unlisten?.()
}

// --- Navigation (jump_to_waiting_agent hotkey) ------------------------------

export function onNavigate(cb: (tab: string) => void): () => void {
  if (!isTauri) return () => {}
  let unlisten: (() => void) | undefined
  listen<string>('app://navigate', (e) => cb(e.payload)).then((fn) => {
    unlisten = fn
  })
  return () => unlisten?.()
}

// --- Privacy: Touch ID (F8), idle auto-cover (F10), autostart (F9) ----------

const privacyMock = {
  unlockRequired: false,
  idleAutoCover: { enabled: false, minutes: 10 },
  autostart: false,
}

export async function setUnlockRequired(enabled: boolean): Promise<void> {
  if (!isTauri) {
    privacyMock.unlockRequired = enabled
    return
  }
  await invoke('set_unlock_required', { enabled })
}

export async function isUnlockRequired(): Promise<boolean> {
  if (!isTauri) return privacyMock.unlockRequired
  return invoke('is_unlock_required')
}

export async function setIdleAutoCover(enabled: boolean, minutes: number): Promise<void> {
  if (!isTauri) {
    privacyMock.idleAutoCover = { enabled, minutes }
    return
  }
  await invoke('set_idle_auto_cover', { enabled, minutes })
}

export async function getIdleAutoCover(): Promise<{ enabled: boolean; minutes: number }> {
  if (!isTauri) return privacyMock.idleAutoCover
  const [enabled, minutes] = await invoke<[boolean, number]>('get_idle_auto_cover')
  return { enabled, minutes }
}

export async function setAutostart(enabled: boolean): Promise<void> {
  if (!isTauri) {
    privacyMock.autostart = enabled
    return
  }
  await invoke('set_autostart', { enabled })
}

export async function isAutostartEnabled(): Promise<boolean> {
  if (!isTauri) return privacyMock.autostart
  return invoke('is_autostart_enabled')
}

// --- Settings export/import (shortcuts + agent profiles combined) ----------

export async function exportSettings(): Promise<boolean> {
  if (!isTauri) return false
  return invoke('export_settings')
}

export async function importSettings(): Promise<boolean> {
  if (!isTauri) return false
  return invoke('import_settings')
}

// --- Auto-update (F12) ------------------------------------------------------

export type UpdateInfo = { available: boolean; version?: string; body?: string }

export async function checkForUpdate(): Promise<UpdateInfo> {
  if (!isTauri) return { available: false }
  const update = await checkUpdate()
  if (!update) return { available: false }
  return { available: true, version: update.version, body: update.body }
}

export async function installUpdateAndRelaunch(onProgress?: (pct: number) => void): Promise<void> {
  if (!isTauri) return
  const update = await checkUpdate()
  if (!update) return
  let total = 0
  let downloaded = 0
  await update.downloadAndInstall((event) => {
    if (event.event === 'Started') total = event.data.contentLength ?? 0
    if (event.event === 'Progress') downloaded += event.data.chunkLength
    if (total > 0) onProgress?.(Math.round((downloaded / total) * 100))
  })
  await relaunch()
}

export function onProfilesChanged(cb: () => void): () => void {
  if (!isTauri) return () => {}
  let unlisten: (() => void) | undefined
  listen('agents://profiles-changed', () => cb()).then((fn) => {
    unlisten = fn
  })
  return () => unlisten?.()
}
