
# 🧠 Yaad: Master Product Definition

**The Voice-Powered Memory Vault for macOS(wider support in the future)**
*Local-First. Fleeting Thoughts Only.*

-----

## 1\. Executive Summary & Vision

**The Problem:**
Human memory is leaky. We have fleeting thoughts—"Buy milk," "Great startup idea," "Meeting takeaway"—that disappear before we can type them.

  * **Notes Apps (Notion/Obsidian):** Too slow. They require structure and typing.
  * **Siri/Voice Memos:** Too dumb. They don't understand *meaning* or context.

**The Solution:**
**Yaad** is a lightweight macOS menu bar app that serves as an "external hard drive" for your brain. It uses **voice** for frictionless capture and **semantic search** for instant recall.

**The Edge:**

  * **Zero Latency:** Runs 100% locally on Apple Silicon (M1/M2/M3) using the Neural Engine.
  * **Zero Intrusion:** "Invisible" UI that respects screen real estate.
  * **Fleeting Only:** Optimized for micro-thoughts (max 60s), ensuring high-quality search and focus.

**Pitch Headline:** *"While others build Chatbots, we are building the Hippocampus."*

-----

## 2\. Business Model: The "Data Hostage" Strategy

We utilize a **High-Friction Freemium Model**. We encourage free users to store *unlimited* data (creating dependency and sunk cost), then gate the *retrieval* of older memories to drive conversion.

**Pricing:** \~$10/Month or $100/Year.

| Feature | **Free Tier (The Hook)** | **Pro Tier (The Vault)** |
| :--- | :--- | :--- |
| **Hardware** | **Apple Silicon Only** (M1/M2+) | **Apple Silicon Only** |
| **Storage** | **Unlimited** (Keep saving forever) | **Unlimited** |
| **Recall Window** | **Recent 15 Days Only** | **Unlimited History** |
| **Context** | Basic Voice Only. | **Rich Context** (Add URL, Text Note, Tags). |
| **Search** | Recent matches only. | **"Ghost Search"** (Blurred results \> 15 days). |
| **Sync** | Local Device Only. | **Cloud Sync** (Mac + Future iOS). |
| **Intelligence** | Basic Keywords. | **AI Clusters** (Grouping related thoughts). |

### The "Ghost Search" Conversion Trigger

When a Free user searches for a memory older than 15 days:

1.  **System:** Finds the memory using local vector search (it *knows* the answer).
2.  **UI:** Displays the result card, but the text is **Blurred / Frosted**.
3.  **CTA:** *"Found match from Oct 12. Unlock Vault to reveal."*
4.  **Psychology:** This proves the app works and has the specific answer they need right now.

-----

## 3\. Technical Architecture (The "Golden Stack")

**Philosophy:** Thick Client (Device does the work), Thin Server (Dumb storage).

### A. The Local Stack (macOS)

  * **Framework:** **Tauri v2** (Rust Backend + React Frontend). \~15MB Binary.
  * **Database:** **SQLite** (bundled via `rusqlite`).
  * **Vector Engine:** **`sqlite-vec`**. Stores embeddings directly in SQLite (No external vector DB).
  * **The Brain (Embeddings):** **`fastembed-rs`** running `all-MiniLM-L6-v2` via ONNX on CPU.
  * **The Ears (Transcription):** **`whisper-rs`** (Rust bindings).
      * **Config:** `features = ["coreml"]`.
      * **Hardware:** Uses Apple Neural Engine (ANE). Zero battery impact.
      * **Hard Cap:** **Max 60 seconds.** (Prevents vector dilution).

### B. The Sync Architecture (Pro Only)

  * **Backend:** **Supabase** (Postgres + Auth).
  * **Sync Strategy:** **Append-Only Log** (CRDT-lite).
      * We never "Update" rows to avoid conflict hell.
      * We only "Insert" new versions of a memory with a newer timestamp.
      * The Client downloads the log and reconstructs the latest state.

### C. Deployment (The "Lazy Load")

To avoid a massive installer:

1.  **Installer:** \~15MB (App logic only).
2.  **First Run:** App shows *"Optimizing Neural Engine... (This takes \~10s)"*.
3.  **Action:** App downloads `ggml-base.bin`, compiled CoreML model folder, and ONNX model to `App_Data`.

-----

## 4\. UI/UX Specification: The "Dual-Icon" Interface

**Concept:** Two distinct click targets in the menu bar for speed.
**Layout:** `[ 🔍 ] [ 🎙️ ]`

