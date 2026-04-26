#!/usr/bin/env node
// Fire-and-forget hook shim. Posts a state change to Clawd's local HTTP API.
//
// Usage:
//   node hook.mjs <state-key>           -> POST /state {state: "<state-key>"}
//   node hook.mjs --celebrate           -> POST /celebrate
//
// Always exits 0. Errors (Clawd not running, network failure) are silently swallowed
// so they don't block the Claude Code event loop.

const port = process.env.CLAWD_PORT || 9876;
const arg = process.argv[2];

if (!arg) process.exit(0);

const url = arg === "--celebrate"
  ? `http://127.0.0.1:${port}/celebrate`
  : `http://127.0.0.1:${port}/state`;

const body = arg === "--celebrate"
  ? "{}"
  : JSON.stringify({ state: arg });

const ctrl = new AbortController();
const timer = setTimeout(() => ctrl.abort(), 500);

try {
  await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body,
    signal: ctrl.signal,
  });
} catch {
  // Clawd not running, network blocked, or timed out — ignore.
} finally {
  clearTimeout(timer);
}

process.exit(0);
