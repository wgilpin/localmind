# LocalMind

A privacy-focused desktop knowledge management system that allows you to store and intelligently search notes and bookmarks locally using RAG (Retrieval-Augmented Generation). All data processing happens locally - no data ever leaves your device.

## Overview

LocalMind is a privacy-focused knowledge management system consisting of three key components:

1. **Desktop Application** - A native Rust/egui desktop app for searching and managing your knowledge base
2. **Chrome Extension** - Browser integration for capturing and ingesting web content directly from Chrome
3. **Embedding Server** - Python FastAPI server that processes documents into embeddings for semantic search

Together, these components provide semantic search across your bookmarks and documents with automatic bookmark monitoring, intelligent exclusion rules, and a native desktop GUI with dark theme support.

## Features

- **Chrome Extension Integration** - Capture web pages, bookmarks, and notes directly from your browser
- Semantic search across bookmarks and documents
- Automatic bookmark monitoring and ingestion
- Bookmark folder and domain exclusion rules
- Native desktop GUI with dark theme (egui/eframe)
- 100% local - no data leaves your device
- Fast, lightweight binary (< 15MB)

## Tech Stack

- **Backend**: Rust with Tokio async runtime
- **Frontend**: egui/eframe (pure Rust, no JavaScript)
- **Database**: SQLite via rusqlite (bundled, no external dependencies)
- **Embedding**: Python FastAPI server with `google/embeddinggemma-300M` model
- **HTTP Server**: axum (for Chrome extension compatibility)

## Prerequisites

- **Rust** (1.75+)
- **Python 3.8+** with FastAPI
- **Embedding server** (see setup below)

## Quick Start

### 1. Start the Embedding Server

The embedding server must be running before starting the application:

```bash
cd embedding-server
python embedding_server.py
```

The server will start on `http://localhost:8000` by default.

### 2. Build and Run the Desktop Application

```bash
cd localmind-rs
cargo run
```

The application will:
- Initialize the SQLite database
- Connect to the embedding server
- Start the HTTP server (port 3000-3010) for Chrome extension
- Launch the egui desktop window

### 3. Install the Chrome Extension

The Chrome extension is a key component that enables capturing web content:

1. Open Chrome and navigate to `chrome://extensions`
2. Enable "Developer mode"
3. Click "Load unpacked" and select the `chrome-extension` directory
4. The extension will automatically connect to the LocalMind HTTP server

Once installed, you can use the extension to capture web pages, bookmarks, and notes directly from your browser.

## Building for Production

```bash
cd localmind-rs
cargo build --release
```

The built executable will be in:
- **Windows**: `target/release/localmind-rs.exe`
- **Linux/macOS**: `target/release/localmind-rs`

**Binary size**: < 15MB (verified)

## Project Structure

```
localmind/
├── localmind-rs/              # Rust implementation (current)
│   ├── src/                   # Rust source code
│   │   ├── main.rs            # Application entry point (eframe)
│   │   ├── gui/               # egui GUI modules
│   │   │   ├── app.rs         # Main application state
│   │   │   ├── state.rs       # UI state types
│   │   │   ├── views/         # View components
│   │   │   └── widgets/       # Reusable widgets
│   │   ├── db.rs              # Database operations
│   │   ├── rag.rs             # RAG pipeline
│   │   ├── bookmark.rs        # Bookmark monitoring
│   │   └── http_server.rs     # HTTP API for Chrome extension
│   ├── Cargo.toml            # Rust dependencies
│   └── README.md              # Detailed Rust implementation docs
├── desktop-daemon/            # Node.js/TypeScript (legacy)
│   └── README.md              # Legacy implementation docs
├── chrome-extension/          # Browser integration (shared)
├── embedding-server/         # Python FastAPI embedding server
└── docs/                      # Documentation and planning
```

## UI Architecture

The UI is built with **egui/eframe** (immediate mode GUI):

- **LocalMindApp**: Main application state implementing `eframe::App`
- **Views**: Home, SearchResults, DocumentDetail
- **Widgets**: Toast, Settings, FolderTree
- **State Management**: Direct access to RAG pipeline via `Arc<RwLock<Option<RagPipeline>>>`

### Key UI Features

- **Dark Theme**: Applied automatically on startup
- **Async Operations**: Uses `poll-promise` for async operations in egui's single-threaded context
- **Toast Notifications**: Auto-dismissing notifications for user feedback
- **Settings Modal**: Manage bookmark exclusion rules (folders and domain patterns)

## Database Location

- **Windows**: `%APPDATA%/localmind/localmind.db`
- **macOS/Linux**: `~/.local/share/localmind/localmind.db`

## Chrome Extension

The Chrome extension is a core component of LocalMind that enables seamless content capture from your browser. It communicates with the desktop application via HTTP API.

### Features

- Capture web pages with full content extraction
- Save bookmarks directly to LocalMind
- Create notes from selected text
- Automatic content processing and embedding

### Installation

1. Open Chrome and navigate to `chrome://extensions`
2. Enable "Developer mode"
3. Click "Load unpacked" and select the `chrome-extension` directory
4. The extension will automatically connect to the LocalMind HTTP server

### HTTP API

The desktop application exposes an HTTP API on port 3000-3010 for Chrome extension communication:

- **POST /documents**: Ingest a document from the Chrome extension
  - Body: `{ "title": "...", "content": "...", "url": "...", "extractionMethod": "..." }`
  - Response: `{ "message": "...", "extractionMethod": "..." }`

## Development

### Common Commands

```bash
# Check Rust code
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy

# Run tests
cargo test

# Build release
cargo build --release

# Run application
cargo run
```

### Running in Debug Mode

```bash
cd localmind-rs
cargo run
```

The application will show console output for debugging.

### Hot Reload

- Code changes require restarting `cargo run`
- The embedding server can be restarted independently

### Debugging

- Use `println!` statements for backend debugging
- egui provides built-in debugging tools (accessible via right-click)

## Troubleshooting

### "Failed to connect to embedding server"
Make sure the Python embedding server is running on `http://localhost:8000`.

### Bookmark monitoring not working
Check that Chrome bookmark file is accessible and the file watcher has permissions.

### HTTP server port conflicts
The application tries ports 3000-3010. If all are in use, check for other instances.

### Database errors
Try deleting the database folder and restarting to reinitialize.

## Legacy Implementation

The repository also contains a legacy Node.js/TypeScript implementation in `desktop-daemon/`:

- **Status**: Maintenance mode, fully functional
- **Technology**: Node.js, TypeScript, ChromaDB, Better-SQLite3
- **Use Case**: For users who prefer the Node.js ecosystem or need ChromaDB features

See `desktop-daemon/README.md` for details on the legacy implementation.

## Migration from Tauri/Svelte

The current Rust implementation replaces the previous Tauri + Svelte frontend with a pure Rust egui implementation:

- No Node.js/JavaScript dependencies
- Single binary executable
- Faster startup time
- Lower memory footprint
- Native look and feel

## Contributing

This is the active development version of LocalMind. Contributions are welcome! Please see the project structure and follow Rust best practices.

## License

See the LICENSE file for details.
