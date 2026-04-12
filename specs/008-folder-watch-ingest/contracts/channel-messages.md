# Internal Contracts: Channel Messages

**Branch**: `008-folder-watch-ingest` | **Date**: 2026-04-07

This feature is a desktop GUI application with no new HTTP/REST surface. The contract
boundary is the channel interface between the `FolderWatchService` background task and
the egui update loop, following the established pattern in `gui/app.rs`.

---

## Channel: `folder_watch_tx` / `folder_watch_rx`

**Type**: `tokio::sync::mpsc::UnboundedSender<FolderWatchEvent>` (backend → GUI)

**Direction**: `FolderWatchService` → `LocalMindApp::update()`

**Purpose**: Streams scan progress, file ingestion results, and status changes so the
egui loop can update its display without blocking.

### Message Variants

#### `FolderWatchEvent::ScanStarted`

Sent once at the beginning of an initial folder scan.

```rust
FolderWatchEvent::ScanStarted {
    folder_path: PathBuf,  // absolute path of the folder being scanned
    files_total: usize,    // total count of supported files found
}
```

**GUI response**: Show progress bar for this folder; set files_total as denominator.

---

#### `FolderWatchEvent::FileIngested`

Sent after each file is successfully ingested.

```rust
FolderWatchEvent::FileIngested {
    folder_path: PathBuf,  // parent folder
    file_path: PathBuf,    // file that was ingested
}
```

**GUI response**: Increment progress counter; update `WatchedFile` status to `Ingested`.

---

#### `FolderWatchEvent::FileError`

Sent when a file fails to ingest (parse error, permission denied, etc.).

```rust
FolderWatchEvent::FileError {
    folder_path: PathBuf,
    file_path: PathBuf,
    error: String,         // human-readable error description
}
```

**GUI response**: Increment counter; mark file as error; surface in per-folder error list.

---

#### `FolderWatchEvent::ScanComplete`

Sent once when the initial scan for a folder finishes (all files attempted).

```rust
FolderWatchEvent::ScanComplete {
    folder_path: PathBuf,
}
```

**GUI response**: Hide progress bar; show summary (N ingested, M errors).

---

#### `FolderWatchEvent::FolderRemoved`

Sent after a folder's watcher is torn down and its DB records are deleted.

```rust
FolderWatchEvent::FolderRemoved {
    folder_path: PathBuf,
}
```

**GUI response**: Remove folder from watched-folders list in UI state.

---

#### `FolderWatchEvent::FolderStatusChanged`

Sent when a folder's `FolderStatus` changes (e.g., path becomes unavailable).

```rust
FolderWatchEvent::FolderStatusChanged {
    folder_path: PathBuf,
    status: FolderStatus,  // Active | Error(String) | Unavailable
}
```

**GUI response**: Update status badge on the folder row.

---

#### `FolderWatchEvent::AddFolderFailed`

Sent when `FolderWatchService::add_folder` fails at the service layer (e.g., `AlreadyWatched`,
`NotADirectory`, `DbError`). Distinct from `FileError` (which is per-file) and
`FolderStatusChanged` (which is for folders already in the list). The folder is never
added to the watched list when this event fires.

```rust
FolderWatchEvent::AddFolderFailed {
    folder_path: PathBuf,  // path the user attempted to add
    error: String,         // human-readable reason (e.g., "This folder is already being watched")
}
```

**GUI response**: Surface error string inline below the add-folder input field; do not add
the folder row to the watched-folders list.

---

## Channel: `add_folder_tx` / `add_folder_rx`

**Type**: `tokio::sync::mpsc::UnboundedSender<PathBuf>` (GUI → backend)

**Direction**: `LocalMindApp::update()` → `FolderWatchService`

**Purpose**: Signals the service to add a new folder (after UI validation that path
exists and is a directory).

---

## Channel: `remove_folder_tx` / `remove_folder_rx`

**Type**: `tokio::sync::mpsc::UnboundedSender<PathBuf>` (GUI → backend)

**Direction**: `LocalMindApp::update()` → `FolderWatchService`

**Purpose**: Signals the service to stop watching a folder and remove its documents.

---

## Error Handling Contract

- All errors from `FolderWatchService` are communicated via `FolderWatchEvent::FileError`
  or `FolderWatchEvent::FolderStatusChanged` — never via panics or unwraps.
- The service uses `thiserror`-derived `FolderWatchError` internally; errors are
  converted to `String` before being sent over the channel (avoids Send constraints on
  complex error types).
- Every error is logged with `tracing::error!` before being sent to the GUI.
