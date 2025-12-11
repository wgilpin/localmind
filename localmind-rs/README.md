# LocalMind Rust Implementation

A privacy-focused desktop knowledge management system built with Rust and Tauri, featuring a modern Svelte 5 UI.

## Overview

LocalMind-rs is a standalone desktop application that allows you to store and intelligently search notes and bookmarks locally using RAG (Retrieval-Augmented Generation). All data processing happens locally - no data ever leaves your device unencrypted.

## Tech Stack

- **Backend**: Rust with Tauri 1.5
- **Frontend**: Svelte 5 with Vite
- **Database**: SQLite via rusqlite
- **LLM Integration**: Ollama / LM Studio
- **Embedding**: Local embedding models via LM Studio

## Features

- 🔍 Semantic search across bookmarks and notes
- 🤖 AI-powered responses using local LLMs
- 📚 Automatic bookmark monitoring and ingestion
- 🎯 Bookmark folder and domain exclusion rules
- 💬 Streaming LLM responses
- 🎨 Modern, reactive UI built with Svelte 5
- 🔒 100% local - no data leaves your device

## Prerequisites

- Rust (latest stable)
- Node.js 18+ and npm
- [LM Studio](https://lmstudio.ai/) running at `http://localhost:1234`
- **Required:** LM Studio must have `text-embedding-embeddinggemma-300m-qat` model loaded (hard-coded)
- A chat model (e.g., `qwen2.5-coder`, `gemma-3-1b-it`)

## Development Setup

### 1. Install Dependencies

```bash
# Install Rust dependencies
cargo check

# Install Node dependencies for the UI
npm install
```

### 2. Start Development Environment

**Option A: Automatic (using batch script)**
```bash
./dev.bat
```

**Option B: Manual (recommended for VS Code debugging)**

In one terminal:
```bash
npm run dev
```

In another terminal (or VS Code debugger):
```bash
cargo run
```

The Vite dev server will start on `http://localhost:5173`, and Tauri will load the UI from there.

### 3. Configure LM Studio / Ollama

Make sure you have:
- LM Studio running on `http://localhost:1234` with an embedding model loaded
- Or Ollama running on `http://localhost:11434`

## Building for Production

```bash
# Build the UI and create production executable
npm run build
cargo build --release

# Or use Tauri's build command
cargo tauri build
```

The built application will be in `target/release/`.

## Project Structure

```
localmind-rs/
├── src/                      # Rust backend source
│   ├── main.rs              # Application entry point
│   ├── db.rs                # Database operations
│   ├── rag.rs               # RAG pipeline
│   ├── ollama.rs            # LLM client
│   ├── bookmark.rs          # Bookmark monitoring
│   └── bookmark_exclusion.rs # Exclusion rules
├── src-ui/                   # Svelte 5 frontend
│   ├── App.svelte           # Main application component
│   ├── main.js              # Entry point
│   ├── index.html           # HTML template
│   ├── style.css            # Global styles
│   ├── tauri.svelte.js      # Tauri API integration
│   └── components/          # Svelte components
│       ├── SearchBar.svelte
│       ├── SearchResults.svelte
│       ├── DocumentView.svelte
│       ├── SettingsModal.svelte
│       └── Toast.svelte
├── Cargo.toml               # Rust dependencies
├── tauri.conf.json          # Tauri configuration
├── package.json             # Node dependencies
├── vite.config.js           # Vite configuration
└── svelte.config.js         # Svelte configuration
```

## UI Architecture

The UI is built with **Svelte 5** using modern runes for reactive state management:

- **`$state`**: Reactive state variables
- **`$effect`**: Side effects (similar to React useEffect)
- **`$props`**: Component properties
- **`$derived`**: Computed values

### Key Components

- **App.svelte**: Main application shell, manages global state
- **SearchBar.svelte**: Search input and similarity threshold controls
- **SearchResults.svelte**: Displays search results and AI responses
- **DocumentView.svelte**: Full document viewer with navigation
- **SettingsModal.svelte**: Bookmark exclusion settings
- **Toast.svelte**: Notification toasts

### Tauri Integration

The frontend communicates with the Rust backend via Tauri's IPC:

```javascript
import { getTauriAPI } from './tauri.svelte.js';
const { invoke, listen } = getTauriAPI();

// Call Rust command
const results = await invoke('search_hits', { query, cutoff });

// Listen to events from Rust
await listen('llm-stream-chunk', (event) => {
    console.log(event.payload);
});
```

## Database Location

- **Windows**: `%APPDATA%/localmind/`
- **macOS/Linux**: `~/.localmind/`

## Configuration

The application stores configuration in the database, including:
- Embedding model selection
- LLM endpoint URL
- Bookmark exclusion rules

## Development Tips

### Running in VS Code Debugger

1. Keep Vite running in a terminal: `npm run dev`
2. Use the VS Code debugger to run the Rust application
3. This allows you to see the UI updates while debugging the backend

### Hot Reload

- Frontend changes: Auto-reload via Vite HMR
- Backend changes: Requires restarting `cargo run`

### Debugging

- Frontend: Use browser DevTools (Right-click → Inspect)
- Backend: Use Rust debugging tools or add `println!` statements

## Common Commands

```bash
# Check Rust code
cargo check

# Run tests
cargo test

# Build UI only
npm run build

# Start Vite dev server
npm run dev

# Run application
cargo run

# Build release
cargo tauri build
```

## Troubleshooting

### "Failed to resolve module specifier" error
Make sure Vite is running before starting the Tauri app.

### Bookmark monitoring not working
Check that Chrome/Firefox bookmark file is accessible and the file watcher has permissions.

### LLM responses not working
Ensure LM Studio or Ollama is running and has models loaded.

### Database errors
Try deleting the database folder and restarting to reinitialize.

## Contributing

This is the active development version of LocalMind. See the main project README for contribution guidelines.

## License

See the main project LICENSE file.
