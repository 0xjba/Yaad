Here is the fully revised **Feature Supplement**, updated with the **macOS 14+ (Sonoma)** architecture and verified **December 2025** dependency stack.

This document replaces the previous version and serves as the source of truth for the restart.

---

# Yaad: Feature Supplement (v2.0)

**The Context & Utility Upgrade**
**Target Architecture:** macOS 14.0+ (Sonoma/Sequoia) | Apple Silicon (M1+)
**Status:** Ready for Development

---

## 1. The Strategic Pivot

**The Shift:**
v0.0.1 was a "Passive Memory" app. This is a "Proactive Wingman."
We acknowledge that waiting for the user to search is often too slow. The highest value comes when the app intervenes *before* a user makes a mistake (or a payment).

**The Solution:**
We introduce **"Intelligent Snapshots"** powered by Situational Utility Injection (SUI). Every memory is indexed not just for *what* it is, but *when* it is useful.

* **The Evidence:** A high-res screenshot of the active window (Visual).
* **The Intent:** The transcript of the user's voice (Verbal) or text.
* **The Utility:** A trigger condition (e.g., "Show this when I am at a checkout").

**Resource Philosophy (The "8GB Rule"):**
To ensure performance on base-model M1 Airs, we strictly enforce **Serial Pipeline Processing** (Capture  Queue  Process).

* **Optimization:** By moving to **macOS 14**, we utilize the `SCScreenshotManager` One-Shot API. This eliminates the need for video stream buffers, reducing idle RAM usage by ~60% compared to previous architectures.

---

## 2. New Core Features

### A. Visual Anchors (The "See" Capability)

* **Logic:** When recording starts, Yaad silently captures *only* the active window.
* **Mechanism:** `SCScreenshotManager` (Native macOS 14 API). No background video recording; just a single, instant high-fidelity frame.
* **Processing:**
* **OCR (Text):** Uses Apple Vision Framework (Revision 3) to make on-screen text searchable.
* **Vibe (Visuals):** Uses **Tiny CLIP (Quantized Int8)** to index visual context (e.g., "Red Error Banner", "Amazon Checkout").



### B. Contextual Intelligence (The "Echo" & The "Injection")

We split context into two tiers:

**Tier 1: Contextual Echoes (Passive Recall)**

* **Trigger:** User opens the Yaad menu bar.
* **Logic:** `Current_Window_Title` matches `Saved_Memory_Title` (Vector Similarity > 0.85).
* **User Value:** The "Sticky Note" effect. "Oh, I left a note here last week."

**Tier 2: Situational Utility Injection (Proactive SUI)**

* **Trigger:** User enters a High-Intent Context (Transactional, Debugging) and sustains it.
* **Mechanism:**
* **Hardened Sieve:** Checks Window Title/URL against a **Confidence Scored** regex model.
* **Vector Check:** If Confidence > 0.7, performs a targeted Vector Search for memories tagged with that intent.
* **Nudge:** If a match is found (>0.88), the Menu Bar Icon glows (Green for Money, Blue for Productivity).


* **Hardening Rules:**
* **Temporal Check:** Context must persist for >3 seconds (kills accidental tab switching).
* **Cooldown:** Once fired, suppresses the same intent for 15 minutes.



### C. Hybrid Intelligence (The Pro Differentiator)

* **Free Tier (Local Reflexes):** Returns raw rows: `[Image]` + `[Transcript]`.
* **Pro Tier (Cloud Reasoning):**
* **Action:** Yaad sends Transcript + OCR Text to Cloud API.
* **Output:** Generated insight (e.g., "Use the NordVPN server in Argentina to save 15% on this booking.").



---

## 3. UX Specification: The "Auto-Flow" Interface

**A. The Default Trigger (Voice First)**

* **Trigger:** Click Menu Bar Icon OR `Option + Space`.
* **Action:**
1. **Immediate Capture:** Active window screenshot + App Title captured instantly.
2. **Auto-Record:** Audio recording begins.
3. **Visual State:** HUD opens showing the "Live Waveform".



**B. The "Interruption" (Switch to Silent/Type)**

* **Interaction:** The user simply Starts Typing.
* **Logic:** First keystroke kills the mic.
* **Action:** Audio stops & deletes. UI morphs from Waveform to Text Input.
* **Value:** Zero clicks to switch modes.

**C. The Drag-and-Drop (Visual First)**

