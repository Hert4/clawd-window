use parking_lot::RwLock;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    /// Active phase — pet walks around, climbs, full energy.
    Active,
    /// Resting phase — pet stays put, only plays reaction states.
    Resting,
    /// Sleepy — user has been idle for a while, pet sleeps.
    Sleepy,
}

pub type SharedMood = Arc<RwLock<Mood>>;

pub fn new_shared_mood() -> SharedMood {
    Arc::new(RwLock::new(Mood::Active))
}

const IDLE_THRESHOLD_SECS: u64 = 180; // 3 min — user idle => Sleepy
const MOOD_TICK_SECS: u64 = 30;
const MOOD_SWITCH_MIN_SECS: u64 = 150; // ~2.5 min minimum in a non-sleepy mood
const MOOD_SWITCH_MAX_SECS: u64 = 300; // ~5 min maximum

#[cfg(windows)]
fn user_idle_secs() -> u64 {
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    unsafe {
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if !GetLastInputInfo(&mut info).as_bool() {
            return 0;
        }
        let now = GetTickCount();
        let last = info.dwTime;
        let delta_ms = now.wrapping_sub(last);
        (delta_ms / 1000) as u64
    }
}

#[cfg(not(windows))]
fn user_idle_secs() -> u64 {
    0
}

pub fn spawn_mood_thread(mood: SharedMood) {
    tauri::async_runtime::spawn(async move {
        let mut rng = SmallRng::from_entropy();
        // When the next active<->resting flip is allowed (only used when not Sleepy).
        let mut next_flip_in = rng.gen_range(MOOD_SWITCH_MIN_SECS..MOOD_SWITCH_MAX_SECS);
        let mut elapsed: u64 = 0;
        // Remember pre-sleepy mood so we can restore it when user comes back.
        let mut prev_mood: Mood = Mood::Active;

        loop {
            sleep(Duration::from_secs(MOOD_TICK_SECS)).await;
            elapsed += MOOD_TICK_SECS;

            let idle = user_idle_secs();
            let current = *mood.read();

            if idle >= IDLE_THRESHOLD_SECS {
                if current != Mood::Sleepy {
                    prev_mood = current;
                    *mood.write() = Mood::Sleepy;
                    log::info!("[mood] user idle {}s -> Sleepy", idle);
                }
                continue;
            }

            // User is active. If we were Sleepy, wake up to previous mood.
            if current == Mood::Sleepy {
                *mood.write() = prev_mood;
                log::info!("[mood] user back -> {:?}", prev_mood);
                elapsed = 0;
                next_flip_in = rng.gen_range(MOOD_SWITCH_MIN_SECS..MOOD_SWITCH_MAX_SECS);
                continue;
            }

            // Periodic Active <-> Resting flip.
            if elapsed >= next_flip_in {
                let next = match current {
                    Mood::Active => Mood::Resting,
                    Mood::Resting => Mood::Active,
                    Mood::Sleepy => Mood::Active, // unreachable here, defensive
                };
                *mood.write() = next;
                log::info!("[mood] flip {:?} -> {:?}", current, next);
                elapsed = 0;
                next_flip_in = rng.gen_range(MOOD_SWITCH_MIN_SECS..MOOD_SWITCH_MAX_SECS);
            }
        }
    });
}
