# Data Model: Folder Watch and Ingest

**Branch**: `008-folder-watch-ingest` | **Date**: 2026-04-07

## New Entities

### WatchedFolder

Represents a filesystem directory registered by the user for monitoring.

| Field        | Type    | Constraints              | Description                                      |
|--------------|---------|--------------------------|--------------------------------------------------|
| `id`         | INTEGER | PRIMARY KEY AUTOINCREMENT | Internal identifier                             |
| `path`       | TEXT    | UNIQUE NOT NULL           | Absolute filesystem path to the directory       |
| `status`     | TEXT    | NOT NULL DEFAULT 'active' | `active` \| `error` \| `unavailable`           |
| `created_at` | TEXT    | NOT NULL                  | ISO 8601 timestamp of when folder was added     |

**Validation rules**:
- `path` must be an absolute path that exists and is a directory at insert time
- `status` transitions: `active` → `error` on permission/IO failure; `active` →
  `unavailable` when path no longer exists at startup check; back to `active` on recovery

**State transitions**:
```
[added] → active
active  → error        (IO error during scan or watch)
active  → unavailable  (path not found at app startup)
unavailable → active   (path found again on next startup check)
error   → active       (manual re-scan or recovery)
```

---

### WatchedFile

Tracks each individual file discovered within a watched folder, linking it to its
ingested document and storing the last-known modification timestamp.

| Field           | Type    | Constraints                                    | Description                                      |
|-----------------|---------|------------------------------------------------|--------------------------------------------------|
| `id`            | INTEGER | PRIMARY KEY AUTOINCREMENT                       | Internal identifier                             |
| `folder_id`     | INTEGER | NOT NULL REFERENCES watched_folders(id) CASCADE | Parent watched folder                           |
| `file_path`     | TEXT    | UNIQUE NOT NULL                                 | Absolute filesystem path to the file            |
| `modified_at`   | INTEGER | NOT NULL DEFAULT 0                              | Unix epoch seconds of last known mtime          |
| `document_id`   | INTEGER | NULLABLE                                        | FK to `documents.id`; NULL while pending        |
| `ingest_status` | TEXT    | NOT NULL DEFAULT 'pending'                      | `pending` \| `ingested` \| `error`             |

**Validation rules**:
- `file_path` must have a supported extension (`.pdf`, `.md`, `.txt`) — enforced at
  service layer, not DB constraint
- `ON DELETE CASCADE` from `watched_folders` ensures all `watched_files` rows are removed
  when the parent folder is removed
- `document_id` is set to the value returned by `RagPipeline::ingest_document_with_auth()`
  after successful ingestion; remains NULL on error or while pending

**State transitions**:
```
[discovered] → pending
pending      → ingested   (ingest_document_with_auth succeeds, document_id set)
pending      → error      (ingest_document_with_auth fails)
ingested     → pending    (file modification detected, re-ingest queued)
error        → pending    (retry triggered)
[file deleted from disk] → row deleted from watched_files; document deleted from documents
```

---

## Existing Entities (unchanged, referenced for context)

### Document (`documents` table)

No schema changes. The `source` field is set to the absolute folder path string so that
documents from a specific folder can be identified and bulk-deleted when the folder is
removed.

| Relevant Field | Value for folder-watch documents                    |
|----------------|-----------------------------------------------------|
| `source`       | Absolute path of the containing watched folder      |
| `url`          | `file://` URI of the file (e.g. `file:///home/...`) |
| `needs_auth`   | Always `false` for local files                      |
| `profile`      | `NULL` (not a Chrome profile concept)               |
| `is_dead`      | Set to `true` if file is deleted (soft-delete path) |

### Embeddings (`embeddings` table)

No schema changes. Per-chunk embeddings are written by the existing
`RagPipeline::ingest_document_with_auth()` call; deleted via `db.delete_document()`.

---

## Rust Types

```rust
// src/folder_watcher.rs

pub struct WatchedFolder {
    pub id: i64,
    pub path: PathBuf,
    pub status: FolderStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FolderStatus {
    Active,
    Error(String),
    Unavailable,
}

pub struct WatchedFile {
    pub id: i64,
    pub folder_id: i64,
    pub file_path: PathBuf,
    pub modified_at: i64,
    pub document_id: Option<i64>,
    pub ingest_status: IngestStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IngestStatus {
    Pending,
    Ingested,
    Error(String),
}
```

```rust
// src/gui/state.rs (additions)

#[derive(Debug, Clone)]
pub struct FolderWatchProgress {
    pub folder_path: PathBuf,
    pub files_total: usize,
    pub files_done: usize,
    pub current_file: Option<PathBuf>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum FolderWatchEvent {
    ScanStarted { folder_path: PathBuf, files_total: usize },
    FileIngested { folder_path: PathBuf, file_path: PathBuf },
    FileError { folder_path: PathBuf, file_path: PathBuf, error: String },
    ScanComplete { folder_path: PathBuf },
    FolderRemoved { folder_path: PathBuf },
    FolderStatusChanged { folder_path: PathBuf, status: FolderStatus },
}
```

---

## SQL Schema (to be applied in `db.rs::init_schema`)

```sql
CREATE TABLE IF NOT EXISTS watched_folders (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    path        TEXT UNIQUE NOT NULL,
    status      TEXT NOT NULL DEFAULT 'active',
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS watched_files (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_id     INTEGER NOT NULL
                    REFERENCES watched_folders(id) ON DELETE CASCADE,
    file_path     TEXT UNIQUE NOT NULL,
    modified_at   INTEGER NOT NULL DEFAULT 0,
    document_id   INTEGER,
    ingest_status TEXT NOT NULL DEFAULT 'pending'
);
```
