use std::sync::OnceLock;
use std::time::{Duration, Instant};
use regex::Regex;
use serde::{Deserialize, Serialize};

static TRANSACTIONAL_REGEX: OnceLock<Regex> = OnceLock::new();

// Define the incoming JSON structure from Swift
#[derive(Serialize, Deserialize, Debug)]
pub struct WindowMetadata {
    pub app_name: String,
    pub title: String,
    pub url: String,
    pub error: Option<String>,
}

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

#[derive(PartialEq)]
pub enum TriggerDecision {
    Ignore,
    Monitoring,
    NeedVisual, 
    Activate,
}

pub fn process_metadata_trigger(
    metadata: &WindowMetadata, 
    state: &mut IntentState
) -> TriggerDecision {
    
    // 0. FUTURE: User Blocklist Check
    // if user_settings.blocked_apps.contains(&metadata.app_name) { return TriggerDecision::Ignore; }
    
    let regex = TRANSACTIONAL_REGEX.get_or_init(|| {
        Regex::new(r"(?i)checkout|billing|cart|pricing|buy|payment|subscribe|order|receipt").unwrap()
    });

    // 1. Cooldown Check (15 mins)
    if let Some(last) = state.last_triggered {
        if last.elapsed() < Duration::from_secs(900) {
            return TriggerDecision::Ignore;
        }
    }

    // 2. Score based on Metadata
    let matches_metadata = regex.is_match(&metadata.title) || 
                           regex.is_match(&metadata.url) || 
                           regex.is_match(&metadata.app_name);

    if !matches_metadata {
        state.confirmation_start = None;
        return TriggerDecision::Ignore;
    }

    // 3. Temporal Confirmation (3-Second Rule)
    match state.confirmation_start {
        None => {
            state.confirmation_start = Some(Instant::now());
            TriggerDecision::Monitoring
        }
        Some(start) => {
            if start.elapsed() >= Duration::from_secs(3) {
                state.last_triggered = Some(Instant::now());
                state.confirmation_start = None;
                TriggerDecision::Activate 
            } else {
                TriggerDecision::Monitoring
            }
        }
    }
}

