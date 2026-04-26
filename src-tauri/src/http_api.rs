use crate::file_eater::eat_paths;
use crate::state::{ExternalUntil, PetState, SharedState, StatePayload};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

const EXTERNAL_HOLD_SECS: u64 = 30;
const DEBOUNCE_MS: u64 = 250;

#[derive(Clone)]
struct AppState {
    app: AppHandle,
    state: SharedState,
    external_until: ExternalUntil,
    last_external_set: Arc<Mutex<(PetState, Instant)>>,
}

#[derive(Deserialize)]
struct StateReq {
    state: String,
}

#[derive(Deserialize)]
struct EatReq {
    paths: Vec<String>,
}

#[derive(Serialize)]
struct EatResp {
    eaten: usize,
}

#[derive(Serialize)]
struct OkResp {
    ok: bool,
}

#[derive(Serialize)]
struct ErrResp {
    error: String,
}

async fn health() -> Json<OkResp> {
    Json(OkResp { ok: true })
}

async fn status(State(s): State<AppState>) -> Json<StatePayload> {
    let cur = *s.state.read();
    Json(cur.into())
}

async fn set_state_ep(
    State(s): State<AppState>,
    Json(req): Json<StateReq>,
) -> impl IntoResponse {
    let next = match PetState::from_key(&req.state) {
        Some(p) => p,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrResp { error: format!("unknown state: {}", req.state) }),
            )
                .into_response();
        }
    };

    // Server-side debounce: drop identical state changes within DEBOUNCE_MS.
    {
        let mut last = s.last_external_set.lock();
        if last.0 == next && last.1.elapsed() < Duration::from_millis(DEBOUNCE_MS) {
            return (StatusCode::OK, Json(OkResp { ok: true })).into_response();
        }
        *last = (next, Instant::now());
    }

    // Suppress auto-cycle's self-reset while external client is driving the pet.
    *s.external_until.lock() = Instant::now() + Duration::from_secs(EXTERNAL_HOLD_SECS);

    // Dedup against current pet state (no event if unchanged).
    {
        let mut w = s.state.write();
        if *w == next {
            return (StatusCode::OK, Json(OkResp { ok: true })).into_response();
        }
        *w = next;
    }
    let payload: StatePayload = next.into();
    if let Err(e) = s.app.emit("state-changed", payload) {
        log::warn!("emit state-changed failed: {}", e);
    }
    (StatusCode::OK, Json(OkResp { ok: true })).into_response()
}

async fn celebrate(State(s): State<AppState>) -> Json<OkResp> {
    *s.external_until.lock() = Instant::now() + Duration::from_secs(EXTERNAL_HOLD_SECS);
    let app = s.app.clone();
    let state = s.state.clone();
    tauri::async_runtime::spawn(async move {
        write_state(&app, &state, PetState::Happy);
        sleep(Duration::from_millis(1500)).await;
        write_state(&app, &state, PetState::IdleLiving);
    });
    Json(OkResp { ok: true })
}

async fn eat(
    State(s): State<AppState>,
    Json(req): Json<EatReq>,
) -> impl IntoResponse {
    let bufs: Vec<PathBuf> = req.paths.into_iter().map(PathBuf::from).collect();
    *s.external_until.lock() = Instant::now() + Duration::from_secs(EXTERNAL_HOLD_SECS);
    match eat_paths(s.app.clone(), s.state.clone(), bufs).await {
        Ok(n) => (StatusCode::OK, Json(EatResp { eaten: n })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrResp { error: e }),
        )
            .into_response(),
    }
}

fn write_state(app: &AppHandle, state: &SharedState, s: PetState) {
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

pub fn spawn_http_server(
    app: AppHandle,
    state: SharedState,
    external_until: ExternalUntil,
) {
    let port: u16 = std::env::var("CLAWD_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9876);

    let app_state = AppState {
        app,
        state,
        external_until,
        last_external_set: Arc::new(Mutex::new((PetState::IdleLiving, Instant::now() - Duration::from_secs(60)))),
    };

    let router = Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/state", post(set_state_ep))
        .route("/celebrate", post(celebrate))
        .route("/eat", post(eat))
        .with_state(app_state);

    tauri::async_runtime::spawn(async move {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => {
                log::info!("HTTP API listening on http://{}", addr);
                if let Err(e) = axum::serve(listener, router).await {
                    log::warn!("HTTP server stopped: {}", e);
                }
            }
            Err(e) => {
                log::warn!("HTTP API bind failed on {}: {} (continuing without API)", addr, e);
            }
        }
    });
}
