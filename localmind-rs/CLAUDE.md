# CLAUDE.md - LocalMind Rust Implementation

This file provides guidance for working with the Rust implementation of LocalMind.

## Overview

The Rust implementation (`localmind-rs/`) is the current active development version of LocalMind. It's a desktop application built with Tauri that provides a native GUI for LocalMind's RAG-based knowledge management system. This version aims to be a standalone, performant replacement for the Node.js implementation.

## Technology Stack

- **Backend**: Rust with Tokio async runtime
- **Database**: SQLite via `rusqlite` (bundled, no external dependencies)
- **GUI Framework**: Tauri (web-based UI in native app)
- **Frontend**: Vanilla HTML/CSS/JavaScript (in `src-ui/`)
- **Vector Operations**: Custom implementation for embeddings
- **HTTP Client**: `reqwest` for Ollama integration

## Project Structure

```text
localmind-rs/
├── Cargo.toml              # Rust dependencies and metadata
├── build.rs                # Tauri build script
├── tauri.conf.json         # Tauri configuration
├── icons/                  # Application icons
├── src/                    # Rust source code
│   ├── main.rs            # Application entry point
│   ├── lib.rs             # Library exports
│   ├── db.rs              # Database operations
│   ├── document.rs        # Document model and operations
│   ├── ollama.rs          # Ollama API integration
│   ├── rag.rs             # RAG implementation
│   └── vector.rs          # Vector operations and similarity
├── src-ui/                # Frontend UI (served by Tauri)
│   ├── index.html         # Main UI layout
│   ├── app.js             # Frontend JavaScript
│   └── style.css          # UI styling
└── target/                # Build artifacts (gitignored)
```

## Development Commands

### Setup and Dependencies

```bash
# Install Rust dependencies
cargo check

# Update dependencies
cargo update
```

### Development

```bash
# Run in development mode with GUI
cargo tauri dev

# Build for development (CLI mode if needed)
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Check for issues
cargo clippy
```

### Production Build

```bash
# Build optimized binary
cargo build --release

# Build Tauri app bundle
cargo tauri build
```

## Key Components

### Core Modules

1. **main.rs**: Application entry point and Tauri setup
2. **db.rs**: SQLite database operations and schema management
3. **document.rs**: Document model, CRUD operations, and metadata handling
4. **ollama.rs**: Integration with Ollama API for embeddings and chat
5. **rag.rs**: RAG implementation with vector similarity search
6. **vector.rs**: Vector operations, similarity calculations, and embedding storage

### Frontend (src-ui/)

- **index.html**: Single-page application layout
- **app.js**: Frontend logic, Tauri IPC communication
- **style.css**: UI styling and responsive design

## Database Schema

The Rust implementation uses a simplified SQLite schema optimized for performance:

- **documents**: Core document storage with metadata
- **embeddings**: Vector embeddings stored as blobs
- **chunks**: Text chunks for RAG processing

## Tauri Integration

### IPC Commands

The Rust backend exposes commands to the frontend via Tauri's IPC system:

- Document management (add, update, delete, search)
- RAG queries and responses
- Configuration management
- Vector similarity search

### Configuration

- **tauri.conf.json**: Tauri-specific settings
- Window size, permissions, CSP policies
- Bundle configuration for distribution

## Development Workflow

### Local Development

```bash
# Start development server with hot reload
cargo tauri dev

# Frontend changes are automatically reloaded
# Backend changes require restart
```

### Testing Strategy

```bash
# Unit tests for core modules
cargo test

# Integration tests for database operations
cargo test --test integration

# Vector similarity tests
cargo test vector:: --lib
```

### Code Quality

```bash
# Format all code
cargo fmt

# Check for common issues
cargo clippy

# Full check including unused dependencies
cargo +nightly udeps
```

## Performance Considerations

### Optimizations

- SQLite with bundled build (no external dependencies)
- Custom vector operations for embedding similarity
- Async/await for non-blocking operations
- Efficient memory management with Rust's ownership system

### Memory Usage

- Vectors stored as compressed blobs in SQLite
- Streaming responses for large result sets
- Minimal frontend JavaScript footprint

## Integration with Existing System

### API Compatibility

The Rust implementation is designed as a desktop-first application but maintains conceptual compatibility with the TypeScript version:

- Similar data models and operations
- Compatible configuration concepts
- Shared Ollama integration approach

### Data Migration

Migration utilities are planned for:

- SQLite database conversion
- ChromaDB to SQLite vector migration
- Configuration file compatibility

## Configuration

### Default Settings

- Database: `~/.localmind/localmind.db` (Windows: `%APPDATA%/localmind/localmind.db`)
- Ollama URL: `http://localhost:11434`
- Default model: `qwen3-embedding:0.6b`
- Window size: 1200x800

### Environment Variables

- `RUST_LOG`: Logging level (debug, info, warn, error)
- `LOCALMIND_DB_PATH`: Custom database path
- `OLLAMA_HOST`: Custom Ollama server URL

## Troubleshooting

### Common Issues

1. **Tauri build fails**:

   ```bash
   # Ensure system dependencies are installed
   # On Windows: Visual Studio Build Tools
   # On macOS: Xcode Command Line Tools
   # On Linux: build-essential, webkit2gtk-4.0-dev
   ```

2. **Database connection errors**:

   ```bash
   # Check database file permissions
   # Ensure SQLite bundled feature is enabled
   ```

3. **Ollama integration issues**:

   ```bash
   # Verify Ollama is running on localhost:11434
   # Check model availability with `ollama list`
   ```

### Debugging

```bash
# Enable debug logging
RUST_LOG=debug cargo tauri dev

# Database debugging
RUST_LOG=rusqlite=debug cargo run

# Network debugging (Ollama requests)
RUST_LOG=reqwest=debug cargo run
```

## Build Targets

### Development

- `cargo build`: Fast compilation for testing
- `cargo tauri dev`: GUI development with hot reload

### Release

- `cargo build --release`: Optimized binary
- `cargo tauri build`: Packaged application for distribution

### Cross-compilation

```bash
# Build for different targets (requires setup)
cargo build --target x86_64-pc-windows-gnu
cargo build --target x86_64-apple-darwin
cargo build --target x86_64-unknown-linux-gnu
```

## Future Enhancements

### Planned Features

- Embedded web server for API compatibility
- Advanced vector search algorithms
- Plugin system for custom processors
- Multi-database support

### Performance Goals

- Sub-100ms search responses
- <50MB memory footprint
- Single executable deployment
- Cross-platform compatibility

## Contributing

### Code Style

- Use `cargo fmt` for formatting
- Follow Rust naming conventions
- Add documentation comments for public APIs
- Write tests for new functionality

### Commit Guidelines

- Follow conventional commits format
- Test changes before committing
- Update documentation for API changes

This Rust implementation is the current active development version of LocalMind, focusing on performance, security, and ease of deployment as a standalone desktop application.
