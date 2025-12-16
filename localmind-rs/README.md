# LocalMind Rust Implementation

A privacy-focused desktop knowledge management system built with Rust and egui, featuring a native desktop GUI.

## Overview

LocalMind-rs is a standalone desktop application that allows you to store and intelligently search notes and bookmarks locally using RAG (Retrieval-Augmented Generation). All data processing happens locally - no data ever leaves your device.

## Tech Stack

- **Backend**: Rust
- **Frontend**: egui/eframe (pure Rust, no JavaScript)
- **Database**: SQLite via rusqlite
- **Embedding**: Python FastAPI server with `google/embeddinggemma-300M` model
- **HTTP Server**: axum (for Chrome extension compatibility)

## Features

- 🔍 Semantic search across bookmarks and documents
- 📚 Automatic bookmark monitoring and ingestion
- 🎯 Bookmark folder and domain exclusion rules
- 🎨 Native desktop GUI with dark theme
- 🔒 100% local - no data leaves your device
- 📱 Chrome extension support via HTTP API

## Prerequisites

- Rust (1.75+)
- Python 3.8+ with FastAPI
- Embedding server running (see `embedding-server/` directory)

## Development Setup

### 1. Start the Embedding Server

The embedding server must be running before starting the application:

```bash
cd embedding-server
python embedding_server.py
```

The server will start on `http://localhost:8000` by default.

### 2. Build and Run

```bash
cd localmind-rs
cargo run
```

The application will:
- Initialize the SQLite database
- Connect to the embedding server
- Start the HTTP server (port 3000-3010) for Chrome extension
- Launch the egui desktop window

## Building for Production

```bash
cd localmind-rs
cargo build --release
```

The built executable will be in `target/release/localmind-rs.exe` (Windows) or `target/release/localmind-rs` (Linux/macOS).

**Binary size**: < 15MB (verified)

## Project Structure

```
localmind-rs/
├── src/                      # Rust source code
│   ├── main.rs              # Application entry point (eframe)
│   ├── gui/                  # egui GUI modules
│   │   ├── app.rs            # Main application state and eframe::App implementation
│   │   ├── state.rs          # UI state types (View, InitStatus, Toast, etc.)
│   │   ├── views/            # View components
│   │   │   ├── home.rs       # Home screen with recent documents
│   │   │   ├── search.rs     # Search results view
│   │   │   └── document.rs   # Document detail view
│   │   └── widgets/          # Reusable widgets
│   │       ├── toast.rs      # Toast notifications
│   │       ├── settings.rs   # Settings modal
│   │       └── folder_tree.rs # Bookmark folder tree
│   ├── db.rs                # Database operations
│   ├── rag.rs               # RAG pipeline
│   ├── bookmark.rs          # Bookmark monitoring
│   ├── bookmark_exclusion.rs # Exclusion rules
│   └── http_server.rs       # HTTP API for Chrome extension
├── icons/                    # Application icons
├── Cargo.toml               # Rust dependencies
└── README.md                # This file
```

## UI Architecture

The UI is built with **egui/eframe** (immediate mode GUI):

- **`LocalMindApp`**: Main application state implementing `eframe::App`
- **Views**: Home, SearchResults, DocumentDetail
- **Widgets**: Toast, Settings, FolderTree
- **State Management**: Direct access to RAG pipeline via `Arc<RwLock<Option<RagPipeline>>>`

### Key Features

- **Dark Theme**: Applied automatically on startup
- **Async Operations**: Uses `poll-promise` for async operations in egui's single-threaded context
- **Toast Notifications**: Auto-dismissing notifications for user feedback
- **Settings Modal**: Manage bookmark exclusion rules (folders and domain patterns)

## Database Location

- **Windows**: `%APPDATA%/localmind/localmind.db`
- **macOS/Linux**: `~/.local/share/localmind/localmind.db`

## HTTP API

The application exposes an HTTP API on port 3000-3010 for Chrome extension compatibility:

- **POST /documents**: Ingest a document from the Chrome extension
  - Body: `{ "title": "...", "content": "...", "url": "...", "extractionMethod": "..." }`
  - Response: `{ "message": "...", "extractionMethod": "..." }`

## Configuration

The application stores configuration in the database, including:
- Bookmark exclusion rules (folders and domain patterns)
- Document metadata and embeddings

## Development Tips

### Running in Debug Mode

```bash
cargo run
```

The application will show console output for debugging.

### Hot Reload

- Code changes require restarting `cargo run`
- The embedding server can be restarted independently

### Debugging

- Use `println!` statements for backend debugging
- egui provides built-in debugging tools (accessible via right-click)

## Common Commands

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

## Troubleshooting

### "Failed to connect to embedding server"
Make sure the Python embedding server is running on `http://localhost:8000`.

### Bookmark monitoring not working
Check that Chrome bookmark file is accessible and the file watcher has permissions.

### HTTP server port conflicts
The application tries ports 3000-3010. If all are in use, check for other instances.

### Database errors
Try deleting the database folder and restarting to reinitialize.

## Migration from Tauri/Svelte

This version replaces the previous Tauri + Svelte frontend with a pure Rust egui implementation:
- ✅ No Node.js/JavaScript dependencies
- ✅ Single binary executable
- ✅ Faster startup time
- ✅ Lower memory footprint
- ✅ Native look and feel

## Contributing

This is the active development version of LocalMind. See the main project README for contribution guidelines.

## License

See the main project LICENSE file.
