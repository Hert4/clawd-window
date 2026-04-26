use crate::state::{working_states, Direction, Edge, ExternalUntil, PetState, SharedState, StatePayload};
use crate::window_tracker::SharedWindows;
use rand::seq::SliceRandom;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use tokio::time::{interval, sleep, MissedTickBehavior};

#[cfg(windows)]
use windows::Win32::Foundation::POINT;
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};

#[cfg(windows)]
fn work_area_for_point(x: i32, y: i32) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        let pt = POINT { x, y };
        let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        if monitor.is_invalid() {
            return None;
        }
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            let left = info.rcWork.left;
            let top = info.rcWork.top;
            let right = info.rcWork.right;
            let mut bottom = info.rcWork.bottom;
            const ASSUMED_TASKBAR_PX: i32 = 48;
            if (info.rcMonitor.bottom - bottom) < 30 {
                bottom = info.rcMonitor.bottom - ASSUMED_TASKBAR_PX;
            }
            Some((left, top, right, bottom))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
fn work_area_for_point(_x: i32, _y: i32) -> Option<(i32, i32, i32, i32)> {
    None
}

const WALK_TICK_MS: u64 = 16;
const WALK_SPEED_PX_PER_TICK: i32 = 2;
const CLIMB_SPEED_PX_PER_TICK: i32 = 2;
const WALK_WIN_W: i32 = 200;
const WALK_WIN_H: i32 = 200;
const FOOT_RATIO: f64 = 178.0 / 200.0;
const FLOOR_GAP_PX: i32 = 10;
const FALL_SPEED_PX_PER_TICK: i32 = 4;
const WALL_GRAB_DIST: i32 = 12;

pub fn spawn_auto_cycle(app: AppHandle, state: SharedState, external_until: ExternalUntil) {
    tauri::async_runtime::spawn(async move {
        let mut rng = SmallRng::from_entropy();
        loop {
            let wait = rng.gen_range(25..55);
            sleep(Duration::from_secs(wait)).await;

            // Skip auto-cycle entirely while an external client (HTTP/MCP) is driving the pet.
            if Instant::now() < *external_until.lock() {
                continue;
            }

            let current = *state.read();
            if !matches!(current, PetState::IdleLiving) {
                continue;
            }

            let pick: u32 = rng.gen_range(0..10);
            let next: PetState = if pick < 4 {
                let dir = if rng.gen_bool(0.5) { Direction::Left } else { Direction::Right };
                PetState::Walking { dir }
            } else {
                let pool = working_states();
                match pool.choose(&mut rng) {
                    Some(s) => *s,
                    None => continue,
                }
            };

            set_state(&app, &state, next);

            let hold = if matches!(next, PetState::Walking { .. }) { rng.gen_range(6..12) } else { rng.gen_range(8..16) };
            sleep(Duration::from_secs(hold)).await;

            // Don't reset back to idle if an external client took over during the hold.
            if Instant::now() < *external_until.lock() {
                continue;
            }
            let still = *state.read();
            if still == next {
                set_state(&app, &state, PetState::IdleLiving);
            }
        }
    });
}

