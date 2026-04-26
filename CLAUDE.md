# Clawd — Agent guide

Quick orientation for future Claude/agents working on this codebase. Read this before making non-trivial changes.

---

## What this is

A Windows desktop pet (Tauri 2 + Rust + WebView2 + vanilla JS) styled after Shimeji. A pixel-art crab named **Clawd** lives on the desktop, walks/climbs windows, and "eats" files dropped on him (-> Recycle Bin). 23 hand-crafted SVG animations.

---

## Don't break these (load-bearing invariants)

### 1. SVG `<object>` swap pattern, NOT inline SVG
The 23 SVGs share class names (`body-anim`, `eyes-look`, `eyes-anim`...) but each defines DIFFERENT `@keyframes` for those classes. Inlining them all into one DOM = CSS rules collide and animations play wrong.

The frontend uses ONE `<object id="clawd" data="...">` element and swaps `obj.data = "assets/clawd-XYZ.svg"` on state change. Each SVG document gets its own CSS scope. Animations also reset to `t=0` automatically when `data` changes.

Don't refactor this to inline SVGs or to pre-load all 23 in hidden divs.

### 2. Position math uses PHYSICAL pixels via `outer_size()`
`tauri.conf.json` declares `width: 200, height: 200` in **logical** pixels. On a 150% DPI monitor, the actual window is 300×300 physical pixels. Hardcoding 200 breaks multi-monitor / HiDPI.

In `pet_controller.rs`, every tick reads:
```rust
let outer_size = win.outer_size().unwrap_or(...);
let walk_w = outer_size.width as i32;   // physical
let walk_h = outer_size.height as i32;
let foot_in_window = (FOOT_RATIO * walk_h as f64) as i32;
```

`FOOT_RATIO = 178/200` is derived from the SVG viewBox (`-15 -25 45 45`, foot at SVG y=15). All gravity / climbing / wall-grab math goes through `walk_w` / `walk_h` / `foot_in_window`. Do NOT reintroduce `WALK_WIN_W` / `WALK_WIN_H` constants in calculations — they exist only as fallbacks for `unwrap_or`.

This was a real bug that took diagnostic instrumentation + a stronger model to find. Don't undo it.

### 3. File eating uses `trash` crate, NOT `std::fs::remove_*`
`trash::delete()` moves to Recycle Bin (recoverable). Permanent delete is a safety regression — users will drop important files on Clawd by accident.

### 4. `windows_tracker.rs` filter chain is empirical
The chain (`!IsIconic`, `!shell`, `!is_cloaked`, `!WS_EX_TOOLWINDOW`, `!WS_EX_NOACTIVATE`, has-title, min-size 200×100, not full-screen-size) was tuned to avoid phantom climbs on UWP/store apps and overlays. Removing any filter has been shown to cause regressions — verify before deleting.

