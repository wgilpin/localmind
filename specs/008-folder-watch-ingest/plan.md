# Implementation Plan: Folder Watch and Ingest

**Branch**: `008-folder-watch-ingest` | **Date**: 2026-04-07 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/008-folder-watch-ingest/spec.md`

## Summary

Users can register local filesystem directories to be monitored by LocalMind. On
registration the application performs an initial scan, ingests all PDF, Markdown, and
plain-text files found recursively, and stores change-tracking metadata. A background
file-system watcher detects additions, modifications, and deletions and re-ingests or
removes the affected content automatically. The UI surfaces watched folders, per-folder
status, and ingestion progress. No new Cargo dependencies are required — `notify` 6.0,
`pdf-extract`, and the existing RAG pipeline already provide all needed capabilities.

## Technical Context

**Language/Version**: Rust stable (via rustup), matching existing project toolchain
**Primary Dependencies**: `notify` 6.0 (already present), `rusqlite` 0.31 bundled
(already present), `pdf-extract` (already present via `fetcher.rs`), `tokio` 1.0
(already present)
**Storage**: SQLite via rusqlite — two new tables: `watched_folders`, `watched_files`
**Testing**: `cargo test` — TDD for all backend service methods; egui components not
unit tested per constitution
**Target Platform**: Windows, macOS, Linux (matching existing eframe/egui app)
**Project Type**: Single Rust project under `localmind-rs/`
**Performance Goals**: Initial scan of 100 files completes within 60s (spec SC-001);
change detection and re-ingestion within 30s of file save (spec SC-002); egui update
loop never blocked by watcher or I/O
**Constraints**: All ingestion runs at `OperationPriority::BackgroundIngest` (existing
semaphore pattern) to yield to user searches; no new Cargo.toml dependencies

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle                              | Status | Notes                                                                                                                                                           |
|----------------------------------------|--------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------|
| I — Privacy & Offline-First            | PASS   | All file access is local; no external network calls introduced                                                                                                  |
| II — Performance & Native Experience   | PASS   | Watcher runs in background tokio task; existing `BackgroundIngest` semaphore prevents search latency impact                                                     |
| III — UI/UX Excellence                 | PASS   | Ingestion progress shown in UI; error states surfaced per-folder; loading indicator during initial scan                                                         |
| IV — Intelligent Automation            | PASS   | Folder watching is fully opt-in (user explicitly adds each folder); folders can be removed at any time                                                          |
| V — Developer Quality                  | PASS   | TDD for all backend methods; `thiserror` for error types; `tracing` for logging; no `unwrap()` in service code                                                  |
| VI — Python Standards                  | N/A    | Rust-only feature                                                                                                                                               |
| VII — Observability & Logging          | PASS   | `tracing` used for all log statements; every error logged before conversion                                                                                     |
| VIII — LLM / AI Integration            | N/A    | No LLM calls introduced by this feature                                                                                                                         |
| Simplicity Mandate                     | PASS   | No new dependencies; reuses existing `RagPipeline::ingest_document_with_auth`, `notify` watcher pattern from `bookmark.rs`, and `pdf-extract` from `fetcher.rs` |
| Explicit Approval Gates                | NOTE   | Two new tables added via `CREATE TABLE IF NOT EXISTS` (additive, non-breaking) — same pattern used throughout `db.rs`; no destructive migrations                |

**Post-design re-check**: Confirmed after Phase 1 — design stays within existing patterns,
no violations introduced.

## Project Structure

### Documentation (this feature)

```text
specs/008-folder-watch-ingest/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (channel message types)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code

```text
localmind-rs/
├── Cargo.toml                          (no changes — all deps already present)
└── src/
    ├── folder_watcher.rs               (NEW — FolderWatchService backend)
    ├── lib.rs                          (add pub mod folder_watcher)
    ├── db.rs                           (add watched_folders/watched_files tables + CRUD)
    ├── gui/
    │   ├── app.rs                      (add folder watch channels + initialization)
    │   ├── state.rs                    (add WatchedFolderStatus, FolderWatchProgress types)
    │   └── widgets/
    │       └── watched_folders.rs      (NEW — list + add/remove UI widget)
    └── [all other files unchanged]

tests within localmind-rs/src/folder_watcher.rs (unit tests, inline per Rust convention)
```

**Structure Decision**: Single Rust project (`localmind-rs/`). New backend logic in its
own module `folder_watcher.rs` per the constitution's module-per-concern rule. GUI
additions are confined to `app.rs`, `state.rs`, and a new `watched_folders.rs` widget
that follows the existing widget pattern.

## Complexity Tracking

> No constitution violations requiring justification.
