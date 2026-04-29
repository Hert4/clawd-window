// Clawd desktop pet
mod file_eater;
mod http_api;
mod mood;
mod pet_controller;
mod pet_pack;
mod state;
mod tray;
mod window_tracker;

use crate::mood::{new_shared_mood, SharedMood};
use crate::pet_pack::{new_shared_pack, SharedPack};
use crate::state::{new_external_until, new_shared_state, ExternalUntil, PetState, SharedState, StatePayload};
use crate::window_tracker::{new_shared_windows, SharedWindows};
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
fn set_state_cmd(state_key: String, app: AppHandle, state: tauri::State<'_, SharedState>) -> Result<(), String> {
    let next = PetState::from_key(&state_key)
        .ok_or_else(|| format!("unknown state: {}", state_key))?;
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
    let external_until = new_external_until();
    let pack = new_shared_pack();
    let mood = new_shared_mood();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .manage(shared)
        .manage(windows)
        .manage(external_until)
        .manage(pack)
        .manage(mood)
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
            let external_until = app.state::<ExternalUntil>().inner().clone();
            let pack = app.state::<SharedPack>().inner().clone();
            let mood = app.state::<SharedMood>().inner().clone();

            #[cfg(windows)]
            let self_hwnd: isize = app
                .get_webview_window("main")
                .and_then(|w| w.hwnd().ok().map(|h| h.0 as isize))
                .unwrap_or(0);
            #[cfg(not(windows))]
            let self_hwnd: isize = 0;

            window_tracker::spawn_window_poller(windows.clone(), self_hwnd);
            mood::spawn_mood_thread(mood.clone());
            pet_controller::spawn_auto_cycle(app.handle().clone(), shared.clone(), external_until.clone(), pack.clone(), mood);
            pet_controller::spawn_walker(app.handle().clone(), shared.clone(), windows, pack);
            http_api::spawn_http_server(app.handle().clone(), shared, external_until);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_state_cmd,
            get_state_cmd,
            file_eater::eat_files,
            pet_pack::list_packs_cmd,
            pet_pack::get_pack_cmd,
            pet_pack::set_pack_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
