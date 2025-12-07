---
name: Yaad Development Plan
overview: "Comprehensive development plan for Yaad, a voice-powered memory vault for macOS, covering all three phases: Free MVP (local-only), Feedback & Polish, and Pro Upgrade with cloud sync and monetization."
todos:
  - id: setup-env
    content: Set up development environment (Rust, Node.js, Xcode CLI tools) and verify Apple Silicon compatibility
    status: completed
  - id: verify-deps
    content: Verify all Rust dependencies (Tauri v2, sqlite-vec, whisper-rs, fastembed) are available and compatible
    status: completed
  - id: init-project
    content: Initialize Tauri v2 project with React template, configure Tailwind CSS, and set up menu bar app structure
    status: completed
  - id: database-layer
    content: Implement SQLite database with vec_memories virtual table, schema, and cleanup triggers
    status: completed
  - id: embeddings
    content: Integrate fastembed for text embeddings and implement vector search functionality
    status: completed
  - id: whisper-integration
    content: Integrate whisper-rs with CoreML for voice transcription with 60-second hard cap
    status: completed
  - id: capture-ui
    content: Build capture mode UI with glass panel, live transcript, and keyboard shortcuts
    status: completed
  - id: recall-ui
    content: Build recall mode UI with search input, voice search, and result cards
    status: completed
  - id: tauri-commands
    content: Implement Tauri commands for save_memory, search_memories, get_memory, and delete_memory
    status: completed
  - id: phase1-integration
    content: End-to-end integration testing of Phase 1 features (capture, save, search, recall)
    status: completed
  - id: ui-polish
    content: Polish UI animations, transitions, and user feedback in Phase 2
    status: completed
  - id: search-tuning
    content: Tune search quality and ranking algorithms in Phase 2
    status: completed
  - id: first-run
    content: Implement lazy load download system for models on first run (GitHub Releases)
    status: completed
  - id: ghost-search
    content: Implement 15-day recall limit and ghost search (blurred results) for free tier
    status: pending
  - id: supabase-setup
    content: Set up Supabase project, schema, and implement append-only sync strategy
    status: pending
  - id: auth-subscription
    content: Implement authentication and subscription status checking with Supabase
    status: pending
  - id: payment-integration
    content: Integrate payment provider (Stripe/LemonSqueezy) for subscription management
    status: pending
  - id: pro-features-ui
    content: Build Pro features UI (context editing, sync status, subscription management)
    status: pending
---

# Yaad Development Plan

## Overview

This plan covers the complete development of Yaad across all three phases as specified in the PRD. The project will be built using Tauri v2 (Rust + React), with a local-first architecture using SQLite, vector search, and on-device AI processing.

## Prerequisites & Setup

### Development Environment Setup

- Install Rust toolchain (rustup)
- Install Node.js and npm/yarn
- Install Xcode Command Line Tools (via `xcode-select --install` - provides compiler, linker, and macOS SDK; full Xcode IDE not required)
- Verify Apple Silicon (M1/M2/M3) compatibility
- Set up Git repository

### Technical Stack Verification

During initial setup, verify:

- Tauri v2.0 availability and stability
- `sqlite-vec` crate compatibility with SQLite
- `whisper-rs` with CoreML feature support
- `fastembed` Rust crate availability
- `rusqlite` with bundled feature

**Note**: If any dependency is unavailable or incompatible, document alternatives and get approval before proceeding.

## Phase 1: Free MVP (Weeks 1-4)

### Project Structure

```
yaad/
├── src-tauri/          # Rust backend
│   ├── Cargo.toml
│   ├── src/
│   │   ├── mai
n












.rs
│   │   ├── db.rs       # SQLite + sqlite-vec setup
│   │   ├── embeddings.rs # fastembed integration
│   │   ├── whisper.rs  # whisper-rs transcription
│   │   └── commands.rs # Tauri commands
│   └── tauri.conf.json
├── src/                # React frontend
│   ├── App.tsx
│   ├── components/
│   │   ├── MenuBar.tsx
│   │   ├── CapturePanel.tsx
│   │   ├── RecallPanel.tsx
│   │   └── SearchResults.tsx
│   └── styles/
└── package.json
```

### 1.1 Project Initialization

- Initialize Tauri v2 project with React template
- Configure `Cargo.toml` with all dependencies from PRD
- Set up Tailwind CSS for frontend styling
- Configure Tauri for menu bar app (no dock icon)
- Set up dual menu bar icons (search + mic)

**Files**: `Cargo.toml`, `tauri.conf.json`, `package.json`, `tailwind.config.js`

### 1.2 Database Layer

- Create SQLite database with schema from PRD
- Implement `vec_memories` virtual table using sqlite-vec
- Set up cleanup trigger for deleted memories
- Create database initialization function
- Add database path management (App_Data directory)

**Files**: `src-tauri/src/db.rs`

### 1.3 Embeddings Integration

- Integrate `fastembed` with `all-MiniLM-L6-v2` model
- Implement embedding generation for text
- Store embeddings in `vec_memories` table
- Create embedding search function (vector similarity)

**Files**: `src-tauri/src/embeddings.rs`

### 1.4 Whisper Transcription

- Integrate `whisper-rs` with CoreML feature
- Configure for Apple Neural Engine usage
- Implement 60-second hard cap for recordings
- Create transcription command/function
- Handle real-time transcription streaming (if supported)

**Files**: `src-tauri/src/whisper.rs`

### 1.5 Capture Mode UI

- Build glass panel dropdown component
- Implement mic icon with red/wiggle animation
- Create live transcript display
- Add keyboard shortcuts (Option+Space, Esc, Enter)
- Implement save/discard functionality
- Connect to Rust backend for transcription

