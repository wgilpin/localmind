---
description: "Task list for Folder Watch and Ingest"
---

# Tasks: Folder Watch and Ingest

**Input**: Design documents from `/specs/008-folder-watch-ingest/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/channel-messages.md

**Tests**: TDD applies to all backend service methods per constitution (Principle V).
Test tasks are written first within each phase and must FAIL before implementation begins.

**Organization**: Tasks are grouped by user story to enable independent implementation
and testing of each story. All paths are relative to `localmind-rs/`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no shared dependencies)
- **[Story]**: Maps to user story from spec.md (US1, US2, US3)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: New types, module declarations, and DB schema additions needed before any
user story implementation can start.

- [X] T001 Add `watched_folders` and `watched_files` `CREATE TABLE IF NOT EXISTS` blocks to `init_schema()` in `src/db.rs` (SQL from data-model.md)
- [X] T002 [P] Define `WatchedFolder`, `WatchedFile`, `FolderStatus`, `IngestStatus` structs and enums in new file `src/folder_watcher.rs`
- [X] T003 [P] Define `FolderWatchError` using `thiserror` in `src/folder_watcher.rs` (variants: AlreadyWatched, PathNotFound, NotADirectory, DbError, IngestError, IoError)
- [X] T004 [P] Add `FolderWatchEvent` enum and `FolderWatchProgress` struct to `src/gui/state.rs` (from data-model.md Rust types section)
- [X] T005 Register `pub mod folder_watcher` in `src/lib.rs` and `pub mod watched_folders` in `src/gui/widgets/mod.rs`

---

## Phase 2: Foundational (Database CRUD Layer)

**Purpose**: All database methods that US1, US2, and US3 depend on. No user story
can be implemented until these are complete.

**TDD note**: Write each test task first, run `cargo test` to confirm it FAILS, then
implement.

### Tests (write first — must fail before implementation)

- [X] T006 [P] Write unit tests for `Database::add_watched_folder` and `Database::get_watched_folders` using in-memory SQLite in `src/db.rs` `#[cfg(test)]` module
- [X] T007 [P] Write unit tests for `Database::upsert_watched_file`, `Database::get_watched_file_by_path`, `Database::get_watched_files_for_folder` in `src/db.rs` `#[cfg(test)]` module
- [X] T008 [P] Write unit tests for `Database::delete_watched_folder` (confirm CASCADE removes watched_files rows) and `Database::delete_document` (confirm rows removed from documents, embeddings, documents_fts) in `src/db.rs` `#[cfg(test)]` module
- [X] T045 [P] Write unit test for `Database::delete_documents_by_source(folder_path: &Path)` (confirm rows deleted from documents, embeddings, and documents_fts WHERE source = folder path) in `src/db.rs` `#[cfg(test)]` module — needed by T035 before `delete_watched_folder` is called

### Implementation (after tests exist and fail)

- [X] T009 Implement `Database::add_watched_folder(path: &Path) -> Result<i64, DbError>` in `src/db.rs` (INSERT OR FAIL to enforce UNIQUE) — depends on T006
- [X] T010 Implement `Database::get_watched_folders() -> Result<Vec<WatchedFolder>, DbError>` in `src/db.rs`
- [X] T011 [P] Implement `Database::upsert_watched_file(folder_id, file_path, modified_at, document_id, status)` in `src/db.rs` — depends on T007
- [X] T012 [P] Implement `Database::get_watched_file_by_path(path: &Path) -> Result<Option<WatchedFile>, DbError>` in `src/db.rs`
- [X] T013 [P] Implement `Database::get_watched_files_for_folder(folder_id: i64) -> Result<Vec<WatchedFile>, DbError>` in `src/db.rs`
- [X] T014 Implement `Database::delete_watched_folder(id: i64) -> Result<(), DbError>` in `src/db.rs` (DELETE CASCADE handles watched_files) — depends on T008
- [X] T015 [P] Implement `Database::delete_document(document_id: i64) -> Result<(), DbError>` deleting from documents, embeddings, and documents_fts in `src/db.rs` — depends on T008
- [X] T016 [P] Implement `Database::update_watched_folder_status(id: i64, status: &FolderStatus) -> Result<(), DbError>` in `src/db.rs`
- [X] T046 Implement `Database::delete_documents_by_source(folder_path: &Path) -> Result<(), DbError>` in `src/db.rs` (DELETE from documents/embeddings/documents_fts WHERE source = ?) — depends on T045

