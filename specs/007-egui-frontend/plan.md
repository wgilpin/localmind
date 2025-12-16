# Implementation Plan: Native egui Desktop GUI

**Branch**: `007-egui-frontend` | **Date**: 2025-12-15 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/007-egui-frontend/spec.md`

## Summary

Replace the Svelte + Tauri frontend with a native Rust GUI using egui/eframe, eliminating all JavaScript/Node.js dependencies. The existing Rust backend (RAG pipeline, database, embedding client, HTTP server) remains unchanged. The egui GUI runs in the same process, replacing Tauri IPC with direct function calls. Document content is displayed as plain text with HTML tags stripped. The home screen shows recent documents before any search is performed.

## Technical Context

**Language/Version**: Rust 1.75+
**Primary Dependencies**: egui 0.29+, eframe 0.29+, open (for browser launching), html2text (for HTML stripping)
**Storage**: SQLite via rusqlite (existing, unchanged)
**Testing**: cargo test (unit tests for UI state logic)
**Target Platform**: Windows, macOS, Linux (desktop)
**Project Type**: Single desktop application
**Performance Goals**: <100ms search UI response, <3s application launch, <50MB binary
**Constraints**: <200MB memory idle, single binary output, no external assets
**Scale/Scope**: Single user local application

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Privacy & Offline-First | PASS | No change to data handling; egui runs locally |
| II. Performance & Native Experience | PASS | egui meets performance targets; single executable maintained |
| II. Performance & Native Experience | REQUIRES UPDATE | Constitution specifies "Native window decorations via Tauri" - will need amendment to reflect egui/eframe |
| III. Modern UI/UX Excellence | REQUIRES UPDATE | Constitution specifies Svelte 5 runes - will need amendment post-migration |
| IV. Intelligent Automation | PASS | No change to automation behavior |
| V. Developer Quality | PASS | Rust quality gates (clippy, fmt, tests) remain |
| VI. Python Development Standards | N/A | No Python changes in this feature |
| Simplicity Mandate | PASS | Reduces complexity by removing JS/Node.js layer |
| Technology Stack | REQUIRES UPDATE | Fixed stack specifies Svelte/Tauri - will need amendment |

**Gate Result**: PASS with noted constitution updates required post-implementation.

**Constitution Amendment Note**: After successful migration, the following constitution sections must be updated to reflect egui/eframe replacing Svelte 5/Tauri:
- Section II (Performance & Native Experience): Update "Native window decorations and OS integration via Tauri" to reflect eframe
- Section III (Modern UI/UX Excellence): Remove Svelte 5 runes requirement, add egui-specific UI guidelines
- Technology Stack: Update "Frontend: Svelte 5+ with Vite" to "Frontend: egui/eframe (pure Rust)"

## Project Structure

### Documentation (this feature)

```text
specs/007-egui-frontend/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (N/A - no new APIs)
└── tasks.md             # Phase 2 output
```

### Source Code (repository root)

```text
localmind-rs/
├── Cargo.toml           # UPDATE: Add egui, eframe, open, html2text; Remove tauri
├── build.rs             # UPDATE: Remove tauri-build or convert to eframe
├── src/
│   ├── main.rs          # REPLACE: egui/eframe app instead of Tauri
│   ├── gui/             # NEW: egui UI modules
│   │   ├── mod.rs       # GUI module root
│   │   ├── app.rs       # Main eframe App implementation
│   │   ├── views/       # View components
│   │   │   ├── mod.rs
│   │   │   ├── search.rs      # Search bar + results list
│   │   │   ├── document.rs    # Document detail view
│   │   │   └── home.rs        # Home screen with recent docs
│   │   ├── widgets/     # Reusable widgets
│   │   │   ├── mod.rs
│   │   │   ├── toast.rs       # Toast notification system
│   │   │   ├── settings.rs    # Settings modal
│   │   │   └── folder_tree.rs # Bookmark folder tree
│   │   └── state.rs     # Application state management
│   ├── lib.rs           # UNCHANGED: Backend library exports
│   ├── db.rs            # UNCHANGED
│   ├── rag.rs           # UNCHANGED
│   ├── bookmark.rs      # UNCHANGED
│   ├── http_server.rs   # UPDATE: Remove Tauri state types, use Arc<RwLock<RAG>> directly
│   └── [other backend modules unchanged]
├── src-ui/              # DELETE: All Svelte/Vite files
├── package.json         # DELETE
├── package-lock.json    # DELETE
├── vite.config.js       # DELETE
├── svelte.config.js     # DELETE
├── tauri.conf.json      # DELETE
└── icons/               # KEEP: App icons for eframe

tests/
└── gui/                 # NEW: GUI state tests
    └── state_tests.rs
