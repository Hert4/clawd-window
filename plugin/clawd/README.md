# Clawd — Claude Code plugin

Plug the [Clawd desktop pet](https://github.com/Hert4/clawd-window) into Claude Code so it reacts to your coding sessions.

## What this gives you

- **Slash commands** — `/clawd:happy`, `/clawd:sleep`, `/clawd:celebrate`, `/clawd:status`, `/clawd:eat <path>`
- **Hooks (auto-react)** — every time Claude uses a tool, Clawd switches to a working animation; on a system notification (permission prompt), Clawd plays the notification animation; when Claude finishes a task, Clawd celebrates (happy → idle).
- **MCP tools** — Claude can call `clawd_set_state`, `clawd_status`, `clawd_celebrate`, `clawd_eat`, `clawd_health`, `clawd_list_states` autonomously.

## Prerequisites

1. **Clawd desktop app** running — install from [the latest release](https://github.com/Hert4/clawd-window/releases). The plugin talks to the app via `http://127.0.0.1:9876`. If the app isn't running, all calls silently fail (hooks) or return a friendly "app not running" error (skills/MCP).
2. **Node.js ≥ 18** in `PATH`. The MCP server and hook shim are vanilla Node — no npm install needed.
3. **Claude Code** with plugin support (current version).

Verify Node:

```powershell
node --version    # v18+
```

## Install

### Dev mode (test before installing)

From the repo root:

```powershell
claude --plugin-dir "C:\Users\ADMIN\OneDrive\Máy tính\crab\plugin\clawd"
```

### Permanent install

```powershell
# Inside an interactive Claude Code session:
/plugin install "C:\Users\ADMIN\OneDrive\Máy tính\crab\plugin\clawd"
```

After install, run `/plugin reload` if hooks/skills don't show up.

## Usage

Once Clawd app is running and the plugin is loaded:

| Command | What it does |
|---|---|
| `/clawd:happy` | Clawd plays happy animation |
| `/clawd:sleep` | Force Clawd to sleep |
| `/clawd:celebrate` | Happy 1.5s → idle |
| `/clawd:status` | Print current state |
| `/clawd:eat <path>` | Move file/folder to Recycle Bin (must be under `%USERPROFILE%`) |

Hooks fire automatically — you don't need to do anything. Just code with Claude and Clawd will react.

You can also ask Claude in chat: *"check if clawd is running"* → it will invoke the `clawd_health` MCP tool.

## Configuration

| Env var | Default | Purpose |
|---|---|---|
| `CLAWD_PORT` | `9876` | Port the desktop app listens on. Set the same value on both the app and the plugin if you change it. |

Set it in PowerShell before launching Claude Code:

```powershell
$env:CLAWD_PORT = "9999"
claude
```

## Troubleshooting

- **Hooks not firing** — confirm the Clawd app is running (system tray icon). Hooks fail silently when the app is offline so they don't slow Claude Code down.
- **MCP returns "clawd app not running"** — same fix: launch the app.
- **Port conflict** (something else uses 9876) — the app logs `HTTP API bind failed` on startup and continues without HTTP. Set `CLAWD_PORT` to a free port and restart both the app and Claude Code.
- **Hooks visibly thrash the pet** — there's a 250 ms server-side debounce on identical state changes, so it should be smooth. If you still see flicker, check that no other source is also driving Clawd.
- **`/clawd:eat` returns `eaten: 0`** — path was rejected (outside `%USERPROFILE%` or doesn't exist). Check the Clawd app's log window for the warn line.
- **MCP server crashes on init** — make sure `node --version` ≥ 18 (built-in `fetch` is used). Older Node won't work.

## How it's wired

```
Claude Code session
    │
    │  PreToolUse / Stop / Notification
    ├──→ hooks.json
    │       ↓
    │       node bin/hook.mjs <state>     (fire-and-forget, ≤500 ms timeout)
    │       ↓
    │       POST http://127.0.0.1:9876/state
    │
    │  /clawd:* slash commands
    ├──→ skills/<name>/SKILL.md
    │       ↓
    │       Bash tool: curl ...
    │
    │  Claude calls clawd_* tool
    └──→ .mcp.json
            ↓
            node bin/mcp-server.mjs       (stdio JSON-RPC)
            ↓
            POST http://127.0.0.1:9876/...
                   ↓
                   Clawd Tauri app  →  WebView pet animation
```

The HTTP API is local-only (`127.0.0.1`). No auth — single-user assumption.

## Files

```
plugin/clawd/
├── .claude-plugin/plugin.json    # Manifest
├── skills/                       # Slash commands (5)
│   ├── happy/SKILL.md
│   ├── sleep/SKILL.md
│   ├── celebrate/SKILL.md
│   ├── status/SKILL.md
│   └── eat/SKILL.md
├── hooks/hooks.json              # PreToolUse, Notification, Stop
├── .mcp.json                     # MCP server config
├── bin/
│   ├── hook.mjs                  # Fire-and-forget hook shim
│   └── mcp-server.mjs            # Hand-rolled MCP JSON-RPC, no deps
└── README.md
```

## Tested with

- Claude Code (current as of April 2026)
- Clawd v0.1.0
- Node v18+
- Windows 10/11

Other platforms aren't tested — Clawd app itself is Windows-only.
