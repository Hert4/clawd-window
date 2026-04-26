---
description: Clawd celebrates — happy animation 1.5s then back to idle
---

Trigger Clawd's celebrate sequence (happy 1.5s → idle).

Run this Bash command:

```bash
curl -s -X POST "http://127.0.0.1:${CLAWD_PORT:-9876}/celebrate"
```

If `{"ok":true}` → tell the user "Clawd ăn mừng 🎉". If error → report it.

If connection refused → Clawd app isn't running, ask user to start it.
