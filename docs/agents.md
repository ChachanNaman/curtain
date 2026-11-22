# Agent Preset Research

Research snapshot as of 2026-11-21. For each tool: process match pattern, official hook/notify mechanism (if any), and a recommendation for how Curtain should integrate.

Caveat: several details below come from WebSearch result summaries rather than a directly-fetched primary doc page (noted per tool). Treat anything marked "unconfirmed" as needing a manual doc check before shipping, not as verified fact.

---

## 1. Gemini CLI (`@google/gemini-cli`)

- **Install**: `npm install -g @google/gemini-cli`
- **Process match**: node process running the `gemini` bin — `ps aux` typically shows `node .../gemini-cli/dist/index.js` or just `gemini` depending on shebang/symlink. Match on argv containing `gemini-cli` or binary name `gemini`.
- **Hook mechanism**: **Yes.** Gemini CLI has an official hooks system plus a simpler notification toggle.
  - Config file: `~/.gemini/settings.json` (user) or `<project>/.gemini/settings.json` (workspace).
  - Simple notification toggle (not a hook, just terminal/OS notification): `general.enableNotifications` (bool) and `general.notificationMethod` (string, default `"auto"`) — fires "for action-required prompts and session completion."
  - Full hooks system: `hooks` object in settings.json, keyed by event name, each entry an array of `{ matcher, sequential, hooks: [{ type: "command", command, name, timeout, description }] }`. Confirmed event names (from official hooks reference page, `geminicli.com/docs/hooks/reference/`): `BeforeTool`, `AfterTool`, `BeforeAgent`, `AfterAgent`, `BeforeModel`, `BeforeToolSelection`, `AfterModel`, `SessionStart`, `SessionEnd`, `Notification`, `PreCompress`. There is **no dedicated "Stop" event** — completion/done signaling is via `AfterAgent`/`SessionEnd`, and "waiting for input" via `Notification`.
  - Docs: [Hooks reference](https://geminicli.com/docs/hooks/reference/), [Hooks overview](https://geminicli.com/docs/hooks/) ([GitHub source](https://github.com/google-gemini/gemini-cli/blob/main/docs/hooks/reference.md)), [Settings reference](https://geminicli.com/docs/cli/settings/).
  - Note: an earlier search surfaced a `hooksConfig.enabled` / `hooksConfig.notifications` pair as a "master toggle" — this could not be independently confirmed against the primary hooks reference doc; treat as unconfirmed.
- **Recommendation**: Add an official Connect button — write a `Notification`/`AfterAgent` hook entry into the user's `~/.gemini/settings.json` that shells out to Curtain's notify endpoint.

---

## 2. Cursor CLI (`cursor-agent`)

- **Install**: Installed via Cursor's own installer/update mechanism (not a standard npm/brew/cargo package found in search); binary name is `cursor-agent`.
- **Process match**: `ps aux` → look for `cursor-agent` in argv.
- **Hook mechanism**: **Yes.**
  - Config file: `~/.cursor/hooks.json` (user-level) or `<project-root>/.cursor/hooks.json` (project-level).
  - Schema: `{ "version": 1, "hooks": { "stop": [ { "command": "./hooks/notify.sh", "loop_limit": 10 } ] } }`. The `stop` event fires when the agent loop completes (status: `"completed" | "aborted" | "error"`). Hook receives JSON on stdin (`conversation_id`, `generation_id`, `model`, `hook_event_name`, `status`, `loop_count`) and may return `{"followup_message": "..."}` on stdout.
  - There is also a separate simple boolean, not a hook: `"notifications": true` in `~/.cursor/cli-config.json`, which sends a terminal notification when the agent finishes or needs input (lower-fidelity than the `stop` hook).
  - Docs: [cursor.com/docs/hooks](https://cursor.com/docs/hooks) (note: `.md` suffix variant 404s — use the non-`.md` URL), [CLI configuration reference](https://cursor.com/docs/cli/reference/configuration), [Cursor forum: how hooks work in Cursor CLI](https://forum.cursor.com/t/how-do-hooks-work-in-cursor-cli/150201).
- **Recommendation**: Add an official Connect button — write a `stop` hook entry into `~/.cursor/hooks.json` pointing at Curtain's notify script.

---

## 3. Aider

- **Install**: `pipx install aider-chat` (also `pip install aider-chat`).
- **Process match**: Python process; `ps aux` shows `python .../aider` or just `aider` (console-script entry point). Match on argv containing `aider`.
- **Hook mechanism**: **Partial — a built-in notification flag, not a generic hook/event system.**
  - Config file: `.aider.conf.yml` (project or home dir), or CLI flags, or env vars.
  - Exact keys: YAML `notifications: true` / `notifications-command: <cmd>`; CLI `--notifications` / `--notifications-command <cmd>`; env `AIDER_NOTIFICATIONS` / `AIDER_NOTIFICATIONS_COMMAND`.
  - Trigger: fires "when the LLM has finished generating a response and is waiting for your input" — i.e. covers both "turn complete" and "waiting for input" as a single event (no separate events, no JSON payload documented — just runs the configured command).
  - Docs: [aider.chat/docs/usage/notifications.html](https://aider.chat/docs/usage/notifications.html).
- **Recommendation**: Add an official Connect button via `--notifications-command` pointing at a small script that pings Curtain — simpler than a hook but functionally sufficient since Aider already distinguishes "done/waiting" from busy.

---

## 4. OpenCode (opencode.ai)

- **Install**: `curl -fsSL https://opencode.ai/install | bash` (no standard npm global package found for the CLI itself; plugins are npm packages).
- **Process match**: binary name `opencode`; `ps aux` shows `opencode` in argv (it's a TUI, so expect a long-running foreground process plus possibly a server/worker subprocess).
- **Hook mechanism**: **Yes, but via a JS/TS plugin system, not a settings.json hook array like Claude Code.**
  - Config file: `opencode.json` (project or global) with a `"plugin": ["pkg-name"]` array that loads plugin packages (e.g. `@aungmyokyaw/opencode-notify`). Plugins are real code (TypeScript), not declarative hook entries.
  - Relevant event (confirmed via multiple third-party plugin-dev guides, not an official opencode.ai doc page I could directly fetch — the official `opencode.ai/docs/events/` page 404'd on fetch): `session.idle` — emitted when a session becomes idle; payload `{ sessionID }`; documented as deprecated-but-still-emitted for backward compatibility and noted to only fire if files were modified. Plugin hook shape: `event: async ({ event }) => { if (event.type === "session.idle") {...} }`.
  - Because there's no built-in "notify on done" setting — it requires writing/installing a plugin — this is a **community-plugin-based** mechanism rather than first-party declarative config. Treat the exact event list as **unconfirmed against primary docs** (the official events/plugins pages could not be fetched directly; info comes from third-party guides and existing community plugins like `opencode-notify`, `opencode-notifier`).
- **Recommendation**: Default to wrapper/PTY approach for v1, OR ship a small Curtain opencode plugin (JS file dropped into the plugin dir) that listens for `session.idle` / `session.error` and posts to Curtain — more reliable than PTY-scraping but requires node/bun plugin loading, so treat as a "semi-official" tier below a pure Connect button.

---

## 5. GitHub Copilot CLI

- **Install**: via GitHub's own installer / `gh` extension mechanism (not confirmed as a plain npm/brew package in search results).
- **Process match**: binary/process name `copilot` (GitHub Copilot CLI); match on argv containing `copilot`.
- **Hook mechanism**: **Yes — official and well-documented.**
  - Config files: repo-level `.github/hooks/<name>.json`; user-level `~/.copilot/hooks/<name>.json` (or `$COPILOT_HOME/hooks/` on Linux/macOS, `%USERPROFILE%\.copilot\hooks\` on Windows).
  - Schema: `{ "version": 1, "hooks": { "<eventName>": [ { "type": "command", "bash": "...", "powershell": "...", "cwd": ".", "timeoutSec": 30, "env": {...} } ] } }`.
  - Confirmed event names: `sessionStart`, `sessionEnd`, `userPromptSubmitted`, `preToolUse`, `postToolUse`, `errorOccurred`, `agentStop` (fires when the agent finishes responding or the CLI quits — this is the "turn complete" signal), plus (per search summary, not independently doc-fetched) `userPromptTransformed`, `postToolUseFailure`, `permissionRequest`, `preCompact`, `subagentStart`, `subagentStop`, `notification`.
  - Docs: [Hooks for GitHub Copilot (concepts)](https://docs.github.com/en/copilot/concepts/agents/hooks), [Using hooks with GitHub Copilot CLI (how-to)](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/use-hooks), [Hooks reference](https://docs.github.com/en/copilot/reference/hooks-reference).
- **Recommendation**: Add an official Connect button — write an `agentStop` (and optionally `notification`) hook entry into `~/.copilot/hooks/curtain.json`.

---

## 6. Amp (sourcegraph/ampcode)

- **Install**: via Amp's own CLI installer (not confirmed as npm/brew in search); binary name `amp`.
- **Process match**: `ps aux` → argv containing `amp`.
- **Hook mechanism**: **No generic event-hook/webhook system found for "done"/"waiting for input."** There is a notification *setting* and a plugin API, but no documented declarative hook array for lifecycle events.
  - Config file: `~/.config/amp/settings.json` (or `.jsonc`), key `amp.notifications.enabled` (bool, default `true`) — "Play notification sounds when the agent completes a task or is blocked waiting for user input." This is a built-in OS/terminal-bell notification, not something Curtain can intercept directly (it plays a sound, doesn't shell out).
  - There is a JS/TS **Plugin API** (`.amp/plugins/` project-level or `~/.config/amp/plugins/` system-level, run via Bun) exposing `ctx.ui.notify(...)` and hooks like `agent.start`, `tool.call`, `tool.result` — but no documented "agent done" or "turn complete" plugin hook was found in the fetched docs.
  - Docs: [Configuration](https://ampcode.com/docs/cli/settings), [Plugin API](https://ampcode.com/docs/plugin-api), [Plugins](https://ampcode.com/docs/customize/plugins).
- **Recommendation**: Default to wrapper/PTY approach — no confirmed official hook for "waiting for input" exists; a Curtain-authored Amp plugin subscribing to agent lifecycle events is a possible future "semi-official" path but wasn't confirmed to expose the right event in docs fetched.

---

## 7. Goose (block/goose)

- **Install**: `pipx install goose`, Homebrew `brew install block-goose-cli`, or `cargo install --path .` from source. Binary name after install: `goose`.
- **Process match**: `ps aux` → argv containing `goose` (or `block-goose-cli` if invoked via the Homebrew-wrapped binary name, but typical invocation is just `goose`).
- **Hook mechanism**: **Yes, via a plugin-based hooks system,** with a usable `SessionEnd` event; no distinct "waiting for input" event confirmed.
  - Hooks ship as part of a plugin: plugin lives at `~/.agents/plugins/<plugin-name>/` (user scope) or `<project>/.agents/plugins/<plugin-name>/` (project scope), with a `hooks/hooks.json` file inside.
  - Schema: `{ "hooks": { "SessionEnd": [ { "hooks": [ { "type": "command", "command": "...", "timeout": 30, "on_failure": "allow" } ], "matcher": "optional regex" } ] } }`.
  - Confirmed events referenced in docs: `SessionEnd`, `PostToolUseFailure`, and others tied to tool/session lifecycle (a tool about to fire, a tool just finished, user submitted a prompt, session started, session ended). The docs explicitly note there is **no notification-equivalent event** distinct from `SessionEnd` — i.e. no dedicated "idle/waiting for input" event was found.
  - Docs: [Hooks](https://goose-docs.ai/docs/guides/context-engineering/hooks/), [Hooks announcement blog post](https://goose-docs.ai/blog/2026/05/14/goose-hooks/).
- **Recommendation**: Add a semi-official Connect integration using a `SessionEnd` hook for "done" detection, but fall back to wrapper/PTY for the "waiting for input" signal since no such hook event is documented.

---

## 8. Qwen Code

- **Install**: `npm install -g @qwen-code/qwen-code`.
- **Process match**: node process; argv containing `qwen-code` or binary name `qwen`.
- **Hook mechanism**: **Yes — explicitly modeled on Claude Code's hook system, including a `Notification` event.**
  - Config file: settings.json — exact path not confirmed from the fetched doc excerpt, but by Qwen Code's own convention (mirrors Gemini CLI, which it forked from) it is almost certainly `~/.qwen/settings.json` (user) / `<project>/.qwen/settings.json` (workspace). **Flag as unconfirmed** — verify against `docs/users/features/hooks.md` directly before shipping.
  - Confirmed event names: `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PostToolBatch`, `UserPromptSubmit`, `SessionStart`, `SessionEnd`, `SessionDelete`, `MessageDisplay`, `Stop`, `Notification`.
  - `Notification` event payload fields: `message`, optional `title`, `additionalContext`; matches types including `permission_prompt`, `idle_prompt`, `auth_success` (and possibly `elicitation_dialog`, truncated in source). The `idle_prompt` type is the "waiting for input" signal; `Stop` is the "turn complete" signal.
  - Global disable switch: top-level `"disableAllHooks": true`.
  - Hooks support `type: "command"` and also `"http"` (with `url`, `headers`, `allowedEnvVars`, `timeout` default 600s) — meaning Qwen Code can POST directly to a webhook URL without a local script, which is notably more Curtain-friendly than most other tools.
  - Docs: [Qwen Code Hooks](https://qwenlm.github.io/qwen-code-docs/en/users/features/hooks/) ([GitHub source](https://github.com/QwenLM/qwen-code/blob/main/docs/users/features/hooks.md)).
- **Recommendation**: Add an official Connect button — best integration of the batch; can likely use the `http` hook type to POST straight to a local Curtain listener for both `Notification` (idle_prompt) and `Stop` events, no shell script needed.

---

## Terminal-output heuristic for "waiting for confirmation" (PTY wrapper fallback)

For tools without a hook mechanism (or as a universal fallback), the common approach is a PTY wrapper that tails stdout and matches on prompt-like patterns at the end of a line/block, e.g.:

- `\(y\/n\)`, `\[y\/N\]`, `\[Y\/n\]` (case-insensitive) — classic yes/no confirmations
- `\?\s*$` — line ending in a question mark (generic prompt)
- `press enter`, `press any key`, `continue\?` — "press to continue" prompts
- `\(approve|allow|deny|reject\)`, `do you want to proceed`, `permission` — agent permission prompts
- A trailing prompt with no newline for N seconds (idle-timeout heuristic) combined with the above regexes reduces false positives, since many agents leave the cursor sitting after printing a prompt without a trailing `\n`.

This is inherently heuristic and tool-specific prompt wording drifts over time, so it should be treated as a lower-confidence fallback behind official hooks wherever available.
