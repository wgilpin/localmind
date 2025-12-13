# Data Model: HTTP REST API Server

**Date**: 2025-01-27  
**Feature**: HTTP REST API Server for Chrome Extension Integration

## Overview

The HTTP API server uses the existing data model from the RAG system. No new database schema changes are required. The server accepts HTTP requests and maps them to existing `Document` entities.

## Entities

### Document

**Source**: Existing entity from `localmind-rs/src/db.rs`

**Fields**:
- `id: i64` - Primary key, auto-increment
- `title: String` - Document title (required)
- `content: String` - Document content (required)
- `url: Option<String>` - Source URL (optional)
- `source: String` - Source identifier (e.g., "chrome_extension", "manual_note", "chrome_bookmark")
- `created_at: String` - ISO 8601 timestamp
- `embedding: Option<Vec<u8>>` - Legacy field (not used in chunk-based system)
- `is_dead: Option<bool>` - Soft delete flag

**Relationships**:
- One-to-many with `chunk_embeddings` table (via `doc_id`)
- Accessed via `Database` struct methods

**Validation Rules**:
- `title` must be non-empty (FR-004)
- `content` must be non-empty (FR-004)
- `url` is optional but validated if present (YouTube URL detection)
- `extractionMethod` stored as metadata in `source` field or separate metadata table (implementation detail)

**State Transitions**:
1. **Created**: Document inserted via `Database::insert_document()`
2. **Chunked**: Document content split into chunks via `DocumentProcessor::chunk_text()`
3. **Embedded**: Each chunk embedded via `EmbeddingClient::generate_embedding()`
4. **Indexed**: Chunks stored in vector store via `VectorStore::add_chunk_vector()`
5. **Searchable**: Document appears in search results via `RAG::search()` or `RAG::get_search_hits_with_cutoff()`

### HTTP Request Payload

**Source**: Incoming JSON from Chrome extension

**Fields**:
- `title: String` - Document title (required)
- `content: String` - Document content (required)
- `url: Option<String>` - Source URL (optional)
- `extractionMethod: Option<String>` - Extraction method metadata (optional, defaults to "dom")

**Validation Rules**:
- `title` must be present and non-empty (FR-004)
- `content` must be present and non-empty (FR-004)
- Request body must not exceed 10MB (FR-004a)
- `extractionMethod` defaults to "dom" if not provided (FR-007a)

**Mapping to Document**:
- `title` → `Document::title`
- `content` → `Document::content`
- `url` → `Document::url`
- `extractionMethod` → stored as metadata (implementation: can append to `source` or use separate field)

### HTTP Response

**Success Response (200 OK)**:
```json
{
  "message": "Document added successfully.",
  "extractionMethod": "dom"
}
```

**Error Responses**:
- `400 Bad Request`: `{ "message": "Title and content are required." }`
- `413 Payload Too Large`: `{ "message": "Request payload exceeds 10MB limit." }`
- `503 Service Unavailable`: `{ "message": "System initializing. Please wait a moment and try again." }`
- `500 Internal Server Error`: `{ "message": "Failed to add document." }`

### RAG State

**Source**: Existing `RagState` type from `localmind-rs/src/main.rs`

**Structure**:
```rust
type RagState = Arc<RwLock<Option<RAG>>>;
```

**Fields** (via `RAG` struct):
- `db: Database` - SQLite database connection
- `vector_store: Mutex<VectorStore>` - In-memory vector store
- `embedding_client: EmbeddingClient` - Ollama or LM Studio client
- `document_processor: DocumentProcessor` - Text chunking utility
- `query_embedding_cache: Mutex<HashMap<String, Vec<f32>>>` - Query result cache

**Access Pattern**:
- Read lock for searches: `rag_state.read().await`
- Write lock for document ingestion: `rag_state.write().await`
- Shared between HTTP server and Tauri GUI (FR-013, FR-014)

## Data Flow

### Document Ingestion Flow

1. **HTTP Request Received**
   - Chrome extension sends POST `/documents` with JSON payload
   - Server validates request size (<10MB)
   - Server validates required fields (title, content)

2. **YouTube URL Processing** (if applicable)
   - Check if `url` contains YouTube domain
   - Attempt transcript fetch (30 second timeout)
   - Clean title (remove bracketed prefixes)
   - Use transcript as content if successful, else use provided content

3. **RAG State Access**
   - Acquire read lock on `RagState`
   - Check if RAG system is initialized
   - If not initialized, return HTTP 503
   - If initialized, proceed to document ingestion

4. **Document Processing**
   - Call `RAG::ingest_document(title, content, url, source)`
   - Document is chunked via `DocumentProcessor`
   - Chunks are embedded via `EmbeddingClient`
   - Chunks are stored in database and vector store

5. **Response**
   - Return HTTP 200 with success message
   - Include `extractionMethod` in response

## Constraints

### Size Limits
- Request body: 10MB maximum (FR-004a)
- Document content: No explicit limit (handled by RAG system)
- Concurrent requests: At least 10 supported (SC-004)

### Concurrency
- Multiple HTTP requests can be processed concurrently
- Database uses semaphores for priority-based access (`OperationPriority`)
- Vector store uses `Mutex` for thread-safe access
- RAG state uses `RwLock` for concurrent reads, exclusive writes

### Performance Targets
- API response time: <5 seconds for documents up to 100KB (SC-005)
- Document searchability: Within 2 seconds of API response (SC-003)
- Server startup: Within 5 seconds of application launch (SC-002)

## Database Schema

No schema changes required. Uses existing tables:
- `documents` - Main document storage
- `chunk_embeddings` - Chunk-level embeddings and metadata
- `excluded_folders` - Bookmark exclusion rules (not used by HTTP API)
- `excluded_domains` - Domain exclusion rules (not used by HTTP API)

## Notes

- `extractionMethod` metadata can be stored by appending to `source` field (e.g., "chrome_extension:dom") or using a separate metadata column if needed
- YouTube transcript fetching reuses existing `YouTubeProcessor` functionality
- All document processing follows the same pipeline regardless of source (HTTP or Tauri IPC)
