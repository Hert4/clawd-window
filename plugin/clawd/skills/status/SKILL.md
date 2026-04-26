---
description: Show Clawd's current state
---

Call the `clawd_status` MCP tool (from the `clawd` server). No arguments.

The tool returns text like `Clawd: {"state":"idle_living","direction":null,"edge":null}`. Parse it and report the state concisely to the user, e.g.:

- `Clawd đang idle_living`
- `Clawd đang walking (direction: left)` — when direction is set
- `Clawd đang climbing (edge: right)` — when edge is set

If the tool returns "clawd app not running", tell the user `Clawd app chưa chạy`.

Do NOT run any Bash/curl commands — use the MCP tool only.
