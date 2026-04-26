---
description: Force Clawd to sleep
---

Force the Clawd desktop pet into sleep state.

Run this Bash command:

```bash
curl -s -X POST "http://127.0.0.1:${CLAWD_PORT:-9876}/state" \
  -H "Content-Type: application/json" \
  -d '{"state":"sleeping"}'
```

If `{"ok":true}` → tell the user "Clawd đi ngủ 💤". If error → report it.

If connection refused → Clawd app isn't running, ask user to start it from the system tray.
