// Clawd desktop pet
mod file_eater;
mod pet_controller;
mod state;
mod tray;
mod window_tracker;

use crate::state::{new_shared_state, PetState, SharedState, StatePayload};
use crate::window_tracker::{new_shared_windows, SharedWindows};
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
fn set_state_cmd(state_key: String, app: AppHandle, state: tauri::State<'_, SharedState>) -> Result<(), String> {
    let next = match state_key.as_str() {
        "idle_living" => PetState::IdleLiving,
        "sleeping" => PetState::Sleeping,
        "happy" => PetState::Happy,
        "dizzy" => PetState::Dizzy,
        "dragging" => PetState::Dragging,
        "eating" => PetState::Eating,
        "going_away" => PetState::GoingAway,
        "disconnected" => PetState::Disconnected,
        "notification" => PetState::Notification,
        "working_typing" => PetState::WorkingTyping,
        "working_thinking" => PetState::WorkingThinking,
        "working_juggling" => PetState::WorkingJuggling,
        "working_building" => PetState::WorkingBuilding,
        "working_carrying" => PetState::WorkingCarrying,
        "working_conducting" => PetState::WorkingConducting,
        "working_confused" => PetState::WorkingConfused,
        "working_debugger" => PetState::WorkingDebugger,
        "working_overheated" => PetState::WorkingOverheated,
        "working_pushing" => PetState::WorkingPushing,
        "working_sweeping" => PetState::WorkingSweeping,
        "working_wizard" => PetState::WorkingWizard,
        "working_beacon" => PetState::WorkingBeacon,
        _ => return Err(format!("unknown state: {}", state_key)),
    };
    *state.write() = next;
    let payload: StatePayload = next.into();
    app.emit("state-changed", payload).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_state_cmd(state: tauri::State<'_, SharedState>) -> StatePayload {
    let s = *state.read();
    s.into()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shared = new_shared_state();
    let windows = new_shared_windows();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .manage(shared)
        .manage(windows)
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            tray::setup_tray(&app.handle())?;

            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_skip_taskbar(true);
                #[cfg(debug_assertions)]
                {
                    w.open_devtools();
                }
            }

            let shared = app.state::<SharedState>().inner().clone();
            let windows = app.state::<SharedWindows>().inner().clone();

            #[cfg(windows)]
            let self_hwnd: isize = app
                .get_webview_window("main")
                .and_then(|w| w.hwnd().ok().map(|h| h.0 as isize))
                .unwrap_or(0);
            #[cfg(not(windows))]
            let self_hwnd: isize = 0;

            window_tracker::spawn_window_poller(windows.clone(), self_hwnd);
            pet_controller::spawn_auto_cycle(app.handle().clone(), shared.clone());
            pet_controller::spawn_walker(app.handle().clone(), shared, windows);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_state_cmd,
            get_state_cmd,
            file_eater::eat_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
