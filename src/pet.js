"use strict";

const EYE_SELECTORS = [".eyes-look", ".eyes-anim", ".eyes-blink"];
const PIXEL_CSS = `svg { image-rendering: pixelated; shape-rendering: crispEdges; }`;

const VISIBLE_BBOX = { x: 50, y: 60, w: 100, h: 110 };
const DRAG_THRESHOLD_PX2 = 25;
const DRAG_TIMEOUT_MS = 250;
const EYE_TRACK_RADIUS = 280;
const EYE_TRACK_MAX = 6;

const objSvg = document.getElementById("clawd");
const objImg = document.getElementById("clawd-img");
const objVid = document.getElementById("clawd-vid");
const overlay = document.getElementById("overlay");

let currentState = "idle_living";
let currentDirection = null;
let currentEdge = null;
let currentEyesEl = null;
let preHappyState = null;
let currentManifest = null;

// Cycler state: shuffled-no-repeat rotation through manifest.pool for idle packs.
// Only swaps the displayed asset while pet is in idle_living — state changes
// (click → happy, drop → eating, ...) interrupt and override.
const CYCLER_INTERVAL_MS = 5000;
let cyclerActive = false;
let cyclerPool = [];
let cyclerOrder = [];
let cyclerCursor = 0;
let cyclerTimer = null;

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
  for (const el of [objSvg, objImg, objVid]) {
    el.classList.remove("flip", "climb-left", "climb-right", "climb-top");
    if (direction === "left") el.classList.add("flip");
  }
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

objSvg.addEventListener("load", () => {
  const doc = objSvg.contentDocument;
  try { injectPixelCss(doc); } catch (e) { console.warn("[Clawd] injectPixelCss:", e); }
  try { currentEyesEl = findEyesEl(doc); } catch (e) { console.warn("[Clawd] findEyesEl:", e); currentEyesEl = null; }
  console.log("[Clawd] svg loaded, eyes el:", currentEyesEl ? currentEyesEl.tagName + "." + currentEyesEl.getAttribute("class") : "NOT FOUND");
});

function resolveAssetPath(state) {
  if (!currentManifest || !currentManifest.states) return null;
  return currentManifest.states[state] || currentManifest.states.idle_living || null;
}

function showAssetByPath(path) {
  if (!path) return;
  const ext = path.split(".").pop().toLowerCase();
  if (ext === "svg") showSvg(path);
  else if (ext === "webm") showVid(path);
  else showImg(path);
}

function shuffleArray(arr) {
  for (let i = arr.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [arr[i], arr[j]] = [arr[j], arr[i]];
  }
}

function pickNextCyclerSticker() {
  if (cyclerPool.length === 0) return null;
  if (cyclerCursor >= cyclerOrder.length) {
    cyclerOrder = Array.from({ length: cyclerPool.length }, (_, i) => i);
    shuffleArray(cyclerOrder);
    cyclerCursor = 0;
  }
  return cyclerPool[cyclerOrder[cyclerCursor++]];
}

function tickCycler() {
  if (!cyclerActive) return;
  // Only swap when pet is in idle_living — non-idle states control the asset themselves.
  if (currentState !== "idle_living") return;
  const path = pickNextCyclerSticker();
  if (path) showAssetByPath(path);
}

function startCycler(pool) {
  stopCycler();
  if (!Array.isArray(pool) || pool.length === 0) return;
  cyclerPool = pool.slice();
  cyclerOrder = Array.from({ length: cyclerPool.length }, (_, i) => i);
  shuffleArray(cyclerOrder);
  cyclerCursor = 0;
  cyclerActive = true;
  console.log("[Clawd] cycler started, pool size:", cyclerPool.length);
  tickCycler();
  cyclerTimer = setInterval(tickCycler, CYCLER_INTERVAL_MS);
}

function stopCycler() {
  if (cyclerTimer) clearInterval(cyclerTimer);
  cyclerTimer = null;
  cyclerActive = false;
  cyclerPool = [];
  cyclerOrder = [];
  cyclerCursor = 0;
}

function showSvg(path) {
  const cur = objSvg.getAttribute("data") || "";
  if (cur !== path) objSvg.data = path;
  objSvg.hidden = false;
  objImg.hidden = true;
  if (!objVid.paused) objVid.pause();
  objVid.hidden = true;
}

function showImg(path) {
  if (objImg.getAttribute("src") !== path) objImg.src = path;
  objImg.hidden = false;
  objSvg.hidden = true;
  if (!objVid.paused) objVid.pause();
  objVid.hidden = true;
  currentEyesEl = null;
}

function showVid(path) {
  if (objVid.getAttribute("src") !== path) objVid.src = path;
  objVid.hidden = false;
  objSvg.hidden = true;
  objImg.hidden = true;
  objVid.play().catch(() => {});
  currentEyesEl = null;
}

function applyState(payload) {
  const state = payload.state || payload;
  const direction = payload.direction || null;
  const edge = payload.edge || null;
  currentState = state;
  currentDirection = direction;
  currentEdge = edge;
  applyDirectionEdge(direction, edge);

  // Cycler controls the asset while in idle_living. Snap to a fresh sticker
  // immediately on entering idle (so we don't get stuck on the previous state's asset).
  if (cyclerActive && state === "idle_living") {
    const path = pickNextCyclerSticker();
    if (path) showAssetByPath(path);
    return;
  }

  const path = resolveAssetPath(state);
  if (!path) {
    console.warn("[Clawd] no asset for state:", state, "manifest:", currentManifest && currentManifest.id);
    return;
  }
  showAssetByPath(path);
}

async function loadPack(packId) {
  try {
    const idxRes = await fetch("packs/index.json");
    const idx = await idxRes.json();
    const entry = (idx.packs || []).find((p) => p.id === packId);
    if (!entry) {
      console.warn("[Clawd] pack not in index.json:", packId);
      return;
    }
    const manRes = await fetch(entry.manifest);
    currentManifest = await manRes.json();
    console.log("[Clawd] loaded pack:", currentManifest.id, "behavior:", currentManifest.behavior, "states:", Object.keys(currentManifest.states || {}).length, "pool:", (currentManifest.pool || []).length);

    const scale = typeof currentManifest.scale === "number" ? currentManifest.scale : 1;
    document.documentElement.style.setProperty("--pet-scale", scale);

    // Cycler runs whenever pack provides a pool — independent of walker/idle behavior.
    // It rotates stickers during idle_living; non-idle states (walking, eating, ...) take over the asset.
    if (Array.isArray(currentManifest.pool) && currentManifest.pool.length > 0) {
      startCycler(currentManifest.pool);
    } else {
      stopCycler();
    }

    applyState({ state: currentState, direction: currentDirection, edge: currentEdge });
  } catch (err) {
    console.error("[Clawd] loadPack failed:", err);
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
  await listen("pack-changed", (event) => {
    const packId = event.payload && event.payload.id;
    if (packId) loadPack(packId);
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
    const pack = await invoke("get_pack_cmd");
    if (pack && pack.id) await loadPack(pack.id);
  } catch (err) {
    console.warn("get_pack_cmd failed:", err);
  }
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
