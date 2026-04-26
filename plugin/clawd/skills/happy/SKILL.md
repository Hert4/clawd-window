---
description: Make Clawd play the happy animation
---

Make the Clawd desktop pet play the happy animation by calling its local HTTP API.

Run this Bash command (don't add commentary, just execute and report the result):

```bash
curl -s -X POST "http://127.0.0.1:${CLAWD_PORT:-9876}/state" \
  -H "Content-Type: application/json" \
  -d '{"state":"happy"}'
```

If the response is `{"ok":true}`, tell the user "Clawd vui rồi 🦀" (in Vietnamese, since this is Hert4's tone). If the response includes `"error"`, tell the user the error.

If curl fails with connection refused, the Clawd app isn't running — tell the user to start it from the system tray.