**Checkpoint**: Run `cargo test` — all Phase 2 tests must pass before proceeding

---

## Phase 3: User Story 1 — Add a Watched Folder (Priority: P1) — MVP

**Goal**: User adds a folder; app scans it, ingests all PDF/MD/TXT files, and makes
them searchable.

**Independent Test**: Add a folder containing known .txt and .md files; confirm files
appear in search results after ingestion completes. No file-watching active yet.

### Tests for US1 (write first — must fail before implementation)

- [X] T017 [P] [US1] Write unit tests for `read_file_content` (returns text for .txt, .md; calls pdf_extract for .pdf; errors on unsupported extension) in `src/folder_watcher.rs` `#[cfg(test)]` module
- [X] T018 [P] [US1] Write unit tests for `FolderWatchService::scan_folder` (discovers .pdf/.md/.txt recursively, ignores other extensions, handles empty folder) in `src/folder_watcher.rs` `#[cfg(test)]` module
- [X] T019 [P] [US1] Write unit tests for `FolderWatchService::add_folder` error cases (AlreadyWatched, PathNotFound, NotADirectory) in `src/folder_watcher.rs` `#[cfg(test)]` module using in-memory SQLite (consistent with T006–T008; error cases abort before ingestion so no RAG mock is needed)

### Implementation for US1

- [X] T020 [US1] Implement `read_file_content(path: &Path) -> Result<String, FolderWatchError>` in `src/folder_watcher.rs` (.txt/.md via `fs::read_to_string`, .pdf via `pdf_extract::extract_text`) — depends on T017
- [X] T021 [US1] Implement `FolderWatchService::scan_folder(folder_id, folder_path, db, rag, event_tx)` performing recursive `WalkDir`-style discovery of supported files, emitting `ScanStarted` then `FileIngested`/`FileError` per file, then `ScanComplete` — depends on T018, T020
- [X] T022 [US1] Implement `FolderWatchService::add_folder(path: &Path)` — validate path, call `db.add_watched_folder`, spawn `tokio::task` running `scan_folder` — depends on T019, T021
- [X] T023 [P] [US1] Add `folder_watch_tx`/`folder_watch_rx` (`UnboundedSender<FolderWatchEvent>`), `add_folder_tx`/`add_folder_rx`, and `remove_folder_tx`/`remove_folder_rx` channel pairs to `LocalMindApp` struct in `src/gui/app.rs`
- [X] T024 [P] [US1] Implement `LocalMindApp::check_folder_watch_events()` polling `folder_watch_rx` each frame and updating a `watched_folders: Vec<WatchedFolder>` field on app state in `src/gui/app.rs`
- [X] T025 [US1] Create `src/gui/widgets/watched_folders.rs` — renders a list of watched folder rows (path, status badge, progress bar during scan)
- [X] T026 [US1] Add "Add Folder" text-input and button to the watched_folders widget; on click validate path is an existing directory then send on `add_folder_tx` in `src/gui/widgets/watched_folders.rs` — depends on T025
- [X] T027 [US1] Integrate watched_folders widget into the settings panel (`src/gui/widgets/settings.rs` or `src/gui/app.rs`) so it is reachable from the UI — depends on T026
- [X] T047 [US1] Add `FolderWatchEvent::AddFolderFailed { folder_path: PathBuf, error: String }` variant to `FolderWatchEvent` enum in `src/gui/state.rs`; update `LocalMindApp::check_folder_watch_events()` to surface the error string inline below the add-folder input in the watched_folders widget
- [X] T048 [US1] Handle `add_folder_rx` in `LocalMindApp::update()` — poll the receiver each frame, forward each `PathBuf` to `FolderWatchService::add_folder`, and on `Err` send `FolderWatchEvent::AddFolderFailed` via `folder_watch_tx` — in `src/gui/app.rs` — depends on T022, T047

**Checkpoint**: At this point US1 is fully functional and independently testable.
Run quickstart.md "Adding a Watched Folder" steps to validate.

---

