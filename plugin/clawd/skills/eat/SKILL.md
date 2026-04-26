---
description: Feed a file/folder to Clawd (moves it to Recycle Bin)
---

Have Clawd eat the path provided in `$ARGUMENTS`. The file/folder is moved to the Recycle Bin (recoverable, NOT permanently deleted). Only paths under the user's home directory are accepted — system paths like `C:\Windows\` are rejected.

Run this Bash command (escape backslashes in the path properly for JSON):

```bash
PATH_ARG=$(printf '%s' "$ARGUMENTS" | sed 's/\\/\\\\/g; s/"/\\"/g')
curl -s -X POST "http://127.0.0.1:${CLAWD_PORT:-9876}/eat" \
  -H "Content-Type: application/json" \
  -d "{\"paths\":[\"$PATH_ARG\"]}"
```

Response is `{"eaten": N}` where N is the number actually moved. If 0, the path was rejected (likely outside home dir or doesn't exist) — check the Clawd app log for the warn line.

If connection refused → Clawd app isn't running.
