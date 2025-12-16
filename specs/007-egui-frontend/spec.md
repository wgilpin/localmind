# Feature Specification: Native egui Desktop GUI

**Feature Branch**: `007-egui-frontend`  
**Created**: 2025-12-15  
**Status**: Draft  
**Input**: User description: "Replace Svelte/Tauri frontend with native egui GUI. Migrate the LocalMind desktop application from the current Svelte + Tauri + Vite stack to a pure Rust GUI using egui/eframe, eliminating all JavaScript/Node.js dependencies."

## Clarifications

### Session 2025-12-15

- Q: How should document content be rendered in detail view (HTML handling)? → A: Strip HTML tags and display as plain text.
- Q: What should the application display before the user enters a search query? → A: Show search box plus a list of recent documents.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Semantic Search with Results Display (Priority: P1)

A user launches LocalMind and wants to search their indexed bookmarks and documents. They type a query into the search field and see relevant results with similarity scores displayed in a scrollable list.

**Why this priority**: Search is the core function of LocalMind. Without this, the application has no value.

**Independent Test**: Can be fully tested by launching the app, entering a search query, and verifying results appear with titles and similarity scores. Delivers the primary value proposition of the application.

**Acceptance Scenarios**:

1. **Given** the application is running and documents are indexed, **When** user types a query and presses Enter or clicks Search, **Then** matching documents appear in a scrollable list with titles and similarity scores (0.0-1.0).
2. **Given** search results are displayed, **When** user scrolls, **Then** additional results beyond the visible area become visible.
3. **Given** no documents match the query, **When** search completes, **Then** an appropriate "no results" message is displayed.

---

### User Story 2 - Document Detail View (Priority: P1)

A user sees a search result they want to examine in detail. They click on the result to view the full document content, then navigate back to results.

**Why this priority**: Viewing document content is essential for the search feature to be useful. Part of the core search experience.

**Independent Test**: Can be tested by clicking any search result and verifying the full content displays, then clicking back to return to results.

**Acceptance Scenarios**:

1. **Given** search results are displayed, **When** user clicks on a result, **Then** the full document content is displayed in a detail view.
2. **Given** document detail view is shown, **When** user clicks back button or presses Escape, **Then** the search results view is restored with the previous query and results.
3. **Given** document has a URL, **When** viewing document details, **Then** the URL is displayed and can be opened in the default browser.

---

### User Story 3 - Bookmark Ingestion Progress Feedback (Priority: P2)

When LocalMind starts or processes new bookmarks, the user sees progress notifications showing which bookmarks are being processed and overall progress.

**Why this priority**: Users need feedback during potentially long-running operations to know the system is working and not frozen.

**Independent Test**: Can be tested by starting the application with unprocessed bookmarks and verifying progress toasts appear showing current bookmark and count.

**Acceptance Scenarios**:

1. **Given** the application is processing bookmarks, **When** processing is in progress, **Then** a toast notification displays "Processing bookmarks... X/Y (Z%)" with the current bookmark title.
2. **Given** bookmark processing completes, **When** the final bookmark is processed, **Then** a completion toast displays "Completed! N bookmarks ingested" and auto-dismisses after 5 seconds.
3. **Given** a toast is displayed, **When** the user does not interact with it and duration expires, **Then** the toast automatically disappears.

---

### User Story 4 - Settings: Exclusion Rules Management (Priority: P2)

A user wants to exclude certain bookmark folders or domain patterns from being indexed. They open settings, select folders to exclude via a tree view, add domain patterns, and save.

**Why this priority**: Allows users to customize what gets indexed, but not required for basic functionality.

**Independent Test**: Can be tested by opening settings, selecting folders/adding domains, saving, and verifying excluded bookmarks are removed from search results.

**Acceptance Scenarios**:

1. **Given** the settings modal is closed, **When** user clicks the settings button, **Then** a modal opens displaying exclusion rules configuration.
2. **Given** settings modal is open, **When** viewing folder exclusions, **Then** a tree view of Chrome bookmark folders is displayed with checkboxes.
3. **Given** settings modal is open, **When** user checks a folder, **Then** that folder is marked for exclusion.
4. **Given** settings modal is open, **When** user adds a domain pattern (e.g., "*.example.com"), **Then** the pattern is added to the exclusion list.
5. **Given** exclusion rules are configured, **When** user clicks Save, **Then** rules are persisted, matching bookmarks are removed from the index, and the modal closes.
6. **Given** settings modal is open, **When** user clicks Cancel or presses Escape, **Then** changes are discarded and the modal closes.

---

### User Story 5 - Application Launch and Initialization (Priority: P1)

A user double-clicks the LocalMind application. A window opens showing the search interface. The system initializes in the background and indicates when ready.