## Phase 4: User Story 2 — Automatic Re-ingestion on File Change (Priority: P2)

**Goal**: Changes to files in watched folders (create, modify, delete) are detected
automatically and the knowledge base is updated without user action.

**Independent Test**: Modify a file in a watched folder; within 30 seconds search
reflects the updated content.

### Tests for US2 (write first — must fail before implementation)

- [X] T028 [P] [US2] Write unit tests for mtime-based change detection helper (returns `true` when `modified_at` differs from stored value, `false` when same) in `src/folder_watcher.rs` `#[cfg(test)]` module
- [X] T029 [P] [US2] Write unit tests for `FolderWatchService::handle_file_event` dispatch logic (Create → ingest, Modify with changed mtime → re-ingest, Remove → delete) using mock rag and db in `src/folder_watcher.rs` `#[cfg(test)]` module

### Implementation for US2

- [X] T030 [US2] Implement `FolderWatchService::start_watching(folder_id, folder_path)` — creates `notify::RecommendedWatcher` with `RecursiveMode::Recursive` in a `std::thread::spawn`, bridges events to an `UnboundedSender` with 500ms manual debounce (matching `bookmark.rs` pattern) — depends on T028
- [X] T031 [US2] Implement `FolderWatchService::handle_file_event(event)` — on `EventKind::Create`/`Modify` check mtime via `fs::metadata().modified()`, re-ingest if changed and update `watched_files`; on `EventKind::Remove` call `db.delete_document` and reload VectorStore entries for the folder — depends on T029, T030
- [X] T032 [US2] Wire watcher startup into `FolderWatchService::add_folder` so `start_watching` is called after `scan_folder` completes in `src/folder_watcher.rs` — depends on T031
- [X] T033 [US2] After `db.delete_document` in `handle_file_event`, evict the removed document's vectors from the in-memory `VectorStore` (reload affected folder's embeddings from DB) in `src/folder_watcher.rs`

**Checkpoint**: US2 independently testable. Run quickstart.md "Automatic Re-ingestion"
steps (modify file, add file, delete file) to validate.

---

## Phase 5: User Story 3 — Remove a Watched Folder (Priority: P3)

**Goal**: User removes a folder from the watched list; monitoring stops and all
ingested content from that folder is deleted from the knowledge base.

**Independent Test**: Remove a folder, confirm its content is absent from search,
and confirm subsequent file changes in that folder trigger no ingestion.

### Tests for US3 (write first — must fail before implementation)

- [X] T034 [P] [US3] Write unit tests for `FolderWatchService::remove_folder` (stops watcher, deletes DB rows via CASCADE, emits `FolderRemoved`, content no longer searchable) using mock db and rag in `src/folder_watcher.rs` `#[cfg(test)]` module

### Implementation for US3

- [X] T035 [US3] Implement `FolderWatchService::remove_folder(path: &Path)` — drop the folder's watcher handle from an internal `HashMap<PathBuf, WatcherHandle>`, call `db.delete_documents_by_source(path)` to remove documents/embeddings/fts (MUST run before the next step), then `db.delete_watched_folder(id)` (CASCADE removes watched_files), evict folder's vectors from in-memory VectorStore, emit `FolderRemoved` — in `src/folder_watcher.rs` — depends on T034, T046
- [X] T036 [US3] Add "Remove" button to each folder row in watched_folders widget; on click send path on `remove_folder_tx` in `src/gui/widgets/watched_folders.rs`
- [X] T037 [US3] Handle `remove_folder_rx` in `LocalMindApp::update()` — forward path to `FolderWatchService::remove_folder` via the service handle — in `src/gui/app.rs`

**Checkpoint**: All three user stories now independently functional. Run quickstart.md
"Removing a Watched Folder" steps to validate.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Status visibility, error surfacing, startup resilience, and quality gates.