### A. Capture Mode (`[🎙️]`)

  * **Trigger:** Click Mic Icon OR Hold `Option + Space`.
  * **Visual:**
      * **Icon:** Turns Red and **Wiggles** (Active "Alive" State).
      * **Window:** Glass Panel drops down immediately.
  * **Interaction:**
      * **Auto-Record:** Starts listening instantly upon open.
      * **Live Transcript:** Text appears as you speak in real-time.
      * **Discard:** Press `Esc` or click `X` to "Poof" (delete without saving).
      * **Save:** Press `Enter` or click `Stop` to save.

### B. Recall Mode (`[🔍]`)

  * **Trigger:** Click Search Icon OR Tap `Option + Space`.
  * **Visual:** Glass Panel drops down.
  * **Interaction:**
      * **Input:** Cursor focused in search bar. Ready to type "Passport".
      * **Voice Search:** Click the **Internal Mic Button** inside the search bar.
          * User asks: *"Where is my passport?"*
          * App transcribes query → Runs Search immediately.
      * **The Stack:** Results appear as cards (Top Expanded, Lower Collapsed).
      * **Pro Context:** Clickable chips for URLs/Notes appear on cards.
      * **Ghost Cards:** If result \> 15 days (Free Tier), card is blurred with a Lock icon.

-----

## 5\. Developer Handoff Package

### `Cargo.toml` Dependencies

Copy this exactly to ensure the right features are enabled.

```toml
[dependencies]
tauri = { version = "2.0", features = [] }
serde = { version = "1.0", features = ["derive"] }
# The Database
rusqlite = { version = "0.31", features = ["bundled", "uuid"] }
# Vector Search
sqlite-vec = "0.1"
# Embeddings (CPU)
fastembed = "3"
# Transcription (Apple Silicon / Neural Engine)
# CoreML is critical for battery life.
whisper-rs = { version = "0.13", features = ["coreml"] }
```

### Database Schema (SQL)

Updated to support the new "Context" features and 60s limit tracking.

```sql
-- 1. Human Readable Data
CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY,               -- UUID (v4)
    content TEXT NOT NULL,             -- Transcribed Text
    duration_sec INTEGER,              -- Max 60
    
    -- Rich Context (Pro Features)
    context_url TEXT,                  -- Optional: Linked URL
    context_note TEXT,                 -- Optional: Typed note added later
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_synced BOOLEAN DEFAULT 0,
    is_deleted BOOLEAN DEFAULT 0
);

-- 2. Machine Searchable Data (Virtual Table)
CREATE VIRTUAL TABLE vec_memories USING vec0(
    embedding float[384]               -- MiniLM Dimensions
);

-- 3. Cleanup Trigger
CREATE TRIGGER IF NOT EXISTS delete_vector 
AFTER UPDATE OF is_deleted ON memories 
WHEN NEW.is_deleted = 1
BEGIN
  DELETE FROM vec_memories WHERE rowid = NEW.rowid;
END;
```

-----

## 6\. Development Phasing (Revised)

### Phase 1: The "Free MVP" (Weeks 1-4)

  * **Goal:** A fully functional **Local-Only** app. No Payment, No Sync.
  * **Focus:** Nailing the "Magic" of capture and recall speed.
  * **Deliverables:**
      * Tauri App with "Dual-Icon" Menu Bar.
      * `whisper-rs` (CoreML) integration.
      * `sqlite-vec` + `fastembed` working locally.
      * 60s Hard Cap Logic.
      * **Release Strategy:** Launch on Twitter/Reddit/ProductHunt as "Free Beta."

### Phase 2: The "Feedback & Polish" (Weeks 5-6)

  * **Goal:** Fix bugs found by early users.
  * **Focus:** Search quality tuning ("HyDE-lite" tweaks) and UI smoothness (Animations).
  * **Deliverables:**
      * "Wiggle" animation polish.
      * Result Ranking algorithm tuning.
      * First Run "Lazy Load" UX refinement.

### Phase 3: The "Pro" Upgrade (Weeks 7+)

  * **Goal:** Turn on the Revenue.
  * **Focus:** The "Ghost Search" and Cloud Sync.
  * **Deliverables:**
      * Implement 15-day Query Limit logic ("Ghost Cards").
      * Integrate Supabase Sync.
      * Add Stripe/LemonSqueezy integration.
      * Launch "Yaad Pro."