```

**Structure Decision**: Extend existing `localmind-rs/src/` with new `gui/` module. Backend modules remain unchanged. All frontend JS/Svelte files deleted.

## Complexity Tracking

> No constitution violations requiring justification. Migration reduces complexity.

| Change | Justification | Alternative Considered |
|--------|---------------|------------------------|
| Add gui/ module hierarchy | Separates UI concerns from backend | Inline in main.rs - rejected for maintainability |
| Use html2text crate | Clean HTML→text conversion | Manual regex - rejected as error-prone |

## Phase 0: Research Summary

### Key Decisions

1. **egui/eframe version**: Use 0.29+ (latest stable) for best async support
2. **HTML stripping**: Use `html2text` crate (lightweight, well-maintained)
3. **Browser opening**: Use `open` crate (cross-platform, minimal)
4. **State management**: Single `AppState` struct with direct RAG access
5. **Async integration**: Use `tokio` runtime with eframe's `poll_promise` pattern

### Dependencies to Add

```toml
[dependencies]
eframe = "0.29"
egui = "0.29"
open = "5"
html2text = "0.12"

# egui extras for additional widgets
egui_extras = { version = "0.29", features = ["all_loaders"] }
```

### Dependencies to Remove

```toml
# Remove these
tauri = "..."
tauri-build = "..."
```

### Files to Delete

- `localmind-rs/src-ui/` (entire directory)
- `localmind-rs/package.json`
- `localmind-rs/package-lock.json`
- `localmind-rs/vite.config.js`
- `localmind-rs/svelte.config.js`
- `localmind-rs/tauri.conf.json`
- `localmind-rs/build.rs` (or rewrite for non-Tauri build)

## Phase 1: Design

### Application State Model

```rust
pub struct LocalMindApp {
    // Core backend (shared with HTTP server)
    rag: Arc<RwLock<Option<RagPipeline>>>,
    
    // UI State
    current_view: View,
    search_query: String,
    search_results: Vec<SearchResult>,
    selected_document: Option<Document>,
    recent_documents: Vec<Document>,
    
    // Settings
    settings_open: bool,
    excluded_folders: Vec<String>,
    excluded_domains: Vec<String>,
    bookmark_folders: Vec<BookmarkFolder>,
    domain_input: String,
    
    // Toasts
    toasts: Vec<Toast>,
    
    // Initialization
    init_status: InitStatus,
    
    // Async operations
    pending_search: Option<Promise<SearchResults>>,
}

pub enum View {
    Home,
    SearchResults,
    DocumentDetail,
}

pub enum InitStatus {
    Starting,
    WaitingForEmbedding,
    Ready,
    Error(String),
}
```

### UI Layout (egui panels)

```
┌────────────────────────────────────────────────────────────┐
│  [Search Box]                                    [⚙ Settings] │
├────────────────────────────────────────────────────────────┤
│                                                              │
│  [View::Home]        Recent Documents:                       │
│                      • Doc 1 - 2 hours ago                  │
│                      • Doc 2 - 1 day ago                    │
│                      • ...                                   │
│                                                              │
│  [View::SearchResults]  Results for "query":                │
│                         • Title (0.89)                      │
│                         • Title (0.76)                      │
│                         • [Load More]                       │
│                                                              │
│  [View::DocumentDetail] [← Back]                            │
│                         Title                               │
│                         URL (clickable)                     │
│                         ─────────────                       │
│                         Content (scrollable)                │
│                                                              │
├────────────────────────────────────────────────────────────┤
│  [Toast area - bottom right, overlaid]                      │
└────────────────────────────────────────────────────────────┘
```

### Key Integration Points

1. **RAG Pipeline Access**: Direct `Arc<RwLock<Option<RagPipeline>>>` shared between GUI and HTTP server
2. **HTTP Server**: Spawned as tokio task, shares RAG state
3. **Bookmark Monitoring**: Spawned as tokio task, sends progress via channel to GUI
4. **Search Execution**: Async via `poll_promise` pattern in egui

### HTML Stripping Strategy

```rust
use html2text::from_read;

fn strip_html(content: &str) -> String {
    from_read(content.as_bytes(), 80)
        .unwrap_or_else(|_| content.to_string())
}
```

### Recent Documents Query

Add to `db.rs`:
```rust
pub async fn get_recent_documents(&self, limit: usize) -> Result<Vec<Document>> {
    // SELECT * FROM documents ORDER BY created_at DESC LIMIT ?
}
```

## Quickstart

See [quickstart.md](./quickstart.md) for developer setup instructions.

## Contracts

No new external APIs are introduced. The existing HTTP API on port 3000-3010 remains unchanged for Chrome extension compatibility. Internal function calls replace Tauri IPC.

## Next Steps

Run `/speckit.tasks` to generate the implementation task breakdown.