**Why this priority**: Core user experience - the application must start and present a usable interface.

**Independent Test**: Can be tested by launching the application and verifying the window appears with search UI within acceptable time.

**Acceptance Scenarios**:

1. **Given** LocalMind is not running, **When** user launches the application, **Then** a window appears within 3 seconds showing the search interface.
2. **Given** the embedding server is starting, **When** the application launches, **Then** a status indicator shows "Initializing..." until ready.
3. **Given** initialization completes, **When** the system is ready, **Then** an "initializing" indicator disappears and search becomes functional.
4. **Given** the application is ready and no search has been performed, **When** viewing the home screen, **Then** a list of recently added documents is displayed below the search box.

---

### Edge Cases

- What happens when the embedding server is unreachable? Display an error toast and allow the user to retry or exit gracefully.
- What happens when the database file is corrupted or missing? Display an error message indicating the issue; allow the application to start fresh if user confirms.
- How does the system handle extremely long document content in detail view? Content should be scrollable; no truncation.
- What happens if a search query is empty? Do not perform search; optionally show a hint to enter a query.
- How does the system handle window resize? UI should remain functional and readable at minimum window size of 800x600.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST display a search input field that accepts text queries.
- **FR-002**: System MUST display search results as a scrollable list with document title and similarity score.
- **FR-003**: System MUST allow users to click a search result to view full document content as plain text (HTML tags stripped).
- **FR-004**: System MUST provide back navigation from document detail view to search results.
- **FR-005**: System MUST display the document URL in detail view (if available) as a clickable link that opens in the default browser.
- **FR-006**: System MUST display toast notifications for bookmark processing progress.
- **FR-007**: Toast notifications MUST auto-dismiss after the specified duration (default 5 seconds) or remain visible if duration is 0 (for progress toasts).
- **FR-008**: System MUST provide a settings modal accessible via a settings button.
- **FR-009**: Settings modal MUST display Chrome bookmark folders in a tree view with checkboxes for exclusion.
- **FR-010**: Settings modal MUST allow adding domain patterns for exclusion via text input.
- **FR-011**: Settings modal MUST validate domain patterns before saving (accept valid patterns, reject invalid with error message).
- **FR-012**: System MUST persist exclusion rules to the database when saved.
- **FR-013**: System MUST remove bookmarks matching exclusion rules from the index when rules are saved.
- **FR-014**: System MUST continue running the HTTP server on port 3000-3010 for Chrome extension compatibility.
- **FR-015**: System MUST compile to a single binary with no external JavaScript or asset dependencies.
- **FR-016**: System MUST run the GUI in the same process as the RAG backend, using direct function calls instead of IPC.
- **FR-017**: Application window MUST support standard window operations (minimize, maximize, close, resize).
- **FR-018**: System MUST indicate initialization status to the user during startup.
- **FR-019**: System MUST display a list of recently added documents on the home screen before any search is performed.

### Key Entities

- **SearchResult**: Represents a search hit with document ID, title, content snippet, similarity score, and optional URL.
- **Document**: Full document with ID, title, content, URL, source type, and creation timestamp.
- **Toast**: Notification with message, type (info/success/error), duration, and unique ID.
- **ExclusionRule**: Either a folder ID or domain pattern used to filter bookmarks from indexing.
- **BookmarkFolder**: Chrome bookmark folder with ID, name, path hierarchy, and bookmark count.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Application launches and displays the search interface within 3 seconds on standard hardware.
- **SC-002**: Search results appear within 2 seconds of submitting a query (excluding embedding server latency).
- **SC-003**: Application binary size is under 50 MB (excluding debug symbols).
- **SC-004**: Application runs with under 200 MB memory usage during idle state (note: constitution's 50MB target refers to core application excluding in-memory vector store; 200MB includes loaded embeddings).
- **SC-005**: All existing search and document viewing functionality works identically to the Svelte/Tauri version.
- **SC-006**: Zero JavaScript or Node.js dependencies in the final build.
- **SC-007**: Chrome extension continues to function correctly via the HTTP API.
- **SC-008**: Application remains responsive during bookmark ingestion (search still works while processing).

## Assumptions

- The existing Rust backend (RAG pipeline, database, embedding client, HTTP server) remains unchanged and is reused directly.
- The egui/eframe framework provides sufficient UI capabilities for the required functionality.
- Users have the embedding server running separately (Python process) as is current behavior.
- Minimum supported window size is 800x600 pixels.
- The application targets Windows, macOS, and Linux (same platforms as Tauri).

## Out of Scope

- Changing the backend logic, database schema, or RAG pipeline.
- Modifying the Python embedding server.
- Adding new features beyond parity with the current Svelte UI.
- Mobile platform support.
- Theming or appearance customization beyond basic dark theme matching current design.