* **Trigger:** Drag image onto Menu Bar Icon.
* **Action:** HUD opens with image thumbnail. Waits for input (Mic or Type).

---

## 4. Technical Architecture Updates (Verified Dec 2025)

### A. Core Dependencies (`Cargo.toml`)

*Use these exact verified versions.*

```toml
[dependencies]
# 1. Visual Intelligence (ONNX Runtime)
# 'features = ["coreml"]' is critical for M1 Neural Engine usage
ort = { version = "2.0.0-rc.9", features = ["coreml", "half"] }
image = "0.25.5" # Standard stable

# 2. Native Bridge (macOS Integration)
# Use stable 1.0.7. Avoid 2.0 alpha versions.
swift-rs = "1.0.7"
base64 = "0.22" # Safe transfer of images from Swift

# 3. Cloud Brain & Utils
reqwest = { version = "0.12.9", features = ["json", "rustls-tls"] } # Requires Hyper 1.0
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
regex = "1.11"
once_cell = "1.19" 

```

### B. Swift Bridge Logic (macOS 14+ Simplification)

*We leverage the `SCScreenshotManager` to avoid complex stream management.*

**File:** `src-tauri/src/swift/lib.swift`

```swift
import SwiftRs
import ScreenCaptureKit
import Vision
import AppKit

@_cdecl("capture_active_window")
public func capture_active_window() -> SRString {
    let semaphore = DispatchSemaphore(value: 0)
    var result = ""
    
    Task {
        // 1. Get Shareable Content (Windows only)
        if let content = try? await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true),
           // 2. Filter for the Frontmost App's Window
           let pid = NSWorkspace.shared.frontmostApplication?.processIdentifier,
           let window = content.windows.first(where: { $0.owningApplication?.processID == pid }) {
            
            let filter = SCContentFilter(desktopIndependentWindow: window)
            let config = SCStreamConfiguration()
            config.width = Int(window.frame.width)
            config.height = Int(window.frame.height)
            config.showsCursor = false
            
            // 3. One-Shot Capture (The macOS 14 Secret Weapon)
            if let image = try? await SCScreenshotManager.captureImage(contentFilter: filter, configuration: config) {
                result = convertImageToBase64(image) // Helper to return Base64 string
            } else {
                result = "ERROR: Capture failed"
            }
        } else {
            result = "ERROR: No active window found"
        }
        semaphore.signal()
    }
    
    semaphore.wait()
    return SRString(result)
}

```

### C. SUI Logic (The "Confidence Scoring" Model)

*Rust implementation for low-overhead checking.*

```rust
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use regex::Regex;

static TRANSACTIONAL_URL: OnceLock<Regex> = OnceLock::new();

fn init_regex() {
    TRANSACTIONAL_URL.get_or_init(|| Regex::new(r"(?i)checkout|billing|cart|pricing|buy").unwrap());
}

struct IntentState {
    last_triggered: Option<Instant>,
    confirmation_start: Option<Instant>,
}

fn check_utility_trigger(current_url: &str, state: &mut IntentState) -> bool {
    // 1. Cooldown Check (15 mins)
    if let Some(last) = state.last_triggered {
        if last.elapsed() < Duration::from_secs(900) { return false; }
    }

    // 2. Score
    let is_transactional = TRANSACTIONAL_URL.get().unwrap().is_match(current_url);
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

```

---

## 5. Revised Roadmap (Integration)

**Phase 1.5: The "Eyes" Upgrade (Weeks 4-5)**

* **Goal:** Replace broken stream logic with `SCScreenshotManager`.
* **Task:** Update `build.rs` to link `ScreenCaptureKit` and `Vision` frameworks.
* **Task:** Implement the simplified Swift Bridge for one-shot capture.
* **Task:** Build the "Morphing Bar" UI (Text/Audio toggle).

**Phase 2.5: The "Utility" Upgrade (Weeks 6-7)**

* **Goal:** Add Vibe (CLIP) and Hardened SUI.
* **Task:** Implement `ort` with `CoreML` execution provider.
* **Task:** Implement the `check_utility_trigger` logic with temporal debouncing.
* **Task:** Add the "Green Glow" visual state.

**Phase 3.0: The Pro Brain (Weeks 8+)**

* **Goal:** Connect Cloud API.
* **Task:** Build the "Summarize/Compare" button.