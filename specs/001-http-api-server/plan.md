# Implementation Plan: HTTP REST API Server for Chrome Extension Integration

**Branch**: `001-http-api-server` | **Date**: 2025-01-27 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-http-api-server/spec.md`

## Summary

Add an HTTP REST API server to the Rust LocalMind application that runs alongside the Tauri GUI, exposing a single `POST /documents` endpoint on localhost:3000 (or alternative ports) for Chrome extension integration. The server must share the same RAG state and database as the Tauri GUI, handle CORS for browser extensions, and start automatically on application launch. This enables the Chrome extension to work with the Rust backend without requiring the separate TypeScript desktop-daemon server.

**Technical Approach**: Use axum HTTP framework with tower-http CORS middleware, integrated into existing Tokio runtime within the same executable/process as the Tauri GUI. Server starts in Tauri setup function after RAG initialization, runs on separate Tokio task (same process, different async task). Reuses existing YouTube transcript fetching and document processing pipeline. **Important**: This is NOT a separate executable - the HTTP server is embedded within the LocalMind application process, sharing the same memory space, RAG state, and database connection as the Tauri GUI.

## Technical Context

**Language/Version**: Rust 1.75+ (edition 2021)  
**Primary Dependencies**: 
- Tauri 1.5+ (existing)
- Tokio 1.x with full features (existing async runtime)
- axum 0.7+ (HTTP server framework - see research.md)
- tower 0.4+ (middleware utilities)
- tower-http 0.5+ with cors feature (CORS middleware)
- reqwest 0.11 (existing HTTP client for YouTube transcripts)

**Storage**: SQLite via rusqlite (bundled, existing database shared with Tauri GUI)  
**Testing**: cargo test (Rust standard testing framework)  
**Target Platform**: Windows, macOS, Linux (cross-platform via Tauri)  
**Project Type**: Single project (Tauri desktop application with embedded HTTP server in same process)  
**Performance Goals**: 
- HTTP server startup within 5 seconds of application launch (SC-002)
- API response time <5 seconds for documents up to 100KB (SC-005)
- Handle at least 10 concurrent requests without errors (SC-004)
- Documents searchable in Tauri GUI within 2 seconds of API response (SC-003)

**Constraints**: 
- Must run alongside Tauri GUI without interference (FR-012)
- Must share same RAG state and database instance (FR-013, FR-014)
- Localhost-only access (no authentication needed per Out of Scope)
- Request payload limit: 10MB (FR-004a)
- Port range: 3000-3010 with automatic fallback (FR-020, FR-020b)
- YouTube transcript timeout: 30 seconds (FR-022a)

**Scale/Scope**: 
- Single endpoint: POST /documents
- Localhost-only server (no external access)
- Chrome extension integration (single client type)
- Shared state with existing Tauri GUI application

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### I. Privacy & Offline-First Architecture
✅ **PASS**: HTTP server listens only on localhost, no external network access. All data processing remains local. No telemetry or external data transmission.

### II. Performance & Native Experience
✅ **PASS**: Uses existing Tokio async runtime. HTTP server runs in background without blocking Tauri GUI. Performance targets align with constitution (<100ms search latency not applicable here, but <5s API response acceptable for document ingestion).

### III. Modern UI/UX Excellence
✅ **PASS**: HTTP API is backend-only, no UI changes required. Error messages follow user-friendly format per spec.

### IV. Intelligent Automation with User Control
✅ **PASS**: HTTP server starts automatically but only serves localhost. No background data collection. User controls what documents are sent via Chrome extension.

### V. Developer Quality & Maintainability
⚠️ **NEEDS REVIEW**: 
- New HTTP server dependency must be justified (will be addressed in research.md)
- Code must pass cargo clippy and cargo fmt
- HTTP server module must have doc comments
- Must integrate cleanly with existing module structure

**Gate Status**: ✅ **PASS** 

**Post-Research Update**: HTTP server dependency (axum) justified in research.md. Axum chosen for excellent Tokio integration, clean API, and built-in CORS support. Minimal additional dependencies (tower, tower-http) align with simplicity mandate.

## Project Structure

### Documentation (this feature)

```text
specs/001-http-api-server/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
localmind-rs/
├── src/
│   ├── main.rs          # Tauri entry point, HTTP server startup
│   ├── http_server.rs   # NEW: HTTP server module
│   ├── db.rs            # Existing: Database operations
│   ├── rag.rs           # Existing: RAG pipeline
│   ├── youtube.rs       # Existing: YouTube transcript fetching
│   └── ...
├── src-ui/              # Existing: Svelte frontend
└── Cargo.toml           # Add HTTP server dependency
```

**Structure Decision**: Single project structure maintained. New `http_server.rs` module added to `src/` directory. HTTP server integrates into existing `main.rs` Tauri setup function. No new directories needed. **Architecture**: The HTTP server runs as a Tokio async task within the same process as the Tauri GUI application. When the user launches `localmind-rs` executable, both the Tauri GUI window and the HTTP server start automatically. They share the same `RagState` (Arc<RwLock<Option<RAG>>>) and database connection, ensuring data consistency without inter-process communication.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| New HTTP server dependency | Need async HTTP server that integrates with Tokio runtime | Using raw TCP sockets would require implementing HTTP protocol, CORS, JSON parsing manually - violates simplicity mandate |