pub fn spawn_walker(app: AppHandle, state: SharedState, windows: SharedWindows) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = interval(Duration::from_millis(WALK_TICK_MS));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let mut pet_x: i32 = 0;
        let mut pet_y: i32 = 0;
        let mut last_set_x: i32 = 0;
        let mut last_set_y: i32 = 0;
        let mut last_external_change = Instant::now() - Duration::from_secs(10);
        let _climb_cooldown_until = Instant::now() - Duration::from_secs(10);
        let mut corner_turn_remaining: i32 = 0;
        let mut corner_turn_target_x: i32 = 0;
        let mut initialized = false;

        loop {
            ticker.tick().await;

            let cur = *state.read();
            if matches!(cur, PetState::Eating) {
                initialized = false;
                continue;
            }

            let win = match app.get_webview_window("main") {
                Some(w) => w,
                None => continue,
            };

            let actual = match win.outer_position() {
                Ok(p) => p,
                Err(_) => continue,
            };

            // tauri.conf width/height are LOGICAL pixels; on HiDPI monitors the actual
            // physical window is larger (1.5x at 150% scale). Use outer_size() so math
            // stays correct when pet crosses between monitors with different DPI.
            let outer_size = win
                .outer_size()
                .unwrap_or(tauri::PhysicalSize::new(WALK_WIN_W as u32, WALK_WIN_H as u32));
            let walk_w = outer_size.width as i32;
            let walk_h = outer_size.height as i32;
            let foot_in_window = (FOOT_RATIO * walk_h as f64) as i32;

            let probe_x = if initialized { pet_x + walk_w / 2 } else { actual.x + walk_w / 2 };
            let probe_y = if initialized { pet_y + walk_h / 2 } else { actual.y + walk_h / 2 };
            let (work_left, work_top, work_right, work_bottom) =
                match work_area_for_point(probe_x, probe_y) {
                    Some(r) => r,
                    None => continue,
                };
            let m_pos = tauri::PhysicalPosition::new(work_left, work_top);
            let m_size = tauri::PhysicalSize::new(
                (work_right - work_left) as u32,
                (work_bottom - work_top) as u32,
            );

            if !initialized {
                pet_x = actual.x;
                pet_y = actual.y;
                last_set_x = actual.x;
                last_set_y = actual.y;
                initialized = true;
            }

            let dx = (actual.x - last_set_x).abs();
            let dy = (actual.y - last_set_y).abs();
            let externally_moved = dx > 3 || dy > 3;

            if externally_moved {
                pet_x = actual.x;
                pet_y = actual.y;
                last_external_change = Instant::now();
                corner_turn_remaining = 0;
            }

            if last_external_change.elapsed() < Duration::from_millis(180) {
                last_set_x = actual.x;
                last_set_y = actual.y;
                continue;
            }

            if corner_turn_remaining > 0 {
                let delta = corner_turn_target_x - pet_x;
                let step = delta / corner_turn_remaining;
                pet_x += step;
                corner_turn_remaining -= 1;
                if corner_turn_remaining == 0 {
                    pet_x = corner_turn_target_x;
                }
                if pet_x != actual.x || pet_y != actual.y {
                    let _ = win.set_position(PhysicalPosition::new(pet_x, pet_y));
                }
                last_set_x = pet_x;
                last_set_y = pet_y;
                continue;
            }

            let mut apply_gravity = true;

            match cur {
                PetState::Walking { dir } => {
                    let step = match dir {
                        Direction::Left => -WALK_SPEED_PX_PER_TICK,
                        Direction::Right => WALK_SPEED_PX_PER_TICK,
                    };
                    pet_x += step;
                    let min_x = m_pos.x - walk_w * 75 / 200;
                    let max_x = m_pos.x + m_size.width as i32 - walk_w * 125 / 200;
                    if pet_x <= min_x {
                        pet_x = min_x;
                        emit_state(
                            &app,
                            &state,
                            PetState::Climbing {
                                hwnd: 0,
                                edge: Edge::Left,
                                offset: 0,
                            },
                        );
                        apply_gravity = false;
                    } else if pet_x >= max_x {
                        pet_x = max_x;
                        emit_state(
                            &app,
                            &state,
                            PetState::Climbing {
                                hwnd: 0,
                                edge: Edge::Right,
                                offset: 0,
                            },
                        );
                        apply_gravity = false;
                    }

                    let pet_top = pet_y;
                    let pet_bottom = pet_y + walk_h;
                    let pet_left = pet_x;
                    let pet_right = pet_x + walk_w;

                    let wall_grab = WALL_GRAB_DIST * walk_w / 200;
                    for w in windows.read().iter() {
                        let vertical_overlap =
                            pet_bottom > w.top + 30 && pet_top + 20 < w.bottom;
                        if !vertical_overlap {
                            continue;
                        }
                        match dir {
                            Direction::Right => {
                                if pet_right >= w.left
                                    && pet_right <= w.left + wall_grab
                                {
                                    pet_x = w.left - walk_w / 2;
                                    emit_state(
                                        &app,
                                        &state,
                                        PetState::Climbing {
                                            hwnd: w.hwnd,
                                            edge: Edge::Left,
                                            offset: 0,
                                        },
                                    );
                                    apply_gravity = false;
                                    break;
                                }
                            }
                            Direction::Left => {
                                if pet_left <= w.right
                                    && pet_left >= w.right - wall_grab
                                {
                                    pet_x = w.right - walk_w / 2;
                                    emit_state(
                                        &app,
                                        &state,
                                        PetState::Climbing {
                                            hwnd: w.hwnd,
                                            edge: Edge::Right,
                                            offset: 0,
                                        },
                                    );
                                    apply_gravity = false;
                                    break;
                                }
                            }
                        }
                    }
                }
                PetState::Climbing { hwnd, edge, .. } => {
                    apply_gravity = false;
                    if hwnd == 0 {
                        pet_y -= CLIMB_SPEED_PX_PER_TICK;
                        match edge {
                            Edge::Left => pet_x = m_pos.x,
                            Edge::Right => {
                                pet_x = m_pos.x + m_size.width as i32 - walk_w
                            }
                            _ => {}
                        }
                        let stop_y =
                            m_pos.y + (m_size.height as i32) * 2 / 10;
                        if pet_y <= stop_y {
                            let new_dir = match edge {
                                Edge::Left => Direction::Right,
                                Edge::Right => Direction::Left,
                                _ => Direction::Right,
                            };
                            emit_state(
                                &app,
                                &state,
                                PetState::Walking { dir: new_dir },
                            );
                        }
                        if pet_x != actual.x || pet_y != actual.y {
                            let _ = win.set_position(PhysicalPosition::new(pet_x, pet_y));
                        }
                        last_set_x = pet_x;
                        last_set_y = pet_y;
                        continue;
                    }
                    let win_rect = windows.read().iter().find(|w| w.hwnd == hwnd).copied();
                    match win_rect {
                        Some(w) => {
                            pet_y -= CLIMB_SPEED_PX_PER_TICK;
                            match edge {
                                Edge::Left => pet_x = w.left - walk_w / 2,
                                Edge::Right => pet_x = w.right - walk_w / 2,
                                Edge::Top => {}
                            }
                            let target_top_y = w.top - foot_in_window - FLOOR_GAP_PX;
                            if pet_y <= target_top_y {
                                pet_y = target_top_y;
                                let new_dir = match edge {
                                    Edge::Right => Direction::Left,
                                    _ => Direction::Right,
                                };
                                corner_turn_target_x = match edge {
                                    Edge::Left => w.left,
                                    Edge::Right => w.right - walk_w,
                                    _ => pet_x,
                                };
                                corner_turn_remaining = 14;
                                emit_state(&app, &state, PetState::Walking { dir: new_dir });
                            }
                        }
                        None => {
                            emit_state(
                                &app,
                                &state,
                                PetState::Walking { dir: Direction::Right },
                            );
                        }
                    }
                }
                _ => {}
            }

            if apply_gravity {
                let pet_center_x = pet_x + walk_w / 2;
                let screen_floor_y =
                    m_pos.y + m_size.height as i32 - foot_in_window - FLOOR_GAP_PX;

                let mut target_floor = screen_floor_y;
                for w in windows.read().iter() {
                    let pad = 30;
                    if pet_center_x >= w.left + pad && pet_center_x <= w.right - pad {
                        let candidate = w.top - foot_in_window - FLOOR_GAP_PX;
                        if candidate < target_floor && pet_y <= candidate + 8 {
                            target_floor = candidate;
                        }
                    }
                }

                if target_floor < pet_y {
                    pet_y = target_floor;
                } else if target_floor > pet_y {
                    pet_y = (pet_y + FALL_SPEED_PX_PER_TICK).min(target_floor);
                }
            }

            if pet_x != actual.x || pet_y != actual.y {
                let _ = win.set_position(PhysicalPosition::new(pet_x, pet_y));
            }
            last_set_x = pet_x;
            last_set_y = pet_y;
        }
    });
}

fn emit_state(app: &AppHandle, state: &SharedState, s: PetState) {
    {
        let mut w = state.write();
        if *w == s {
            return;
        }
        *w = s;
    }
    let payload: StatePayload = s.into();
    let _ = app.emit("state-changed", payload);
}

fn set_state(app: &AppHandle, state: &SharedState, s: PetState) {
    *state.write() = s;
    let payload: StatePayload = s.into();
    if let Err(e) = app.emit("state-changed", payload) {
        log::warn!("emit state-changed failed: {}", e);
    }
}