`WS_CAPTION` was tested and removed (Chrome/VS Code don't have classic title bars).

### 5. State authority lives in Rust
`Arc<RwLock<PetState>>` in `state.rs` is the source of truth. JS only listens to `state-changed` events and renders. JS asks for state changes via `set_state_cmd` invoke. Don't add JS-side state mirrors that diverge.

---

## Architecture at a glance

```
Rust backend (state authority)              Frontend (render only)
├── state.rs      PetState enum             pet.js   listens for "state-changed",
├── pet_controller.rs   60 Hz walker tick            swaps obj.data, drag/click
├── window_tracker.rs   100 ms EnumWindows           handlers, eye-tracking
├── file_eater.rs       drag-drop -> trash
├── tray.rs             tray icon + menu
└── lib.rs              app init, plugins
```

Communication: Rust emits `state-changed` events with a `StatePayload` (state key + optional direction/edge). JS listens, swaps SVG. JS calls Rust via `invoke("set_state_cmd", { stateKey })` or `invoke("eat_files", { paths })`.

---

## Key files (don't guess — open these first)

| File | What's there |
|---|---|
| `src-tauri/src/pet_controller.rs` | Walking / climbing / gravity tick. Most active file. |
| `src-tauri/src/state.rs` | `PetState` enum (one variant per SVG). `StatePayload` for JS. |
| `src-tauri/src/window_tracker.rs` | `EnumWindows` polling + filter chain. |
| `src-tauri/src/file_eater.rs` | `eat_files` Tauri command using `trash::delete`. |
| `src-tauri/src/lib.rs` | Plugin registration, command list, single-instance setup, devtools auto-open. |
| `src-tauri/tauri.conf.json` | Window config (transparent, frameless, alwaysOnTop, dragDropEnabled). |
| `src/index.html` | Single `<object>` + `<div id="overlay">` for mouse events. |
| `src/pet.js` | `STATE_TO_SVG` map, drag/click discrimination, eye-tracking. |
| `src/style.css` | `image-rendering: pixelated`, transparent body. |
| `src/assets/` | The 23 SVG animations (canonical location). |
| `assest/` | Original SVG sources — typo in the folder name. **Don't rename**, the user has muscle memory for it. |

---

## Conventions / gotchas

- **`assest/` (typo) is intentional.** Original folder. The "real" assets used by the app are in `src/assets/`. Both directories should stay in sync if you add SVGs.
- **`PetState` enum variants are 1:1 with SVG files.** Adding a new state = (a) new variant in `state.rs`, (b) `STATE_TO_SVG` entry in `pet.js`, (c) optionally include in `working_states()` so auto-cycle picks it.
- **Drag vs click**: 5 px / 250 ms threshold in `pet.js`. Don't lower the px threshold (causes accidental drags) or raise the ms (feels laggy).
- **External move grace period**: when user drags Clawd, the walker detects `actual_pos != last_set_pos` and pauses gravity for 180 ms (`last_external_change.elapsed() < 180ms`). Without this, the pet snaps to floor mid-drag and jitters.
- **Walker tick = 16 ms** (~60 Hz). Window tracker poll = 100 ms (slower because Win32 EnumWindows is expensive).
- **`emit_state` dedups** — only writes + emits if state actually changed. Don't replace with `set_state` (which always emits) inside the walker tick or you'll spam events at 60 Hz.
- **Right-click is suppressed** in `pet.js` (`contextmenu` preventDefault). The tray menu is the only menu.
- **`focus: false`** in `tauri.conf.json` — the pet window must NOT take focus on click, otherwise it steals input from the app the user is working in.

---

## Out of scope — don't add unless explicitly asked

The user has explicitly skipped these in the past:
- AI chatbox / LLM integration
- Bash / shell command execution from the pet
- Permanent file deletion (only Recycle Bin)
- Multiple Clawd instances (single-instance plugin enforces one)
- Drag-drop from internet (only local files)

---

## Common tasks

### Adding a new pet state / animation

1. Drop the SVG in `src/assets/clawd-<name>.svg` (and `assest/` if keeping in sync).
2. Add variant to `PetState` enum in `state.rs`.
3. Add mapping in `pet.js` `STATE_TO_SVG`.
4. If it should appear during idle auto-cycle, add to `working_states()` in `state.rs`.
5. Test with tray menu → Force state, or `invoke("set_state_cmd", { stateKey: "<name>" })` from devtools.

### Regenerating icons after editing the source PNG

Source is `src-tauri/icons/icon-source.png` (1024×1024, character cropped tight to ~95% of canvas — empty padding makes the icon look tiny on shortcuts). To regenerate:

```powershell
cargo tauri icon src-tauri/icons/icon-source.png
```

Or use the PowerShell script pattern in git history (commits around DPI fix) if `tauri-cli` is missing — it generates all PNG sizes + a multi-resolution `.ico` (16/24/32/48/64/128/256) using `System.Drawing` with nearest-neighbor scaling (preserves pixel art).

### Changing window size

If you change `width`/`height` in `tauri.conf.json`, the position math still works (uses `outer_size()`). But check:
- `VISIBLE_BBOX` in `pet.js` — hit-testing rectangle, hardcoded for 200×200 logical
- `FOOT_RATIO` constant assumes `viewBox -15 -25 45 45` and 200-tall window. If you change SVG viewBox or window aspect ratio, recompute.

### Building a release

```powershell
cargo tauri build
```

Outputs in `src-tauri/target/release/bundle/`:
- `msi/clawd_X.Y.Z_x64_en-US.msi` — MSI installer (~3 MB)
- `nsis/clawd_X.Y.Z_x64-setup.exe` — NSIS installer (~2 MB, recommended)

### Running dev

```powershell
cargo tauri dev
```

First build = ~5 min. Subsequent rebuilds = fast (Tauri watches `src-tauri/` and re-runs).

---

## Recent scars worth remembering

- **HiDPI multi-monitor positioning bug**: pet appeared correctly above taskbar on primary (1920×1080, 100% DPI) but landed inside the taskbar on secondary (2256×1504, 150% DPI). Root cause: `WALK_WIN_W=200` was a logical-pixel constant used as if physical. Fix: read `outer_size()` per tick. The diagnostic that found it was logging `pet_y`, `actual_after`, `delta_y`, `visible_foot_y`, `gap_to_taskbar`, `scale_factor`, `outer_size` per monitor at rest.

- **Phantom window climbs** (Clawd attaching to invisible windows): caused by missing `is_cloaked` (DWM) and `WS_EX_NOACTIVATE` filters in `window_tracker.rs`. Both are now in the chain.

- **SVG-reload spam** (1000+ reloads observed): caused by `set_state` (no dedup) being called every walker tick. Fixed by introducing `emit_state` (dedup) and using it inside ticks; reserved `set_state` for user-initiated changes.

- **Walker thread panic** (`attempt to subtract with overflow`): static `AtomicI32::new(i32::MIN)` sentinel + `(prev - new).abs() > N` overflows on first subtraction. Use `(prev as i64 - new as i64).abs()` if you reintroduce a similar gating pattern.

- **Click-through on transparent corners**: the pet window is 200×200 but the visible character occupies only the center ~110×110. Mouse events on transparent corners must pass through to the app below. Currently the JS overlay is `pointer-events: auto` and the `<object>` is `pointer-events: none`, but DOM-level corner pass-through isn't fully implemented (`setIgnoreCursorEvents` toggle is wired but selective hit-testing per-pixel is not).

---

## Repo / git notes

- **Local git** lives at `crab/.git/` (this folder). The user's home dir (`C:/Users/ADMIN/`) also has a stale `.git` from an earlier mistake — when running git commands, ensure cwd is the project root so the local `.git` wins.
- **Remote**: `https://github.com/Hert4/clawd-window.git`, default branch `main`.
- **`.gitignore`** excludes `target/` (5+ GB), `build.log`, IDE folders, `.claude/`. Don't commit them.
- The user prefers Vietnamese for chat. Keep code identifiers and comments in English.

---

## Tone for working with this user

- They like terse, action-oriented answers. State results, don't narrate.
- When stuck or guessing, **call advisor** before committing to an approach. The user has explicitly asked for this when previous guess-then-fix loops wasted their time.
- Diagnose before patching: when behavior doesn't match math, instrument first (log actual vs expected) before changing constants.
