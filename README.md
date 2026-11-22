# Curtain

[![License: MIT](https://img.shields.io/badge/license-MIT-d97757)](LICENSE)

Hide any screen in one tap. Keep your machine awake while AI agents work. Get pinged the moment an agent is stuck waiting for you.

Mac + Windows · free · open source · under 15 MB · [curtain site](https://chachannaman.github.io/curtain/)

> **Not a screen locker.** Curtain hides what's on screen — it is not a replacement for locking your machine. Anyone with physical access can still use the keyboard to reach what's underneath a cover unless Away Mode's lock option or Touch ID unlock (F8) is on.

## Install

**macOS**

```bash
brew tap chachannaman/tap
brew install --cask curtain
```

Universal (Apple Silicon + Intel). Unsigned — no $99/yr Apple developer account behind this —
but the cask clears the quarantine flag for you, so there's no Gatekeeper prompt to click
through. Prefer to do it by hand? Download the `.dmg` from
[Releases](https://github.com/chachannaman/curtain/releases/latest), then either right-click
the app → Open, or `xattr -cr /Applications/Curtain.app`.

**Windows**

```powershell
scoop bucket add curtain https://github.com/chachannaman/scoop-bucket
scoop install curtain
```

Unsigned — SmartScreen will warn the first run; click "More info" → "Run anyway". Or grab the
`.msi`/`.exe` directly from [Releases](https://github.com/chachannaman/curtain/releases/latest).

**From source**

```bash
git clone https://github.com/chachannaman/curtain.git
cd curtain
npm install
npm run tauri build
```

Curtain checks for updates on request (About tab → Check for updates) — it never auto-updates
or phones home on its own.

## Needs-you alerts

Curtain detects running coding-agent and long-task processes and pings you (native notification, sound, phone push via [ntfy.sh](https://ntfy.sh)) the moment one needs you.

Built-in presets: Claude Code, Codex, Gemini CLI, Cursor CLI, GitHub Copilot CLI, Qwen Code, Aider (all via each tool's own hook mechanism — see `docs/agents.md` for exactly how each one was verified), plus OpenCode, Amp and Goose (via the wrapper CLI below, since none of them ship a confirmed "waiting for input" hook as of this writing).

### Connecting a built-in preset

Open the app window → **Alerts** tab → click **Connect** next to the tool you use. Curtain backs up the tool's config file first (`*.curtain-backup`), merges in just its own hook entry, and never touches anything else already in there. **Disconnect** removes only Curtain's entry.

### The wrapper CLI — for anything without an official hook

```bash
curtain run --agent <id> -- <command...>
```

Runs `<command...>` in a real pseudo-terminal (colors, interactive prompts, line editing all work exactly as if you'd run it directly), passes your terminal through untouched, and reports back to Curtain:

- **done** (exit 0) or **crashed** (non-zero exit, with the exit code) when the command finishes
- **waiting** when output goes quiet for ~20 seconds *and* the last line looks like a prompt (`?`, `(y/n)`, `[Y/n]`, "Press Enter", `approve`, etc.)

Examples:

```bash
# Aider, if you'd rather not use its native --notifications-command
curtain run --agent aider -- aider

# A long training run — add a "Python script / ML training" profile
# in the Agents tab first so it also keeps your Mac awake
curtain run --agent python-task -- python train.py

# Shell chaining works normally — exit code passes through
curtain run --agent build -- npm run build && echo "build ok"
```

The Agents tab shows a ready-to-paste `alias` for every profile set to the wrapper alert source.

### Generic hook — for scripts, git hooks, anything

```bash
curtain-notify --agent <id> --event waiting|done|crashed|custom --message "..."
```

`curtain-notify` is a tiny CLI (symlinked to `~/.local/bin` automatically once you connect any preset, or any time from the Agents tab) that POSTs a single event to Curtain's local listener and exits. It never blocks — if Curtain isn't running, the call just no-ops.

```bash
# chain it onto anything
long-running-command && curtain-notify --agent my-script --event done

# a git post-commit hook
echo 'curtain-notify --agent git --event custom --message "commit landed"' >> .git/hooks/post-commit

# a custom "waiting" ping from your own script
curtain-notify --agent my-script --event custom --message "needs your review"
```

### Adding your own agent/task

App window → **Agents** → **Add agent** → **Custom**. Search running processes, click one to auto-fill match rules, hit **Test** to confirm it matches, name it, pick an icon/color, save. It shows up in the popover and app window immediately, and counts toward "Keep awake while agents run" if you enable that.

## Shortcuts

Every action is rebindable: App window → **Hotkeys**. Click a binding, press your combo, done — it applies live, no restart. Per-display bindings use each monitor's stable hardware ID, so re-plugging a monitor into a different port never swaps which key covers which screen.

The emergency restore (`⌃⌥⌘Esc` on Mac, `Ctrl+Alt+Shift+Esc` on Windows) is shown read-only and can never be disabled or rebound — however badly you misconfigure everything else, that combo always uncovers every screen.

## Privacy / security note

No cloud backend, no accounts, no analytics, no telemetry. The local alert listener binds to `127.0.0.1` only and rejects anything without its token. Phone push goes through the public, free `ntfy.sh` relay by default — the topic is random and user-overridable if you'd rather self-host.