- [X] T038 Surface `FolderStatus` badge on each folder row (green dot for `Active`, amber for `Unavailable`, red for `Error`) in `src/gui/widgets/watched_folders.rs`
- [X] T039 Show collapsible per-folder error list (files with `IngestStatus::Error`) beneath each folder row in `src/gui/widgets/watched_folders.rs`
- [X] T040 On app startup, check all `watched_folders` paths in DB — emit `FolderStatusChanged { status: Unavailable }` for any path that no longer exists; resume watching when path reappears in `src/folder_watcher.rs`
- [X] T041 [P] Run `cargo clippy -- -D warnings` across all modified files and fix all warnings
- [X] T042 [P] Run `cargo fmt` across all modified files (`src/folder_watcher.rs`, `src/db.rs`, `src/gui/app.rs`, `src/gui/state.rs`, `src/gui/widgets/watched_folders.rs`, `src/gui/widgets/mod.rs`, `src/lib.rs`)
- [X] T043 Run `cargo test --all` and confirm all new and existing tests pass
- [ ] T044 Execute quickstart.md validation steps for all three user stories end-to-end

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 completion — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Phase 2 — no dependency on US2 or US3
- **US2 (Phase 4)**: Depends on Phase 2 and Phase 3 (watchers start after initial scan)
- **US3 (Phase 5)**: Depends on Phase 2; integrates with US1 UI; independently testable
- **Polish (Phase 6)**: Depends on all user story phases

### User Story Dependencies

- **US1 (P1)**: Standalone after Foundational — delivers search over folder contents
- **US2 (P2)**: Builds on US1's `add_folder` flow (watcher starts after initial scan)
- **US3 (P3)**: Standalone DB + watcher teardown; integrates with US1's widget

### Within Each User Story

- TDD: test tasks MUST be written first and confirmed FAILING before implementation
- Types before services (`read_file_content` before `scan_folder`)
- Services before GUI (`scan_folder` before the widget calls it)
- Core feature complete before polish tasks

### Parallel Opportunities

- T002, T003, T004 can all run in parallel (different files)
- T006, T007, T008 test tasks can all be written in parallel (different test functions)
- T009–T016 implementation tasks marked [P] can run in parallel after their tests exist
- T017, T018, T019 test tasks can run in parallel
- T023, T024 channel setup is independent of T017–T022 service work
- T041, T042 (clippy + fmt) can run in parallel as final quality pass

---

## Parallel Examples

### Phase 2 (Foundational)

```
# Write all DB test stubs in parallel:
T006: tests for add_watched_folder / get_watched_folders
T007: tests for upsert_watched_file / get_watched_file_by_path / get_watched_files_for_folder
T008: tests for delete_watched_folder (CASCADE) / delete_document

# Then implement in parallel (tests already in place):
T011: upsert_watched_file       T012: get_watched_file_by_path
T013: get_watched_files_for_folder  T015: delete_document
T016: update_watched_folder_status
```

### Phase 3 (US1)

```
# Write test stubs in parallel:
T017: read_file_content tests    T018: scan_folder tests
T019: add_folder error-case tests

# Then implement:
T020: read_file_content (unblocked after T017)
T021: scan_folder (after T018, T020)
T022: add_folder (after T019, T021)

# GUI work can start independently of service work:
T023: channels on app.rs         T024: check_folder_watch_events
```

---

## Implementation Strategy

### MVP First (US1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational DB layer
3. Complete Phase 3: US1 (add folder + scan + ingest + UI)
4. **STOP and VALIDATE**: Test US1 independently per quickstart.md
5. Demo: user can add a folder and search its contents

### Incremental Delivery

1. Setup + Foundational → DB ready
2. US1 complete → folder ingestion works, searchable content
3. US2 complete → live change detection added
4. US3 complete → full lifecycle (add / auto-sync / remove)
5. Polish → production-ready UI and quality gates

---

## Notes

- `[P]` tasks touch different files or are independent functions — safe to parallelize
- TDD: each test task MUST run and FAIL before the corresponding implementation task
- No new Cargo dependencies — `notify`, `pdf-extract`, `tokio`, `rusqlite` all present
- `std::fs::read_dir` + recursion or `walkdir` crate for directory traversal (check if `walkdir` is already a dependency before adding it; if not, use `std::fs::read_dir` recursively per simplicity mandate)
- VectorStore reload after deletion: call existing `RagPipeline` reload path or
  rebuild from DB for the affected folder's embeddings
- Watcher handles stored in `HashMap<PathBuf, JoinHandle<()>>` (or `_watcher: Box<dyn Watcher>`) on `FolderWatchService` to allow per-folder teardown
