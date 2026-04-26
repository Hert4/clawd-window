---
description: Show Clawd's current state
---

Query Clawd's current state from the local HTTP API.

Run this Bash command:

```bash
curl -s "http://127.0.0.1:${CLAWD_PORT:-9876}/status"
```

Parse the JSON response (fields: `state`, optional `direction`, optional `edge`) and report it concisely to the user. Example: "Clawd đang `walking` (direction: left)" or "Clawd đang `idle_living`".

If connection refused → tell the user "Clawd app chưa chạy" (app not running).
