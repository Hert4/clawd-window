use crate::state::{PetState, SharedState};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::time::sleep;

// Reject paths outside the user's home directory. Eating C:\Windows\... or arbitrary
// system files would be a footgun even via Recycle Bin (Windows blocks some, others
// disrupt the OS). Canonicalize to resolve .. and symlinks before checking.
pub fn validate_eat_path(p: &Path) -> Result<PathBuf, String> {
    let canon = std::fs::canonicalize(p)
        .map_err(|e| format!("canonicalize {:?}: {}", p, e))?;
    let home = dirs::home_dir().ok_or_else(|| "no home dir".to_string())?;
    let home_canon = std::fs::canonicalize(&home).unwrap_or(home);
    if !canon.starts_with(&home_canon) {
        return Err(format!("path outside home: {:?}", canon));
    }
    Ok(canon)
}

pub async fn eat_paths(
    app: AppHandle,
    state: SharedState,
    paths: Vec<PathBuf>,
) -> Result<usize, String> {
    set_state(&app, &state, PetState::Eating);
    sleep(Duration::from_millis(300)).await;

    let mut ok = 0usize;
    let mut had_error = false;
    for p in &paths {
        let validated = match validate_eat_path(p) {
            Ok(v) => v,
            Err(e) => {
                had_error = true;
                log::warn!("eat rejected: {}", e);
                continue;
            }
        };
        match trash::delete(&validated) {
            Ok(()) => ok += 1,
            Err(e) => {
                had_error = true;
                log::warn!("trash::delete failed for {:?}: {}", validated, e);
            }
        }
    }

    sleep(Duration::from_millis(1200)).await;

    if had_error && ok == 0 {
        set_state(&app, &state, PetState::WorkingConfused);
        sleep(Duration::from_millis(1500)).await;
    } else {
        set_state(&app, &state, PetState::Happy);
        sleep(Duration::from_millis(1500)).await;
    }

    set_state(&app, &state, PetState::IdleLiving);
    Ok(ok)
}

fn set_state(app: &AppHandle, state: &SharedState, s: PetState) {
    *state.write() = s;
    let payload: crate::state::StatePayload = s.into();
    if let Err(e) = app.emit("state-changed", payload) {
        log::warn!("emit state-changed failed: {}", e);
    }
}

#[tauri::command]
pub async fn eat_files(
    paths: Vec<String>,
    app: AppHandle,
    state: State<'_, SharedState>,
) -> Result<usize, String> {
    let bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let shared = state.inner().clone();
    eat_paths(app, shared, bufs).await
}
