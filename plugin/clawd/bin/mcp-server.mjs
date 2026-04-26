#!/usr/bin/env node
// MCP server for Clawd, hand-rolled JSON-RPC over stdio (no npm deps).
//
// Wraps Clawd's local HTTP API (default 127.0.0.1:9876) so Claude Code can call
// tools to control the desktop pet.
//
// Protocol notes:
//   - All logging MUST go to stderr; stdout is reserved for JSON-RPC frames.
//   - Frames are line-delimited JSON (one message per line).
//   - On `initialize`, respond with protocolVersion + capabilities.tools.
//   - On `notifications/initialized`, no reply (it's a notification).
//   - `tools/list` returns inputSchema as a JSON Schema object.
//   - `tools/call` returns { content: [{type:"text", text}], isError? }.

import readline from "node:readline";

const PROTOCOL_VERSION = "2025-06-18";
const SERVER_INFO = { name: "clawd", version: "0.1.0" };
const CLAWD_PORT = process.env.CLAWD_PORT || 9876;
const BASE = `http://127.0.0.1:${CLAWD_PORT}`;

const STATE_KEYS = [
  "idle_living", "sleeping", "happy", "dizzy", "dragging", "eating",
  "going_away", "disconnected", "notification",
  "working_typing", "working_thinking", "working_juggling", "working_building",
  "working_carrying", "working_conducting", "working_confused", "working_debugger",
  "working_overheated", "working_pushing", "working_sweeping", "working_wizard",
  "working_beacon",
];

const TOOLS = [
  {
    name: "clawd_set_state",
    description: "Set Clawd's animation state. State must be one of the known keys (use clawd_list_states to see them).",
    inputSchema: {
      type: "object",
      properties: { state: { type: "string", enum: STATE_KEYS } },
      required: ["state"],
    },
  },
  {
    name: "clawd_status",
    description: "Get Clawd's current animation state.",
    inputSchema: { type: "object", properties: {}, required: [] },
  },
  {
    name: "clawd_celebrate",
    description: "Make Clawd celebrate: happy animation for 1.5s, then back to idle.",
    inputSchema: { type: "object", properties: {}, required: [] },
  },
  {
    name: "clawd_eat",
    description: "Move file(s) or folder(s) to the Recycle Bin via Clawd. Paths must be inside the user's home directory; system paths are rejected. Returns count actually eaten.",
    inputSchema: {
      type: "object",
      properties: {
        paths: {
          type: "array",
          items: { type: "string" },
          description: "Absolute filesystem paths under the user's home directory.",
        },
      },
      required: ["paths"],
    },
  },
  {
    name: "clawd_health",
    description: "Check whether the Clawd desktop app is running and reachable.",
    inputSchema: { type: "object", properties: {}, required: [] },
  },
  {
    name: "clawd_list_states",
    description: "List all valid animation state keys for clawd_set_state.",
    inputSchema: { type: "object", properties: {}, required: [] },
  },
];

function send(msg) {
  process.stdout.write(JSON.stringify(msg) + "\n");
}

function sendResult(id, result) {
  send({ jsonrpc: "2.0", id, result });
}

function sendError(id, code, message) {
  send({ jsonrpc: "2.0", id, error: { code, message } });
}

async function http(method, path, body) {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), 5000);
  try {
    const res = await fetch(`${BASE}${path}`, {
      method,
      headers: { "Content-Type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: ctrl.signal,
    });
    const text = await res.text();
    let json;
    try { json = JSON.parse(text); } catch { json = { raw: text }; }
    return { ok: res.ok, status: res.status, data: json };
  } catch (e) {
    return { ok: false, networkError: String(e) };
  } finally {
    clearTimeout(timer);
  }
}

function notRunningError() {
  return {
    code: -32000,
    message: "clawd app not running; start it from the system tray (or run 'cargo tauri dev' / install the .exe).",
  };
}

function textContent(text, isError = false) {
  return { content: [{ type: "text", text }], ...(isError ? { isError: true } : {}) };
}

async function callTool(name, args) {
  switch (name) {
    case "clawd_set_state": {
      const state = args?.state;
      if (typeof state !== "string") return textContent("Missing 'state' argument", true);
      const r = await http("POST", "/state", { state });
      if (r.networkError) return { error: notRunningError() };
      if (!r.ok) return textContent(`Server returned ${r.status}: ${JSON.stringify(r.data)}`, true);
      return textContent(`Clawd state set to ${state}.`);
    }
    case "clawd_status": {
      const r = await http("GET", "/status");
      if (r.networkError) return { error: notRunningError() };
      return textContent(`Clawd: ${JSON.stringify(r.data)}`);
    }
    case "clawd_celebrate": {
      const r = await http("POST", "/celebrate");
      if (r.networkError) return { error: notRunningError() };
      return textContent("Clawd is celebrating. 🎉");
    }
    case "clawd_eat": {
      const paths = args?.paths;
      if (!Array.isArray(paths)) return textContent("Missing 'paths' array", true);
      const r = await http("POST", "/eat", { paths });
      if (r.networkError) return { error: notRunningError() };
      if (!r.ok) return textContent(`Server returned ${r.status}: ${JSON.stringify(r.data)}`, true);
      return textContent(`Clawd ate ${r.data.eaten ?? 0} item(s). They are in the Recycle Bin.`);
    }
    case "clawd_health": {
      const r = await http("GET", "/health");
      if (r.networkError) return textContent(`Clawd not running (${r.networkError})`, true);
      return textContent("Clawd is running.");
    }
    case "clawd_list_states": {
      return textContent(STATE_KEYS.join(", "));
    }
    default:
      return textContent(`Unknown tool: ${name}`, true);
  }
}

async function handle(msg) {
  // Notifications (no id) get no reply.
  if (msg.method === "notifications/initialized") return;
  if (msg.method && !("id" in msg)) return; // any other notification

  const { id, method, params } = msg;

  try {
    if (method === "initialize") {
      sendResult(id, {
        protocolVersion: PROTOCOL_VERSION,
        capabilities: { tools: {} },
        serverInfo: SERVER_INFO,
      });
      return;
    }
    if (method === "tools/list") {
      sendResult(id, { tools: TOOLS });
      return;
    }
    if (method === "tools/call") {
      const { name, arguments: args } = params || {};
      const result = await callTool(name, args || {});
      if (result?.error) {
        sendError(id, result.error.code, result.error.message);
      } else {
        sendResult(id, result);
      }
      return;
    }
    if (method === "ping") {
      sendResult(id, {});
      return;
    }
    sendError(id, -32601, `Method not found: ${method}`);
  } catch (e) {
    console.error("[clawd-mcp] handler error:", e);
    sendError(id, -32603, `Internal error: ${e?.message || e}`);
  }
}

const rl = readline.createInterface({ input: process.stdin });
rl.on("line", async (line) => {
  if (!line.trim()) return;
  let msg;
  try {
    msg = JSON.parse(line);
  } catch (e) {
    console.error("[clawd-mcp] bad JSON:", line);
    return;
  }
  await handle(msg);
});
rl.on("close", () => process.exit(0));

console.error(`[clawd-mcp] ready, base=${BASE}`);
