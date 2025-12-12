# Missing Features: Desktop-Daemon Web Frontend Migration

This document outlines features that were available in the desktop-daemon web frontend but are currently missing from the Tauri GUI. These features need to be implemented as Tauri IPC commands and GUI components to provide feature parity.

**Status**: Planned for future implementation  
**Last Updated**: 2025-01-27

## Overview

The desktop-daemon (TypeScript HTTP server) and its web frontend are being deprecated in favor of the Rust application with Tauri GUI. While core functionality (search, AI generation, document viewing, bookmark management) is available in the Tauri GUI, several document management features are missing.

## Missing Features

### 1. Manual Note Creation

**Current State**: Not available in Tauri GUI  
**Desktop-Daemon**: Users could create notes manually via a UI form

**What's Needed**:
- **Tauri IPC Command**: `create_note(title: String, content: String, url: Option<String>) -> Result<String, String>`
  - Creates a new document/note in the RAG system
  - Uses a source identifier like "manual_note" or "user_created"
  - Returns success message or error
- **GUI Component**: Form component in Tauri GUI
  - Input field for note title
  - Textarea for note content
  - Optional URL field
  - Save button that calls the IPC command
  - Success/error feedback

**Implementation Notes**:
- Can reuse existing `add_document` Tauri command or create a dedicated `create_note` command
- Should follow the same document processing pipeline (chunking, embedding)
- Consider adding a "New Note" button or menu item in the main UI

---

### 2. Recent Notes List

**Current State**: Not available in Tauri GUI  
**Desktop-Daemon**: Displayed all recent notes chronologically with pagination and infinite scroll

**What's Needed**:
- **Tauri IPC Command**: `get_recent_notes(limit: i64, offset: i64) -> Result<Vec<Document>, String>`
  - Retrieves documents ordered by creation date (most recent first)
  - Supports pagination with limit and offset
  - Returns list of documents with title, content, url, source, created_at
- **Database Query**: Add method to `Database` struct
  - Query documents ordered by `created_at DESC`
  - Support LIMIT and OFFSET for pagination
- **GUI Component**: List/table component in Tauri GUI
  - Displays recent notes in chronological order
  - Supports pagination or infinite scroll
  - Shows document title, preview, creation date
  - Click to view full document

**Implementation Notes**:
- Consider adding a "Recent Notes" tab or section in the main UI
- May want to show document count and pagination controls
- Could reuse `DocumentView` component for viewing individual notes

---

### 3. Edit Documents

**Current State**: Not available in Tauri GUI  
**Desktop-Daemon**: Users could edit title and content of existing documents

**What's Needed**:
- **Tauri IPC Command**: `update_document(id: i64, title: String, content: String) -> Result<String, String>`
  - Updates document title and content in database
  - May need to re-process document (re-chunk and re-embed) if content changed significantly
  - Returns success message or error
- **Database Method**: Add `update_document` method to `Database` struct
  - Update document record in database
  - Consider whether to update embeddings or mark for re-processing
- **GUI Component**: Edit modal/form component
  - Pre-filled with current document title and content
  - Save button that calls IPC command
  - Cancel button to close without saving
  - Success/error feedback

**Implementation Notes**:
- Editing may require re-embedding if content changes significantly
- Consider adding edit button to document view or search results
- May want to track edit history or last modified timestamp

---

### 4. Delete Documents

**Current State**: Not available in Tauri GUI  
**Desktop-Daemon**: Users could delete documents from the knowledge base

**What's Needed**:
- **Tauri IPC Command**: `delete_document(id: i64) -> Result<String, String>`
  - Deletes document from database
  - Removes associated chunk embeddings from vector store
  - Removes document from vector store
  - Returns success message or error
- **Database Method**: Add `delete_document` method to `Database` struct
  - Delete document record
  - Delete associated chunk embeddings
  - Consider cascade deletion or explicit cleanup
- **Vector Store Method**: Add method to remove document vectors
  - Remove all chunk vectors associated with document ID
