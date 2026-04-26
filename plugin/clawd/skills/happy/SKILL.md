---
description: Make Clawd play the happy animation
---

Call the `clawd_set_state` MCP tool (from the `clawd` server bundled with this plugin) with `state: "happy"`.

If the tool returns `"Clawd state set to happy."`, reply with `Clawd vui rồi 🦀`.
If it returns an error like "clawd app not running", tell the user to start the Clawd desktop app from the system tray.

Do NOT run any Bash/curl commands — use the MCP tool only.