**Files**: `src/components/CapturePanel.tsx`, `src/components/MenuBar.tsx`

### 1.6 Recall Mode UI

- Build search panel with input field
- Implement voice search button (internal mic)
- Create search results card component
- Display results with top card expanded, others collapsed
- Connect to Rust backend for vector search

**Files**: `src/components/RecallPanel.tsx`, `src/components/SearchResults.tsx`

### 1.7 Tauri Commands

- Create `save_memory` command (transcription → embedding → DB)
- Create `search_memories` command (query → embedding → vector search)
- Create `get_memory` command (retrieve by ID)
- Create `delete_memory` command (soft delete)
- Implement error handling and validation

**Files**: `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

### 1.8 Integration & Testing

- End-to-end testing: capture → save → search → recall
- Test 60-second recording limit
- Test vector search accuracy
- Test keyboard shortcuts
- Performance testing (embedding generation speed)

## Phase 2: Feedback & Polish (Weeks 5-6)

### 2.1 UI/UX Enhancements

- Polish "wiggle" animation for mic icon
- Smooth glass panel animations
- Improve result card transitions
- Add loading states and feedback
- Refine typography and spacing

**Files**: `src/components/*.tsx`, `src/styles/*.css`

### 2.2 Search Quality Improvements

- Implement result ranking algorithm tuning
- Add "HyDE-lite" query expansion (if applicable)
- Improve embedding search parameters
- Add relevance scoring adjustments

**Files**: `src-tauri/src/embeddings.rs`, `src-tauri/src/commands.rs`

### 2.3 First Run Experience

- Implement "Lazy Load" download system
- Create "Optimizing Neural Engine..." loading screen
- Download `ggml-base.bin` from GitHub Releases
- Download CoreML model folder
- Download ONNX model for embeddings
- Store models in App_Data directory
- Show progress indicator during download

**Files**: `src/components/FirstRun.tsx`, `src-tauri/src/main.rs`

### 2.4 Error Handling & Edge Cases

- Handle microphone permission errors
- Handle transcription failures gracefully
- Handle database corruption scenarios
- Add user-friendly error messages
- Implement retry mechanisms

**Files**: `src-tauri/src/*.rs`, `src/components/*.tsx`

### 2.5 Performance Optimization

- Optimize embedding generation speed
- Optimize vector search queries
- Reduce memory footprint
- Improve app startup time
- Battery usage optimization

## Phase 3: Pro Upgrade (Weeks 7+)

### 3.1 Database Schema Updates

- Add `context_url` and `context_note` fields to memories table
- Add `tags` support (if needed)
- Add subscription status tracking
- Add sync metadata fields

**Files**: `src-tauri/src/db.rs`

### 3.2 15-Day Recall Limit Logic

- Implement date-based filtering for free tier
- Create "Ghost Search" functionality
- Build blurred/frosted card component for locked results
- Add "Unlock Vault" CTA on ghost cards
- Implement query date checking logic

**Files**: `src-tauri/src/commands.rs`, `src/components/SearchResults.tsx`

### 3.3 Supabase Integration

- Set up Supabase project structure
- Create Postgres schema matching local SQLite
- Implement append-only log sync strategy (CRDT-lite)
- Create sync commands (upload, download, merge)
- Handle conflict resolution
- Implement background sync

**Files**: `src-tauri/src/supabase.rs`, `src-tauri/src/sync.rs`

### 3.4 Authentication & Subscription

- Integrate Supabase Auth
- Create user account system
- Implement subscription status checking
- Add subscription management UI
- Create upgrade flow

**Files**: `src-tauri/src/auth.rs`, `src/components/Subscription.tsx`

### 3.5 Payment Integration

- Choose payment provider (Stripe or LemonSqueezy)
- Set up payment provider account
- Implement subscription creation
- Create payment success/failure handlers
- Add subscription status sync

**Files**: `src-tauri/src/payment.rs`, `src/components/Payment.tsx`

### 3.6 Pro Features UI

- Add context editing UI (URL, notes, tags)
- Implement context chips on result cards
- Add cloud sync status indicator
- Create settings panel for Pro features
- Add subscription management UI

**Files**: `src/components/*.tsx`

### 3.7 AI Clusters (Future Enhancement)

- Research clustering algorithm
- Implement related thoughts grouping
- Create cluster visualization UI
- Add cluster-based navigation

**Files**: `src-tauri/src/clusters.rs`, `src/components/Clusters.tsx`

## Key Implementation Notes

### Dependencies to Verify

- `tauri = { version = "2.0", features = [] }`
- `rusqlite = { version = "0.31", features = ["bundled", "uuid"] }`
- `sqlite-vec = "0.1"`
- `fastembed = "3"`
- `whisper-rs = { version = "0.13", features = ["coreml"] }`

### Critical Constraints

- 60-second hard cap on recordings
- Apple Silicon only (M1/M2/M3)
- Local-first architecture (Phase 1)
- Append-only sync strategy (Phase 3)

### Testing Strategy

- Unit tests for Rust backend functions
- Integration tests for Tauri commands
- E2E tests for capture/recall workflows
- Performance benchmarks for embeddings/search
- Battery impact testing for CoreML usage

## Risk Mitigation

1. **Dependency Unavailability**: If any Rust crate is unavailable, research alternatives and get approval
2. **CoreML Performance**: Test whisper-rs CoreML performance early; have CPU fallback ready
3. **Vector Search Quality**: Implement search quality metrics and tuning mechanisms
4. **Sync Conflicts**: Design robust conflict resolution for Phase 3
5. **Model Downloads**: Ensure reliable hosting and fallback mechanisms for first-run downloads