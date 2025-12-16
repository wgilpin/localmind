# Data Model: Native egui Desktop GUI

**Feature**: 007-egui-frontend
**Date**: 2025-12-15

## Overview

This feature introduces UI-specific data structures for the egui frontend. The backend data model (documents, embeddings, exclusion rules) remains unchanged. This document defines the GUI state structures.

## UI State Entities

### LocalMindApp (Main Application State)

The root application state holding all UI and backend references.

| Field | Type | Description |
|-------|------|-------------|
| rag | `Arc<RwLock<Option<RagPipeline>>>` | Shared reference to backend RAG pipeline |
| current_view | `View` | Active view (Home, SearchResults, DocumentDetail) |
| search_query | `String` | Current search input text |
| search_results | `Vec<SearchResultView>` | Cached search results |
| all_results | `Vec<SearchResultView>` | All results before filtering |
| similarity_cutoff | `f32` | Current similarity threshold (0.0-1.0) |
| selected_document | `Option<DocumentView>` | Currently viewed document |
| recent_documents | `Vec<DocumentView>` | Recent documents for home screen |
| settings_open | `bool` | Settings modal visibility |
| excluded_folders | `HashSet<String>` | Folder IDs marked for exclusion |
| excluded_domains | `Vec<String>` | Domain patterns for exclusion |
| pending_domain | `String` | Domain input field text |
| bookmark_folders | `Vec<BookmarkFolderView>` | Folder tree for settings |
| toasts | `Vec<Toast>` | Active toast notifications |
| init_status | `InitStatus` | Application initialization state |
| pending_search | `Option<Promise<SearchResults>>` | In-flight search operation |

### View (Enum)

Navigation state for the main content area.

| Variant | Description |
|---------|-------------|
| Home | Default view showing recent documents |
| SearchResults | Search results list after query |
| DocumentDetail | Full document view |

### InitStatus (Enum)

Application initialization progress.

| Variant | Description |
|---------|-------------|
| Starting | Application just launched |
| WaitingForEmbedding | Waiting for Python embedding server |
| Ready | RAG pipeline initialized, search available |
| Error(String) | Initialization failed with message |

### SearchResultView

UI representation of a search result.

| Field | Type | Description |
|-------|------|-------------|
| doc_id | `i64` | Document ID for fetching full content |
| title | `String` | Document title |
| snippet | `String` | Content preview (first ~200 chars) |
| similarity | `f32` | Similarity score (0.0-1.0) |
| url | `Option<String>` | Source URL if available |

### DocumentView

UI representation of a full document.

| Field | Type | Description |
|-------|------|-------------|
| id | `i64` | Document ID |
| title | `String` | Document title |
| content | `String` | Full content (HTML stripped to plain text) |
| url | `Option<String>` | Source URL |
| source | `String` | Source type (e.g., "chrome_bookmark") |
| created_at | `String` | Creation timestamp |

### Toast

Notification message with auto-dismiss.

| Field | Type | Description |
|-------|------|-------------|
| id | `u64` | Unique identifier |
| message | `String` | Notification text |
| toast_type | `ToastType` | Info, Success, or Error |
| created_at | `Instant` | When toast was created |
| duration | `Duration` | Auto-dismiss after (0 = persistent) |

### ToastType (Enum)

Toast visual style.

| Variant | Color | Use Case |
|---------|-------|----------|
| Info | Blue | General information, progress |
| Success | Green | Operation completed successfully |
| Error | Red | Error occurred |

### BookmarkFolderView

UI representation of a bookmark folder for tree display.

| Field | Type | Description |
|-------|------|-------------|
| id | `String` | Chrome folder ID |
| name | `String` | Folder display name |
| path | `Vec<String>` | Full path from root |
| children | `Vec<BookmarkFolderView>` | Nested folders |
| bookmark_count | `usize` | Number of bookmarks in folder |

## State Transitions

### View Transitions

```
                    ┌─────────────────────┐
                    │                     │
         ┌──────────►      Home           │
         │          │  (recent docs)      │
         │          └─────────┬───────────┘
         │                    │
         │              [search query]
         │                    │
         │                    ▼
         │          ┌─────────────────────┐
         │          │                     │
  [back] │          │   SearchResults     │
         │          │   (results list)    │
         │          └─────────┬───────────┘
         │                    │
         │              [click result]
         │                    │
         │                    ▼
         │          ┌─────────────────────┐
         │          │                     │
         └──────────┤   DocumentDetail    │
      [back/Esc]    │   (full content)    │
                    └─────────────────────┘
```

### InitStatus Transitions

```
Starting ──► WaitingForEmbedding ──► Ready
                    │
                    └──► Error(msg)
```

### Toast Lifecycle

```
Created ──► Displayed ──► [duration elapsed] ──► Removed
                │
                └──► [duration=0] ──► Persistent until replaced
```

## Backend Entities (Unchanged)

These entities are defined in the existing backend and remain unchanged:

- **Document** (`db.rs`): Full document record
- **SearchResult** (`rag.rs`): Search hit from vector search
- **BookmarkFolder** (`bookmark.rs`): Chrome bookmark folder
- **ExclusionRules** (`bookmark_exclusion.rs`): Folder/domain exclusions

## Database Changes

### New Query: Recent Documents

Add to `db.rs`:

```rust
/// Get most recently added documents for home screen display
pub async fn get_recent_documents(&self, limit: usize) -> Result<Vec<Document>>
```

**SQL**:
```sql
SELECT id, title, content, url, source, created_at, is_dead
FROM documents
WHERE is_dead = 0 OR is_dead IS NULL
ORDER BY created_at DESC
LIMIT ?
```

No schema changes required.

## Validation Rules

### Search Query
- Empty query: Do not execute search, remain on current view
- Whitespace-only: Treat as empty

### Domain Pattern
- Must not be empty after trimming
- Must be valid per `ExclusionRules::validate_pattern()`
- Duplicates rejected (case-insensitive)

### Similarity Cutoff
- Range: 0.0 to 1.0
- Default: 0.3
- Decrement by 0.1 on "Load More"

## Concurrency Considerations

- `rag` is `Arc<RwLock<Option<RagPipeline>>>` - safe for concurrent access
- GUI runs on main thread; async operations use `poll_promise`
- HTTP server runs in separate tokio task, shares `rag` reference
- Bookmark monitoring runs in separate tokio task, sends progress via channel


