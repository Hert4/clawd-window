---
description: Feed a file/folder to Clawd (moves it to Recycle Bin)
---

Call the `clawd_eat` MCP tool (from the `clawd` server) with `paths: [<the path from $ARGUMENTS>]`.

The path comes from `$ARGUMENTS`. Pass it as a single-element array. The tool returns text like `Clawd ate N item(s).`

- If `N >= 1`: confirm to the user, e.g. `Clawd đã ăn $ARGUMENTS — đã vào Recycle Bin.`
- If `N == 0`: the path was rejected (likely outside `%USERPROFILE%` or doesn't exist). Tell the user.
- If "clawd app not running": tell the user to start the Clawd desktop app.

Do NOT run any Bash/curl commands — use the MCP tool only.
