# Research: Folder Watch and Ingest

**Branch**: `008-folder-watch-ingest` | **Date**: 2026-04-07

## Decision 1: File System Watching Library

**Decision**: Use `notify` 6.0 (`RecommendedWatcher` with `RecursiveMode::Recursive`)

**Rationale**: Already a direct dependency in `Cargo.toml`. The existing `bookmark.rs`
module uses `notify::recommended_watcher()` with a blocking-thread approach and a
manual 500ms debounce. This feature follows the same established pattern — no new
dependency, no new pattern.

**Alternatives considered**:
- `notify-debouncer-mini` / `notify-debouncer-full` subcrates: would simplify debounce
  logic but require new Cargo entries, which need explicit user approval per constitution.
  Manual 500ms debounce (matching bookmark.rs) is sufficient for MVP.
- Polling fallback (`PollWatcher`): already built into `notify` as fallback for network
  drives; `RecommendedWatcher` selects it automatically when native events are unavailable.

## Decision 2: Change Detection Strategy

**Decision**: Track per-file modification timestamp (mtime as Unix seconds) in a
`watched_files` database table. On watcher event, compare stored mtime against
`std::fs::metadata().modified()`. Re-ingest only if mtime has changed.

**Rationale**: Avoids re-ingesting files whose content hasn't changed (e.g., attribute-only
touches). Cheap to compute. Consistent with spec assumption: "Changes are determined by
file modification timestamp and/or file size."

**Alternatives considered**:
- Content hashing (SHA-256): More accurate but adds computation cost per file event and
  requires reading the full file twice. Spec explicitly defers this to post-MVP.
- File size only: Can miss in-place overwrites of equal-length content.

## Decision 3: Supported File Parsing

**Decision**:
- `.txt`, `.md`: `std::fs::read_to_string()` — plain UTF-8 read, no external parser
- `.pdf`: `pdf_extract::extract_text()` — already used in `fetcher.rs`, same function

**Rationale**: All parsers already present in the codebase. Markdown is treated as plain
text (no HTML rendering needed for search/RAG purposes). PDF parsing reuses the exact
call site pattern from `fetcher.rs`.

**Alternatives considered**:
- Pulldown-cmark for Markdown: strips formatting for cleaner text. Requires new
  dependency. Deferred — plain text is sufficient for embedding quality.
- Separate PDF parser: unnecessary, `pdf-extract` already handles the use case.

## Decision 4: Ingestion Pipeline Integration

**Decision**: Call `RagPipeline::ingest_document_with_auth()` with:
- `source = folder_path.to_string_lossy()`
- `needs_auth = false` (local files are always accessible)
- `profile = None` (not a Chrome profile concept)

Store the returned `document_id` in the `watched_files` table for later deletion.

**Rationale**: Reuses the full chunking + embedding + FTS pipeline with zero duplication.
The `source` field is already used for filtering in the UI; using the folder path as
source enables per-folder filtering in future.

**Alternatives considered**:
- Separate ingest function for files: unnecessary abstraction for one call site.

## Decision 5: File Deletion Handling

**Decision**: On `EventKind::Remove`, look up `document_id` from `watched_files` by
path, call a new `db.delete_document(id)` method that removes from `documents`,
`embeddings`, `documents_fts`, and `watched_files`. Also reload `VectorStore` entries
for affected document.

**Rationale**: Clean removal requires touching all four storage locations. The `VectorStore`
is in-memory and must be rebuilt or surgically pruned; a full reload of a folder's
vectors is acceptable for MVP given typical folder sizes.

**Alternatives considered**:
- Soft-delete (mark `is_dead = true`): already used for bookmark URLs that return 404.
  Not appropriate here — user explicitly deleted the file and wants it gone.

## Decision 6: Database Schema

**Decision**: Two new tables, created with `CREATE TABLE IF NOT EXISTS` in
`db.rs::init_schema()`:

```sql
CREATE TABLE IF NOT EXISTS watched_folders (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    path        TEXT UNIQUE NOT NULL,
    status      TEXT NOT NULL DEFAULT 'active',
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS watched_files (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_id    INTEGER NOT NULL REFERENCES watched_folders(id) ON DELETE CASCADE,
    file_path    TEXT UNIQUE NOT NULL,
    modified_at  INTEGER NOT NULL DEFAULT 0,
    document_id  INTEGER,
    ingest_status TEXT NOT NULL DEFAULT 'pending'
);
```

**Rationale**: `ON DELETE CASCADE` on `watched_files` means removing a watched folder
automatically cleans up its file records — no manual cleanup required. `ingest_status`
allows the UI to show per-file progress (pending / ingested / error). `modified_at` is
Unix epoch seconds from `std::time::SystemTime`.

**Alternatives considered**:
- Extending the `documents` table with `folder_id`: would couple the folder-watch concern
  into the generic document schema. Separate table is cleaner and avoids nullable columns
  on every document.

## Decision 7: Watcher Lifecycle

**Decision**: One `notify` watcher instance per watched folder, all managed by a single
`FolderWatchService` struct. Watchers are spawned in a dedicated `std::thread::spawn`
(blocking thread, same as `bookmark.rs`) with a `tokio::sync::mpsc` bridge sending
events to an async handler.

**Rationale**: `notify`'s `RecommendedWatcher` callback is synchronous; bridging to an
async channel is the established pattern in this codebase. One watcher per folder (rather
than one global watcher) simplifies add/remove lifecycle — removing a folder just drops
its watcher.

**Alternatives considered**:
- Single watcher on a common ancestor: brittle if folders have no common parent.
- `tokio-notify` wrapper crate: requires new dependency.

## Decision 8: Duplicate Folder Handling

**Decision**: `UNIQUE` constraint on `watched_folders.path` + check at the service layer
before insert. Return a typed error `FolderWatchError::AlreadyWatched` that the UI
surfaces as a toast notification.

**Alternatives considered**:
- Silent no-op: hides bugs; spec requires explicit feedback (FR-010).

## Resolved Clarifications

All spec assumptions are confirmed by codebase research:
- Recursive scanning: `RecursiveMode::Recursive` in `notify` handles this natively
- 30s change detection target (SC-002): achievable with OS-native events (typically <1s);
  polling fallback interval is configurable, defaulting to 30s in `notify`
- No user-facing limit on watched folders: consistent with simplicity mandate