- **GUI Component**: Delete button/confirmation dialog
  - Delete button in document view or list
  - Confirmation dialog ("Are you sure you want to delete this document?")
  - Success/error feedback
  - Refresh UI after deletion

**Implementation Notes**:
- Deletion should be comprehensive: document, chunks, embeddings, vector store entries
- Consider soft delete option (mark as deleted but keep data) vs hard delete
- May want to add "Undo" functionality for accidental deletions

---

### 5. Set Completion Model

**Current State**: `get_ollama_models` exists but no way to set the model  
**Desktop-Daemon**: Users could change the LLM completion model via UI

**What's Needed**:
- **Tauri IPC Command**: `set_completion_model(model: String) -> Result<String, String>`
  - Sets the completion model for LLM generation
  - Validates model is available
  - Stores preference in database or configuration
  - Returns success message or error
- **Configuration Storage**: Store model preference
  - Add to database configuration table or config file
  - Load on application startup
- **GUI Component**: Model selector in settings
  - Dropdown/select showing available models (from `get_ollama_models`)
  - Current model highlighted
  - Save button to update model
  - Success/error feedback

**Implementation Notes**:
- May need to update `RagPipeline` or `OllamaClient` to use configured model
- Consider whether this affects embedding model or only completion model
- May want to show model info (size, capabilities) in UI

---

### 6. Analytics Logging

**Current State**: Not available in Tauri GUI  
**Desktop-Daemon**: Logged search result clicks for analytics

**What's Needed**:
- **Tauri IPC Command**: `log_result_click(search_term: String, document_id: i64, distance: f32) -> Result<(), String>`
  - Logs analytics event to file or database
  - Records timestamp, search term, document ID, similarity distance
  - Returns success or error (non-blocking, shouldn't fail main operation)
- **Logging Storage**: File-based or database logging
  - Append to log file (e.g., `click_analytics.log` in app data directory)
  - Or store in database analytics table
  - Format: timestamp, search term, document ID, distance
- **GUI Integration**: Automatic logging on document clicks
  - Log when user clicks on search result
  - Log when user views document from search results
  - Non-blocking, doesn't affect user experience

**Implementation Notes**:
- Analytics logging should be non-intrusive and non-blocking
- Consider privacy implications - may want to make it opt-in
- Log file location should be in app data directory (same as database)
- Consider log rotation or size limits

---

## Implementation Priority

Suggested priority order:

1. **Delete Documents** (P1) - Critical for data management
2. **Edit Documents** (P1) - Important for content correction
3. **Manual Note Creation** (P2) - Useful feature for manual entry
4. **Recent Notes List** (P2) - Helpful for browsing content
5. **Set Completion Model** (P3) - Nice-to-have configuration
6. **Analytics Logging** (P3) - Optional analytics feature

## Technical Considerations

### Database Schema
- May need to add columns for:
  - `updated_at` timestamp for edit tracking
  - `is_deleted` boolean for soft deletes (if implemented)
  - Configuration table for model preferences

### Vector Store Updates
- Editing documents may require re-embedding
- Deletion requires cleanup of vector store entries
- Consider performance implications of bulk operations

### UI/UX Considerations
- Consistent design with existing Tauri GUI components
- Error handling and user feedback
- Loading states for async operations
- Confirmation dialogs for destructive actions

### Testing
- Unit tests for database operations
- Integration tests for IPC commands
- UI tests for user interactions
- Edge cases: concurrent edits, deletion of non-existent documents, etc.

## Related Files

- `localmind-rs/src/main.rs` - Tauri IPC command definitions
- `localmind-rs/src/db.rs` - Database operations
- `localmind-rs/src/rag.rs` - RAG pipeline operations
- `localmind-rs/src-ui/App.svelte` - Main GUI component
- `localmind-rs/src-ui/components/` - GUI component directory

## References

- Desktop-daemon implementation: `desktop-daemon/src/index.ts`
- Desktop-daemon frontend: `desktop-daemon/frontend/src/lib/components/`
- Existing Tauri commands: See `localmind-rs/src/main.rs` for `invoke_handler` list
