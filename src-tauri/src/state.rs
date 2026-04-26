use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Edge {
    Top,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub enum PetState {
    IdleLiving,
    Walking { dir: Direction },
    Climbing { hwnd: isize, edge: Edge, offset: i32 },
    Sleeping,
    Happy,
    Dizzy,
    Dragging,
    Eating,
    GoingAway,
    Disconnected,
    Notification,
    WorkingTyping,
    WorkingThinking,
    WorkingJuggling,
    WorkingBuilding,
    WorkingCarrying,
    WorkingConducting,
    WorkingConfused,
    WorkingDebugger,
    WorkingOverheated,
    WorkingPushing,
    WorkingSweeping,
    WorkingWizard,
    WorkingBeacon,
}

impl PetState {
    pub fn key(&self) -> &'static str {
        match self {
            PetState::IdleLiving => "idle_living",
            PetState::Walking { .. } => "walking",
            PetState::Climbing { .. } => "climbing",
            PetState::Sleeping => "sleeping",
            PetState::Happy => "happy",
            PetState::Dizzy => "dizzy",
            PetState::Dragging => "dragging",
            PetState::Eating => "eating",
            PetState::GoingAway => "going_away",
            PetState::Disconnected => "disconnected",
            PetState::Notification => "notification",
            PetState::WorkingTyping => "working_typing",
            PetState::WorkingThinking => "working_thinking",
            PetState::WorkingJuggling => "working_juggling",
            PetState::WorkingBuilding => "working_building",
            PetState::WorkingCarrying => "working_carrying",
            PetState::WorkingConducting => "working_conducting",
            PetState::WorkingConfused => "working_confused",
            PetState::WorkingDebugger => "working_debugger",
            PetState::WorkingOverheated => "working_overheated",
            PetState::WorkingPushing => "working_pushing",
            PetState::WorkingSweeping => "working_sweeping",
            PetState::WorkingWizard => "working_wizard",
            PetState::WorkingBeacon => "working_beacon",
        }
    }

    pub fn from_key(key: &str) -> Option<PetState> {
        Some(match key {
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
            _ => return None,
        })
    }

    pub fn all_keys() -> &'static [&'static str] {
        &[
            "idle_living", "sleeping", "happy", "dizzy", "dragging", "eating",
            "going_away", "disconnected", "notification",
            "working_typing", "working_thinking", "working_juggling",
            "working_building", "working_carrying", "working_conducting",
            "working_confused", "working_debugger", "working_overheated",
            "working_pushing", "working_sweeping", "working_wizard", "working_beacon",
        ]
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct StatePayload {
    pub state: &'static str,
    pub direction: Option<Direction>,
    pub edge: Option<Edge>,
}

impl From<PetState> for StatePayload {
    fn from(s: PetState) -> Self {
        let (direction, edge) = match s {
            PetState::Walking { dir } => (Some(dir), None),
            PetState::Climbing { edge, .. } => (None, Some(edge)),
            _ => (None, None),
        };
        StatePayload {
            state: s.key(),
            direction,
            edge,
        }
    }
}

pub type SharedState = Arc<RwLock<PetState>>;

pub fn new_shared_state() -> SharedState {
    Arc::new(RwLock::new(PetState::IdleLiving))
}

pub fn working_states() -> &'static [PetState] {
    &[
        PetState::WorkingTyping,
        PetState::WorkingThinking,
        PetState::WorkingJuggling,
        PetState::WorkingBuilding,
        PetState::WorkingConducting,
        PetState::WorkingDebugger,
        PetState::WorkingPushing,
        PetState::WorkingSweeping,
        PetState::WorkingWizard,
        PetState::WorkingBeacon,
    ]
}

// Tracks "external control deadline" — when an HTTP/MCP client sets a state,
// we bump this to now+30s. spawn_auto_cycle skips its self-reset to idle while
// Instant::now() < *external_until, so user-driven states aren't stomped.
pub type ExternalUntil = Arc<Mutex<Instant>>;

pub fn new_external_until() -> ExternalUntil {
    Arc::new(Mutex::new(Instant::now() - Duration::from_secs(60)))
}
