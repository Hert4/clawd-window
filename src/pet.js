"use strict";

const STATE_TO_SVG = {
  idle_living: "assets/clawd-idle-living.svg",
  walking: "assets/clawd-crab-walking.svg",
  climbing: "assets/clawd-crab-walking.svg",
  sleeping: "assets/clawd-sleeping.svg",
  happy: "assets/clawd-happy.svg",
  dizzy: "assets/clawd-dizzy.svg",
  dragging: "assets/clawd-dizzy.svg",
  eating: "assets/clawd-working-carrying.svg",
  going_away: "assets/clawd-going-away.svg",
  disconnected: "assets/clawd-disconnected.svg",
  notification: "assets/clawd-notification.svg",
  working_typing: "assets/clawd-working-typing.svg",
  working_thinking: "assets/clawd-working-thinking.svg",
  working_juggling: "assets/clawd-working-juggling.svg",
  working_building: "assets/clawd-working-building.svg",
  working_carrying: "assets/clawd-working-carrying.svg",
  working_conducting: "assets/clawd-working-conducting.svg",
  working_confused: "assets/clawd-working-confused.svg",
  working_debugger: "assets/clawd-working-debugger.svg",
  working_overheated: "assets/clawd-working-overheated.svg",
  working_pushing: "assets/clawd-working-pushing.svg",
  working_sweeping: "assets/clawd-working-sweeping.svg",
  working_wizard: "assets/clawd-working-wizard.svg",
  working_beacon: "assets/clawd-working-beacon.svg",
};

const EYE_SELECTORS = [".eyes-look", ".eyes-anim", ".eyes-blink"];
const PIXEL_CSS = `svg { image-rendering: pixelated; shape-rendering: crispEdges; }`;

const VISIBLE_BBOX = { x: 50, y: 60, w: 100, h: 110 };
const DRAG_THRESHOLD_PX2 = 25;
const DRAG_TIMEOUT_MS = 250;
const EYE_TRACK_RADIUS = 280;
const EYE_TRACK_MAX = 6;

const obj = document.getElementById("clawd");
const overlay = document.getElementById("overlay");

let currentState = "idle_living";
let currentDirection = null;
let currentEdge = null;
let currentEyesEl = null;
let preHappyState = null;

const tauri = window.__TAURI__ || null;
console.log("[Clawd] __TAURI__ keys:", tauri ? Object.keys(tauri) : "null");
const invoke = tauri && tauri.core ? tauri.core.invoke : async () => {};
const listen = tauri && tauri.event ? tauri.event.listen : async () => () => {};

function getWindow() {
  if (!tauri) return null;
  if (tauri.webviewWindow && tauri.webviewWindow.getCurrentWebviewWindow) {
    return tauri.webviewWindow.getCurrentWebviewWindow();
  }
  if (tauri.window && tauri.window.getCurrentWindow) {
    return tauri.window.getCurrentWindow();
  }
  if (tauri.window && tauri.window.getCurrent) {
    return tauri.window.getCurrent();
  }
  return null;
}

async function startDragSafe() {
  const w = getWindow();
  if (w && typeof w.startDragging === "function") {
    try { await w.startDragging(); return true; } catch (err) { console.warn("[Clawd] window.startDragging:", err); }
  }
  try { await invoke("plugin:window|start_dragging"); return true; } catch (err) { console.warn("[Clawd] plugin:window|start_dragging:", err); }
  return false;
}

async function setIgnoreCursorEventsSafe(ignore) {
  const w = getWindow();
  if (w && typeof w.setIgnoreCursorEvents === "function") {
    try { await w.setIgnoreCursorEvents(ignore); return; } catch (err) { console.warn("[Clawd] setIgnoreCursorEvents:", err); }
  }
  try { await invoke("plugin:window|set_ignore_cursor_events", { ignore }); } catch (err) { console.warn("[Clawd] invoke set_ignore_cursor_events:", err); }
}

function applyDirectionEdge(direction, edge) {
  obj.classList.remove("flip", "climb-left", "climb-right", "climb-top");
  if (direction === "left") obj.classList.add("flip");
}

function injectPixelCss(doc) {
  if (!doc) return;
  const root = doc.documentElement;
  if (!root) return;
  if (root.querySelector("style[data-pixel]")) return;
  const ns = "http://www.w3.org/2000/svg";
  const s = doc.createElementNS(ns, "style");
  s.setAttribute("data-pixel", "1");
  s.textContent = PIXEL_CSS;
  root.appendChild(s);
}

function findEyesEl(doc) {
  if (!doc) return null;
  for (const sel of EYE_SELECTORS) {
    const el = doc.querySelector(sel);
    if (el) return el;
  }
  return null;
}

