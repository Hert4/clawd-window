use crate::state::{PetState, SharedState};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItemBuilder::with_id("show", "Show").build(app)?;
    let hide = MenuItemBuilder::with_id("hide", "Hide").build(app)?;
    let sleep_item = MenuItemBuilder::with_id("sleep", "Force Sleep").build(app)?;
    let wake = MenuItemBuilder::with_id("wake", "Wake Up").build(app)?;
    let reset = MenuItemBuilder::with_id("reset", "Reset Position").build(app)?;
    let exit = MenuItemBuilder::with_id("exit", "Exit").build(app)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&show, &hide, &sep1, &sleep_item, &wake, &reset, &sep2, &exit])
        .build()?;

    let _tray = TrayIconBuilder::with_id("clawd-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Clawd")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                "show" => {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.show();
                    }
                }
                "hide" => {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.hide();
                    }
                }
                "sleep" => set_pet_state(app, PetState::Sleeping),
                "wake" => set_pet_state(app, PetState::IdleLiving),
                "reset" => {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.set_position(tauri::PhysicalPosition::new(1000i32, 600i32));
                    }
                    set_pet_state(app, PetState::IdleLiving);
                }
                "exit" => app.exit(0),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn set_pet_state(app: &AppHandle, s: PetState) {
    if let Some(state) = app.try_state::<SharedState>() {
        *state.write() = s;
        let payload: crate::state::StatePayload = s.into();
        let _ = app.emit("state-changed", payload);
    }
}
