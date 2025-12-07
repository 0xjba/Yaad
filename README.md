# Yaad - Voice-Powered Memory Vault

**The Voice-Powered Memory Vault for macOS**  
*Local-First. Fleeting Thoughts Only.*

Yaad is a lightweight macOS menu bar app that serves as an "external hard drive" for your brain. It uses **voice** for frictionless capture and **semantic search** for instant recall.

## Features

- **Voice Capture**: Record and transcribe your thoughts using Whisper (CoreML on Apple Neural Engine)
- **Semantic Search**: Find memories using natural language queries powered by vector embeddings
- **Local-First**: All data stored locally on your device
- **Fast**: Optimized for Apple Silicon (M1/M2/M3) with zero-latency processing
- **Menu Bar App**: Runs in macOS menu bar with dual icons (search + mic)

## Development Status

### ✅ Phase 1: Free MVP (Complete)

- [x] Project structure initialized
- [x] Tauri v2 + React setup
- [x] Database schema (SQLite + sqlite-vec)
- [x] Embeddings integration (fastembed)
- [x] Whisper integration (whisper-rs with CoreML)
- [x] Audio capture and resampling (rubato)
- [x] UI components (Capture, Recall, MenuBar, SearchResults)
- [x] Tauri commands implementation
- [x] Model downloads from Hugging Face Hub
- [x] First-run experience
- [x] End-to-end integration

### ✅ Phase 2: Feedback & Polish (Complete)

- [x] UI/UX enhancements (animations, transitions)
- [x] Search quality improvements (ranking, relevance scoring)
- [x] Error handling (microphone, transcription, database)
- [x] Performance optimization (database, startup, memory)

### 🔄 Phase 3: Pro Upgrade (Pending)

- [ ] 15-day recall limit for free tier
- [ ] Ghost search (blurred results)
- [ ] Supabase integration (cloud sync)
- [ ] Authentication & subscription
- [ ] Payment integration
- [ ] Pro features UI

## Prerequisites

- Rust toolchain (rustup) - Latest stable version
- Node.js (v18+) and npm
- Xcode Command Line Tools (`xcode-select --install`)
- Apple Silicon Mac (M1/M2/M3) - Required for CoreML

## Setup

1. **Install dependencies:**
```bash
npm install
```

2. **Build and run in development:**
```bash
npm run tauri dev
```

3. **Build for production:**
```bash
npm run tauri build
```

## First Run

On first launch, the app will:
1. Show "Optimizing Neural Engine..." screen
2. Download required models from Hugging Face Hub:
   - Whisper Base model (`ggml-base.bin`)
   - Whisper CoreML model (`ggml-base-encoder.mlmodelc.zip`)
   - Embedding model (`model_quantized.onnx`)
3. Store models in app data directory
4. Initialize embeddings and whisper models

**Note**: First run may take ~30-60 seconds depending on download speed.

## Project Structure

```
yaad/
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs         # Tauri app entry point
│   │   ├── db.rs           # SQLite + sqlite-vec setup
│   │   ├── embeddings.rs  # fastembed integration
│   │   ├── whisper.rs      # whisper-rs transcription
│   │   ├── commands.rs     # Tauri commands
│   │   ├── models.rs       # Model download management
│   │   └── tests.rs        # Unit tests
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                    # React frontend
│   ├── App.tsx
│   ├── components/
│   │   ├── MenuBar.tsx     # Menu bar icons
│   │   ├── CapturePanel.tsx # Voice capture UI
│   │   ├── RecallPanel.tsx  # Search UI
│   │   ├── SearchResults.tsx # Search results display
│   │   └── FirstRun.tsx     # First-run download screen
│   ├── styles/
│   │   └── index.css       # Global styles + animations
│   └── types.ts
├── product-definition.md   # Full product specification
└── package.json
```

## Technical Stack

### Backend (Rust)
- **Framework**: Tauri v2 with macOS private API
- **Database**: SQLite with sqlite-vec (0.1.7-alpha.2) for vector search
- **Embeddings**: fastembed (all-MiniLM-L6-v2 via ONNX)
- **Transcription**: whisper-rs (0.13) with CoreML feature
- **Audio**: cpal (0.15) for capture, rubato (0.14) for resampling
- **HTTP**: reqwest (0.11) for model downloads

### Frontend (React + TypeScript)
- **Framework**: React 18 with TypeScript
- **Styling**: Tailwind CSS
- **Build**: Vite

## Key Features Implementation

### Voice Capture
- Real-time audio capture via cpal
- Automatic resampling from device rate (44.1kHz/48kHz) to 16kHz
- Whisper transcription with CoreML (Apple Neural Engine)
- 60-second hard cap on recordings
- Manual stop button (toggle)

### Semantic Search
- Vector embeddings using fastembed
- Similarity search via sqlite-vec
- Relevance ranking with boost for close matches
- Results sorted by similarity score

### Database
- SQLite with WAL mode for performance
- Vector search via sqlite-vec vec0 virtual table
- Soft delete with cleanup triggers
- Optimized cache size (16MB)

### Model Management
- Lazy loading: Models downloaded on first run
- Hugging Face Hub integration
- Models stored in app data directory
- Automatic model initialization

## Usage

### Capture Mode
1. Click mic icon in menu bar (or press `Option+Space`)
2. Speak your thought (up to 60 seconds)
3. Click mic again to stop recording
4. Review transcribed text
5. Press `Enter` to save or `Esc` to discard

### Recall Mode
1. Click search icon in menu bar (or press `Option+Space`)
2. Type your search query or click mic for voice search
3. Review results (top result expanded by default)
4. Click results to expand/collapse
5. Press `Esc` to close

## Keyboard Shortcuts

- `Option+Space`: Toggle capture/recall mode
- `Enter`: Save memory (capture) or search (recall)
- `Esc`: Close panel or discard

## Data Storage

- **Database**: `~/Library/Application Support/com.yaad.app/yaad.db`
- **Models**: `~/Library/Application Support/com.yaad.app/models/`
- Uses Tauri's `app_local_data_dir()` API (cross-platform compatible)

## Performance Optimizations

- **Database**: WAL mode, 16MB cache, NORMAL synchronous mode
- **Startup**: Background embedding initialization (non-blocking)
- **Search**: Enhanced ranking algorithm with relevance boost
- **Memory**: Optimized connection handling and query execution

## Error Handling

The app provides user-friendly error messages for:
- Microphone permission issues
- Transcription failures
- Database errors
- Model loading issues
- Network errors (model downloads)

## Known Issues

- `ort` dependency compilation errors (from fastembed, not our code)
  - May need to update fastembed version or handle separately
- sqlite-vec API verification pending (will test during integration)

## Development

### Running Tests
```bash
cd src-tauri
cargo test
```

### Building
```bash
npm run tauri build
```

### Debugging
- Rust: Use `eprintln!` for console output
- Frontend: Browser DevTools (when window is visible)

## License

See `product-definition.md` for full product specifications and business model.

## Contributing

This is a private project. For questions or issues, refer to the product definition document.
