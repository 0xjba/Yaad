use std::sync::OnceLock;
use std::time::{Duration, Instant};
use regex::Regex;

static TRANSACTIONAL_REGEX: OnceLock<Regex> = OnceLock::new();

pub struct IntentState {
    pub last_triggered: Option<Instant>,
    pub confirmation_start: Option<Instant>,
    pub current_context_hash: u64,
}

impl Default for IntentState {
    fn default() -> Self {
        Self {
            last_triggered: None,
            confirmation_start: None,
            current_context_hash: 0,
        }
    }
}

pub fn check_utility_trigger(app_name: &str, ocr_text: &str, state: &mut IntentState) -> bool {
    let regex = TRANSACTIONAL_REGEX.get_or_init(|| {
        Regex::new(r"(?i)checkout|billing|cart|pricing|buy|payment|subscribe|order|receipt").unwrap()
    });

    // 1. Cooldown Check (15 mins)
    if let Some(last) = state.last_triggered {
        if last.elapsed() < Duration::from_secs(900) {
            return false;
        }
    }

    // 2. Score based on App Name and OCR
    let is_transactional = regex.is_match(app_name) || regex.is_match(ocr_text);
    
    if !is_transactional {
        state.confirmation_start = None;
        return false;
    }

    // 3. Temporal Confirmation (3-Second Rule)
    match state.confirmation_start {
        None => {
            state.confirmation_start = Some(Instant::now());
            false
        }
        Some(start) => {
            if start.elapsed() >= Duration::from_secs(3) {
                state.last_triggered = Some(Instant::now());
                state.confirmation_start = None;
                true // TRIGGER THE GLOW
            } else {
                false
            }
        }
    }
}

