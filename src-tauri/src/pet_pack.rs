use crate::state::{PetState, SharedState, StatePayload};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PackBehavior {
    Walker,
    Idle,
}

#[derive(Clone, Debug, Serialize)]
pub struct PackInfo {
    pub id: String,
    pub name: String,
    pub behavior: PackBehavior,
}

const BUILTIN_PACKS: &[(&str, &str, PackBehavior)] = &[
    ("clawd", "Clawd", PackBehavior::Walker),
    ("hsr_aglaea", "Aglaea (HSR)", PackBehavior::Walker),
    ("hsr_main", "Trailblazer (HSR)", PackBehavior::Walker),
];

const DEFAULT_PACK_ID: &str = "clawd";

fn builtin_packs() -> Vec<PackInfo> {
    BUILTIN_PACKS
        .iter()
        .map(|(id, name, behavior)| PackInfo {
            id: id.to_string(),
            name: name.to_string(),
            behavior: *behavior,
        })
        .collect()
}

fn find_pack(id: &str) -> Option<PackInfo> {
    BUILTIN_PACKS
        .iter()
        .find(|(pack_id, _, _)| *pack_id == id)
        .map(|(id, name, behavior)| PackInfo {
            id: id.to_string(),
            name: name.to_string(),
            behavior: *behavior,
        })
}

fn default_pack() -> PackInfo {
    find_pack(DEFAULT_PACK_ID).expect("default pack must exist in BUILTIN_PACKS")
}

pub type SharedPack = Arc<RwLock<PackInfo>>;

pub fn new_shared_pack() -> SharedPack {
    let id = load_persisted_id();
    let pack = find_pack(&id).unwrap_or_else(default_pack);
    Arc::new(RwLock::new(pack))
}

#[derive(Serialize, Deserialize)]
struct PackConfig {
    current_pack: String,
}

fn config_path() -> Option<PathBuf> {
    let mut p = dirs::config_dir()?;
    p.push("clawd");
    p.push("config.json");
    Some(p)
}

fn load_persisted_id() -> String {
    let path = match config_path() {
        Some(p) => p,
        None => return DEFAULT_PACK_ID.to_string(),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return DEFAULT_PACK_ID.to_string(),
    };
    match serde_json::from_str::<PackConfig>(&content) {
        Ok(cfg) => cfg.current_pack,
        Err(_) => DEFAULT_PACK_ID.to_string(),
    }
}

fn persist_id(id: &str) {
    let path = match config_path() {
        Some(p) => p,
        None => return,
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let cfg = PackConfig {
        current_pack: id.to_string(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&cfg) {
        let _ = std::fs::write(&path, json);
    }
}

#[tauri::command]
pub fn list_packs_cmd() -> Vec<PackInfo> {
    builtin_packs()
}

#[tauri::command]
pub fn get_pack_cmd(pack: tauri::State<'_, SharedPack>) -> PackInfo {
    pack.read().clone()
}

#[tauri::command]
pub fn set_pack_cmd(id: String, app: AppHandle) -> Result<(), String> {
    set_pack_from_id(&app, &id)
}

pub fn set_pack_from_id(app: &AppHandle, id: &str) -> Result<(), String> {
    let new_pack = find_pack(id).ok_or_else(|| format!("unknown pack: {}", id))?;

    if let Some(pack) = app.try_state::<SharedPack>() {
        *pack.write() = new_pack.clone();
    }
    persist_id(id);

    // Reset to idle on pack switch — avoids stale Walking/Climbing state if switching
    // to an idle pack (whose walker thread won't move the pet anymore).
    if let Some(state) = app.try_state::<SharedState>() {
        *state.write() = PetState::IdleLiving;
        let payload: StatePayload = PetState::IdleLiving.into();
        let _ = app.emit("state-changed", payload);
    }

    app.emit("pack-changed", new_pack)
        .map_err(|e| e.to_string())?;
    Ok(())
}