obj.addEventListener("load", () => {
  const doc = obj.contentDocument;
  try { injectPixelCss(doc); } catch (e) { console.warn("[Clawd] injectPixelCss:", e); }
  try { currentEyesEl = findEyesEl(doc); } catch (e) { console.warn("[Clawd] findEyesEl:", e); currentEyesEl = null; }
  console.log("[Clawd] svg loaded, eyes el:", currentEyesEl ? currentEyesEl.tagName + "." + currentEyesEl.getAttribute("class") : "NOT FOUND");
});

function applyState(payload) {
  const state = payload.state || payload;
  const direction = payload.direction || null;
  const edge = payload.edge || null;
  const path = STATE_TO_SVG[state];
  if (!path) {
    console.warn("Unknown state:", state);
    return;
  }
  currentState = state;
  currentDirection = direction;
  currentEdge = edge;
  applyDirectionEdge(direction, edge);
  const currentPath = obj.getAttribute("data") || "";
  if (currentPath !== path) {
    obj.data = path;
  }
}

window.applyState = applyState;

let mouseDownPos = null;
let mouseDownTime = 0;
let isDragging = false;

overlay.addEventListener("mousedown", (e) => {
  if (e.button !== 0) return;
  mouseDownPos = { x: e.clientX, y: e.clientY };
  mouseDownTime = Date.now();
  isDragging = false;
});

overlay.addEventListener("mousemove", (e) => {
  if (!mouseDownPos) {
    handleHover(e);
    return;
  }
  const dx = e.clientX - mouseDownPos.x;
  const dy = e.clientY - mouseDownPos.y;
  const elapsed = Date.now() - mouseDownTime;
  if (!isDragging && dx * dx + dy * dy > DRAG_THRESHOLD_PX2) {
    isDragging = true;
    startDragSafe().then((ok) => console.log("[Clawd] drag started:", ok));
  }
});

overlay.addEventListener("mouseup", (e) => {
  if (e.button !== 0) {
    mouseDownPos = null;
    return;
  }
  if (mouseDownPos && !isDragging) {
    onClick();
  }
  mouseDownPos = null;
  isDragging = false;
});

window.addEventListener("blur", () => {
  if (isDragging) {
    isDragging = false;
    mouseDownPos = null;
  }
});

overlay.addEventListener("mouseleave", () => {
  mouseInBbox(false);
});

function onClick() {
  preHappyState = currentState;
  invoke("set_state_cmd", { stateKey: "happy" }).catch(() => {});
  setTimeout(() => {
    invoke("set_state_cmd", { stateKey: "idle_living" }).catch(() => {});
  }, 1500);
}

document.addEventListener("contextmenu", (e) => {
  e.preventDefault();
});

document.addEventListener("dragstart", (e) => e.preventDefault());

let inBbox = false;
function mouseInBbox(b) {
  if (b === inBbox) return;
  inBbox = b;
}

function handleHover(e) {
  const wx = e.clientX;
  const wy = e.clientY;
  const inside =
    wx >= VISIBLE_BBOX.x && wx <= VISIBLE_BBOX.x + VISIBLE_BBOX.w &&
    wy >= VISIBLE_BBOX.y && wy <= VISIBLE_BBOX.y + VISIBLE_BBOX.h;
  mouseInBbox(inside);

  if (currentEyesEl) {
    const cx = VISIBLE_BBOX.x + VISIBLE_BBOX.w / 2;
    const cy = VISIBLE_BBOX.y + VISIBLE_BBOX.h / 2;
    const dx = wx - cx;
    const dy = wy - cy;
    const dist = Math.hypot(dx, dy);
    if (dist < EYE_TRACK_RADIUS) {
      const f = Math.min(1, dist / EYE_TRACK_RADIUS) * EYE_TRACK_MAX;
      const ux = (dx / Math.max(dist, 1)) * f;
      const uy = (dy / Math.max(dist, 1)) * f;
      currentEyesEl.style.animation = "none";
      currentEyesEl.style.transform = `translate(${ux}px, ${uy}px)`;
    } else {
      currentEyesEl.style.animation = "";
      currentEyesEl.style.transform = "";
    }
  }
}

async function init() {
  if (!tauri) {
    console.warn("Tauri global not available — running in plain browser");
    return;
  }
  await listen("state-changed", (event) => {
    applyState(event.payload);
  });
  await listen("cursor-pos", (event) => {
  });
  await listen("tauri://drag-enter", () => {
    invoke("set_state_cmd", { stateKey: "eating" }).catch(() => {});
  });
  await listen("tauri://drag-leave", () => {
    invoke("set_state_cmd", { stateKey: "idle_living" }).catch(() => {});
  });
  await listen("tauri://drag-drop", async (event) => {
    const paths = event.payload && event.payload.paths;
    if (!paths || !paths.length) return;
    try {
      await invoke("eat_files", { paths });
    } catch (err) {
      console.error("eat_files:", err);
    }
  });
  try {
    const cur = await invoke("get_state_cmd");
    if (cur) applyState(cur);
  } catch (err) {
    console.warn("get_state_cmd failed:", err);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  init().catch((err) => console.error("init error:", err));
});
