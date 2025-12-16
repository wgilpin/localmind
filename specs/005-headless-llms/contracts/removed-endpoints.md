# Removed API Endpoints and Commands

**Date**: 2025-01-27  
**Feature**: Headless LLM Migration

## Overview

This feature removes all chat completion and text generation capabilities. The following endpoints, Tauri commands, and UI elements are completely removed (not just disabled).

## Removed Tauri IPC Commands

### Chat/Generation Commands (REMOVED)

The following Tauri commands are removed from `main.rs`:

- `chat_with_rag` - Chat completion with RAG context
- `generate_response` - Generate text response from query
- `generate_response_stream` - Stream text generation
- `cancel_generation` - Cancel ongoing generation

**Removal Impact**:
- Frontend must remove all UI elements that call these commands
- No replacement endpoints (feature explicitly removed)
- Search functionality remains unchanged

### Remaining Commands (PRESERVED)

The following commands remain functional:
- `search_documents` - Semantic search (unchanged)
- `get_document_count` - Get document count (unchanged)
- `get_document` - Get document by ID (unchanged)
- `add_document` - Add document (unchanged)
- `get_ollama_models` - Get available models (may be removed if Ollama support removed)
- `get_stats` - Get statistics (unchanged)
- `search_hits` - Get search hits (unchanged)
- `ingest_bookmarks` - Ingest bookmarks (unchanged)
- `get_exclusion_rules` - Get exclusion rules (unchanged)
- `set_exclusion_rules` - Set exclusion rules (unchanged)
- `get_bookmark_folders` - Get bookmark folders (unchanged)
- `validate_domain_pattern` - Validate domain pattern (unchanged)

## Removed HTTP Endpoints

If any HTTP endpoints existed for chat/generation (not documented in previous specs), they are removed.

**Note**: The HTTP server (`http_server.rs`) only has `POST /documents` endpoint, which remains unchanged.

## Removed Backend Logic

### RAG Pipeline Methods (REMOVED)

From `rag.rs`:
- `generate_answer()` - Generate answer from query
- `generate_answer_with_cancellation()` - Generate with cancellation token
- `generate_answer_stream()` - Stream answer generation
- `generate_answer_stream_with_cancellation()` - Stream with cancellation
- `chat()` - Chat completion method

**Replacement**: None (features removed)

### Embedding Client Methods (UPDATED)

From `rag.rs` - `EmbeddingClient` enum:
- `generate_completion()` - REMOVED
- `generate_completion_with_cancellation()` - REMOVED
- `generate_completion_stream()` - REMOVED
- `generate_completion_stream_with_cancellation()` - REMOVED
- `generate_embedding()` - PRESERVED (implementation changed to local)

## Removed Data Structures

### Generation State (REMOVED)

From `main.rs`:
```rust
type GenerationState = Arc<RwLock<HashMap<String, CancellationToken>>>;
```

**Removal**: This state management is no longer needed.

### Completion Models (REMOVED)

From `lmstudio.rs` and `ollama.rs`:
- `completion_model: String` - Completion model configuration
- `CompletionRequest` structs
- `CompletionResponse` structs
- `StreamCompletionResponse` structs

## Migration Notes

### For Frontend Developers

1. **Remove UI Elements**:
   - Chat input fields
   - Generation buttons
   - Streaming response displays
   - Cancel generation buttons

2. **Update Error Handling**:
   - Remove error handling for removed commands
   - Commands will not exist, so calls will fail at compile time

3. **Preserve Search UI**:
   - All search functionality remains unchanged
   - Document indexing remains unchanged

### For Backend Developers

1. **Remove Module**:
   - Delete `src/lmstudio.rs` entirely
   - Remove `lmstudio` module export from `lib.rs`

2. **Update RAG Initialization**:
   - Remove `new_with_lmstudio()` method
   - Update `new()` to use `LocalEmbeddingClient` instead

3. **Update Embedding Client**:
   - Replace `EmbeddingClient::LMStudio` variant with `EmbeddingClient::Local`
   - Remove completion-related methods

4. **Clean Up Dependencies**:
   - Remove `reqwest` if no longer used elsewhere
   - Verify all HTTP client usage is removed

## Testing Impact

### Tests to Remove

- Tests for chat completion
- Tests for generation streaming
- Tests for cancellation
- Tests for LMStudio connection

### Tests to Update

- Embedding generation tests (update to use local model)
- RAG initialization tests (remove LMStudio path)
- Integration tests (remove chat/generation scenarios)

### Tests to Preserve

- Document indexing tests
- Search functionality tests
- Vector store tests
- Database tests

## Breaking Changes

**Version**: This is a breaking change (major version bump required)

**Migration Path**: 
- Users upgrading will lose chat/generation features
- No data migration needed (embeddings compatible)
- Clear release notes required explaining feature removal

**Documentation Updates**:
- Update README to remove chat/generation references
- Update user documentation
- Update API documentation (if any)

