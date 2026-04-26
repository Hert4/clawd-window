---
description: Force Clawd to sleep
---

Call the `clawd_set_state` MCP tool (from the `clawd` server) with `state: "sleeping"`.

If the tool returns success, reply `Clawd đi ngủ 💤`.
If it returns "clawd app not running", tell the user to start the Clawd desktop app from the system tray.

Do NOT run any Bash/curl commands — use the MCP tool only.
