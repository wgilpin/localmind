# CLAUDE.md - LocalMind Project

This file provides guidance for working with the LocalMind project structure.

## Project Overview

LocalMind is a privacy-focused knowledge management system that allows users to store and intelligently search notes and bookmarks locally using RAG (Retrieval-Augmented Generation). All data processing happens locally - no data ever leaves the user's device unencrypted.

## Repository Structure

This repository contains multiple implementations of LocalMind:

```
localmind/
├── desktop-daemon/          # Node.js/TypeScript implementation (legacy)
│   ├── CLAUDE.md           # TypeScript-specific guidance
│   ├── src/                # TypeScript source code
│   ├── frontend/           # SvelteKit web application
│   └── package.json        # Node.js dependencies
├── localmind-rs/           # Rust implementation (current development)
│   ├── CLAUDE.md           # Rust-specific guidance
│   ├── src/                # Rust source code
│   │   ├── gui/            # egui GUI modules
│   └── Cargo.toml          # Rust dependencies
├── chrome-extension/        # Browser integration (shared)
├── docs/                   # Documentation and planning
└── CLAUDE.md               # This file - project overview
```

## Implementation Status

### TypeScript Implementation (Legacy) 🟡
- **Status**: Maintenance mode, fully functional
- **Location**: `desktop-daemon/`
- **Technology**: Node.js, TypeScript, ChromaDB, Better-SQLite3
- **Use Case**: Current production version, feature-complete
- **Documentation**: See `desktop-daemon/CLAUDE.md`

### Rust Implementation (Active Development) 🟢
- **Status**: Desktop application with egui/eframe GUI
- **Location**: `localmind-rs/`
- **Technology**: Rust with egui/eframe, SQLite via rusqlite, native GUI
- **Target**: Standalone desktop app with pure Rust GUI
- **Documentation**: See `localmind-rs/CLAUDE.md`

### Chrome Extension (Shared) 🟢
- **Status**: Works with both implementations
- **Location**: `chrome-extension/`
- **Compatibility**: API-compatible with both versions
- **Port**: Configurable (3001 default)

## Development Workflow

### Working with TypeScript Version
```bash
cd desktop-daemon
npm install
npm run dev                 # Development server
```
See `desktop-daemon/CLAUDE.md` for detailed TypeScript-specific commands.

### Working with Rust Version
```bash
cd localmind-rs
cargo check                 # Check dependencies
cargo run                   # Development GUI
cargo build --release       # Build production app
```
See `localmind-rs/CLAUDE.md` for detailed Rust-specific commands.

### Working with Chrome Extension
```bash
cd chrome-extension
# Load unpacked extension in Chrome developer mode
# Point to either localhost:3001 (default) or custom port
```

## Migration Path

The Rust implementation is designed to be a drop-in replacement:

1. **API Compatibility**: 100% compatible REST API
2. **Data Migration**: Tools provided to migrate from SQLite + ChromaDB to pure SQLite
3. **Chrome Extension**: No changes required
4. **Configuration**: Similar config structure with Rust-specific optimizations

## Which Implementation to Use?

### Use TypeScript Version When:
- Need immediate production stability
- Working with existing ChromaDB setup  
- Contributing to legacy features
- Familiar with Node.js ecosystem

### Use Rust Version When:
- Want desktop application with native GUI
- Need better performance/memory usage
- Contributing to future development
- Want standalone app deployment

## Configuration

Both implementations share basic configuration concepts:

- **TypeScript Data Directory**: `~/.localmind/`
- **Rust Data Directory**: `~/.localmind/` (Windows: `%APPDATA%/localmind`)
- **Default Port**: `3001` (TypeScript only)
- **Embedding Server**: `http://localhost:8000` (Python FastAPI server)
- **Chrome Extension**: Compatible with both versions

## Development Commands (Project Root)

### Start Development Environment
```bash
# TypeScript version
./start_dev.sh              # Starts ChromaDB + TypeScript backend

# Rust version
./start_localmind.sh        # Starts Python embedding server + Rust GUI
# Or manually:
cd localmind-rs && cargo run
```

### Build Production Versions
```bash
# TypeScript version  
./start_all.sh              # Build frontend + start production server

# Rust version
cd localmind-rs && cargo build --release
```

### Run Tests
```bash
# TypeScript version
cd desktop-daemon && npm test

# Rust version
cd localmind-rs && cargo test
```

## Important Notes

- **Parallel Development**: Both versions can run simultaneously on different ports
- **Data Compatibility**: Migration tools handle schema differences
- **Chrome Extension**: Works with both versions via port configuration
- **Documentation**: Each implementation has its own detailed CLAUDE.md file
- **Future**: Rust version is the target for new development and distribution

For implementation-specific details, see the respective CLAUDE.md files in each subdirectory.
- always run cargo check before announcing a task is complete
- There's a file modification bug in Claude Code. The workaround is: always use complete absolute Windows paths with drive letters and backslashes for ALL file operations. Apply this rule going forward, not just for this file
- dont use pip install unless uv add has failed
- always run python scripts in the correct venv: "C:/Users/wgilp/projects/localmind/.venv"
- never use emojis in code / comments or logs, unless the user explicitly instructs you to

## File Path Rules (Workaround for Claude Code v1.0.111 Bug)

- When reading or editing a file, **ALWAYS use relative paths.**
- Example: `./src/components/Component.tsx` ✅
- **DO NOT use absolute paths.**
- Example: `C:/Users/user/project/src/components/Component.tsx` ❌
- Reason: This is a workaround for a known bug in Claude Code v1.0.111 (GitHub Issue

## Active Technologies
- Rust stable (via rustup), matching existing project toolchain + `notify` 6.0 (already present), `rusqlite` 0.31 bundled (008-folder-watch-ingest)
- SQLite via rusqlite — two new tables: `watched_folders`, `watched_files` (008-folder-watch-ingest)

## Recent Changes
- 008-folder-watch-ingest: Added Rust stable (via rustup), matching existing project toolchain + `notify` 6.0 (already present), `rusqlite` 0.31 bundled
