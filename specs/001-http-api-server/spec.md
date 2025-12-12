# Feature Specification: HTTP REST API Server for Chrome Extension Integration

**Feature Branch**: `001-http-api-server`  
**Created**: 2025-01-27  
**Status**: Draft  
**Input**: User description: "Add an HTTP REST API server to the Rust LocalMind application so the Chrome extension can connect and send documents. Currently, the Rust app only exposes Tauri IPC commands for the desktop GUI, so the extension can't communicate with it. The extension expects an HTTP server on localhost:3000 with a POST /documents endpoint that accepts document data (title, content, URL, extraction method) and stores it in the RAG system. The HTTP server should run alongside the Tauri GUI, share the same RAG state and database, handle CORS for browser extension requests, and start automatically when the application launches. This enables the Chrome extension to work with the Rust backend without requiring the separate TypeScript server, providing a unified backend experience. The implementation should maintain compatibility with the existing TypeScript backend API contract so the Chrome extension works without modification, while ensuring the HTTP server integrates seamlessly with the existing Rust application architecture and doesn't interfere with the Tauri GUI functionality."

## Clarifications

### Session 2025-01-27

- Q: Which CORS origin policy should be used for browser extension requests? → A: Allow all origins (`Access-Control-Allow-Origin: *`)
- Q: How should the system handle port conflicts when port 3000 is unavailable? → A: Try alternative ports automatically (3001, 3002, etc.) with corresponding logic in Chrome extension for port discovery
- Q: What should happen when HTTP requests exceed size limits? → A: Reject requests over 10MB with HTTP 413 Payload Too Large status
- Q: What timeout duration should be used for YouTube transcript fetching? → A: 30 seconds timeout
- Q: What HTTP status code and message should be returned when RAG system is still initializing? → A: HTTP 503 Service Unavailable with "System initializing" message

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Chrome Extension Sends Document via HTTP (Priority: P1)

A user browsing the web with the Chrome extension installed wants to save a webpage to their LocalMind knowledge base. They click the extension icon and select "Save Page". The extension extracts the page content and sends it to the LocalMind backend via HTTP POST request. The document is successfully stored in the RAG system and becomes searchable through the desktop application.

**Why this priority**: This is the core functionality that enables the Chrome extension to work with the Rust backend. Without this, users cannot add documents from their browser, which is a primary use case for the application.

**Independent Test**: Can be fully tested by sending a POST request to `http://localhost:3000/documents` with document data (title, content, url, extractionMethod) and verifying the document appears in search results within the Tauri GUI. This delivers immediate value by enabling browser-based document ingestion.

**Acceptance Scenarios**:

1. **Given** the LocalMind application is running with HTTP server active, **When** the Chrome extension sends a POST request to `/documents` with valid document data (title, content, url, extractionMethod), **Then** the document is stored in the RAG system and returns a success response with format `{ message: "Document added successfully.", extractionMethod: extractionMethod || 'dom' }`
2. **Given** the LocalMind application is running, **When** a document is successfully added via HTTP POST, **Then** that document is immediately searchable and retrievable through the Tauri GUI search functionality
3. **Given** the HTTP server receives a POST request with missing required fields (title or content), **When** the request is processed, **Then** the server returns a 400 Bad Request error with JSON format `{ message: "Title and content are required." }`
4. **Given** the HTTP server receives a POST request with payload exceeding 10MB, **When** the request is processed, **Then** the server returns a 413 Payload Too Large error with JSON format `{ message: "Request payload exceeds 10MB limit." }`
5. **Given** the HTTP server receives a POST request with a YouTube URL (youtube.com/watch or youtu.be), **When** the request is processed, **Then** the system attempts to fetch the YouTube transcript and uses it as the document content, falling back to provided content if transcript fetch fails
6. **Given** the HTTP server receives a POST request with a YouTube URL, **When** the title contains bracketed numbers at the start (e.g., "(1) Video Title"), **Then** the system removes the bracketed prefix before storing the document

---

### User Story 2 - HTTP Server Starts Automatically on Application Launch (Priority: P2)

A user launches the LocalMind desktop application. The application starts normally, displaying the Tauri GUI window. Simultaneously, the HTTP server starts automatically in the background on localhost:3000, ready to accept requests from the Chrome extension without requiring any manual server startup or configuration.

**Why this priority**: Users expect the HTTP server to be available immediately when they launch the application. Manual server startup would create friction and reduce the unified backend experience. This ensures seamless integration between the desktop app and browser extension.

**Independent Test**: Can be fully tested by launching the LocalMind application and immediately sending a test HTTP request to `http://localhost:3000/documents` without any manual server configuration. This delivers value by eliminating setup steps and ensuring the extension always works when the app is running.

**Acceptance Scenarios**:

1. **Given** the user launches the LocalMind application, **When** the application finishes initializing, **Then** the HTTP server is listening on localhost:3000 (or the first available port if 3000 is in use) and ready to accept requests
2. **Given** the application is starting up, **When** the RAG system initialization completes, **Then** the HTTP server becomes available on an available port and can process document requests
3. **Given** port 3000 is already in use, **When** the application starts, **Then** the HTTP server automatically tries ports 3001, 3002, etc. until it finds an available port and binds to it
4. **Given** no ports are available in the range 3000-3010, **When** the application starts, **Then** the application logs an error but continues running the Tauri GUI normally

---

### User Story 3 - HTTP Server Handles CORS for Browser Extension Requests (Priority: P2)

A user's Chrome extension attempts to send a document to the LocalMind HTTP server. The browser enforces CORS (Cross-Origin Resource Sharing) policies for extension requests. The HTTP server responds with appropriate CORS headers, allowing the browser extension to successfully complete the request and receive the response.

**Why this priority**: Browser extensions are subject to CORS restrictions when making HTTP requests. Without proper CORS headers, extension requests will fail even if the server is running correctly. This is essential for the Chrome extension to function.

**Independent Test**: Can be fully tested by sending a POST request from a browser extension context (or using curl with Origin header) and verifying the response includes appropriate CORS headers (Access-Control-Allow-Origin, Access-Control-Allow-Methods, etc.). This delivers value by ensuring browser extension compatibility.

**Acceptance Scenarios**:

1. **Given** the HTTP server receives a request with Origin header from a browser extension, **When** the server processes the request, **Then** the response includes CORS headers with Access-Control-Allow-Origin set to `*` (allowing all origins)
2. **Given** the HTTP server receives a preflight OPTIONS request, **When** the server processes the request, **Then** it returns appropriate CORS headers including Access-Control-Allow-Origin set to `*` and a 200 OK status
3. **Given** the HTTP server receives a request from any origin, **When** the server responds, **Then** the response includes Access-Control-Allow-Origin header set to `*`

---

### User Story 4 - HTTP Server Shares RAG State and Database with Tauri GUI (Priority: P1)

A user adds a document via the Chrome extension HTTP API, then opens the LocalMind desktop application. The document they added through the extension is immediately visible and searchable in the Tauri GUI. Similarly, documents added through the Tauri GUI are immediately available to the RAG system that serves HTTP requests. Both interfaces operate on the same underlying data store and RAG processing pipeline.

**Why this priority**: This ensures data consistency and provides a unified experience. Users expect documents added through either interface to be available everywhere. Without shared state, users would see inconsistent data between the extension and desktop app, breaking the unified backend promise.

**Independent Test**: Can be fully tested by adding a document via HTTP POST, then immediately searching for it in the Tauri GUI, and vice versa. This delivers value by ensuring a seamless, consistent experience across all interfaces.

**Acceptance Scenarios**:

1. **Given** a document is added via HTTP POST request, **When** a user searches for that document in the Tauri GUI, **Then** the document appears in search results
2. **Given** a document is added via Tauri GUI, **When** the RAG system processes HTTP requests, **Then** that document is included in search results and RAG context
3. **Given** both HTTP server and Tauri GUI are accessing the same database concurrently, **When** operations occur simultaneously, **Then** both interfaces see consistent, up-to-date data without conflicts or data loss

---

### User Story 5 - YouTube Video Transcript Enhancement (Priority: P3)

A user saves a YouTube video to their LocalMind knowledge base via the Chrome extension. The extension sends the video URL and page title. The HTTP server detects that it's a YouTube URL and automatically fetches the video transcript, using it as the document content instead of the page HTML. The video title is also cleaned up by removing any bracketed number prefixes. This provides richer, more searchable content for YouTube videos compared to just storing the page HTML.

**Why this priority**: This enhances the value of saved YouTube content by providing actual video transcripts rather than just page metadata. However, it's a nice-to-have enhancement that can gracefully fall back to provided content if transcript fetching fails, so it's lower priority than core functionality.

**Independent Test**: Can be fully tested by sending a POST request with a YouTube URL and verifying the transcript is fetched and used as content, with proper title cleanup. This delivers value by improving searchability of video content.

**Acceptance Scenarios**:

1. **Given** the HTTP server receives a POST request with a YouTube URL (youtube.com/watch or youtu.be), **When** the request is processed, **Then** the system fetches the video transcript and uses it as document content
2. **Given** a YouTube video transcript is successfully fetched, **When** the document is stored, **Then** the cleaned title (without bracketed prefixes) and transcript content are stored in the RAG system
3. **Given** YouTube transcript fetching fails (no transcript available, private video, etc.), **When** the request is processed, **Then** the system falls back to using the provided content field and still stores the document successfully
4. **Given** YouTube transcript fetching takes longer than 30 seconds, **When** the timeout occurs, **Then** the system falls back to using the provided content field and still stores the document successfully

---

### Edge Cases

- What happens when the HTTP server fails to start but the Tauri GUI initializes successfully?
- How does the system handle concurrent document additions from both HTTP API and Tauri GUI?
- What happens when the RAG system is still initializing and an HTTP request arrives? (Clarified: Returns HTTP 503 with "System initializing" message)
- How does the system handle HTTP requests when the database is locked or unavailable?
- What happens when port 3000 is already in use by another application? (Clarified: System tries alternative ports automatically)
- What happens when no ports are available in the range 3000-3010?
- How does the system handle malformed JSON in HTTP POST requests?
- What happens when an HTTP request includes extremely large document content? (Clarified: Requests over 10MB are rejected with HTTP 413)
- How does the system handle HTTP requests during application shutdown?
- What happens when the Chrome extension sends a request with invalid extractionMethod values?
- How does the system handle HTTP requests when the embedding service (Ollama/LM Studio) is unavailable?
- What happens when a YouTube URL is provided but transcript fetching fails (video has no transcript, private video, etc.)?
- How does the system handle YouTube URLs with timestamps or other query parameters?
- What happens when YouTube transcript fetching takes longer than expected or times out? (Clarified: 30 second timeout, falls back to provided content)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST expose an HTTP REST API server listening on localhost:3000
- **FR-002**: System MUST provide a POST /documents endpoint that accepts JSON document data
- **FR-003**: System MUST accept document data containing title (required), content (required), url (optional), and extractionMethod (optional) fields
- **FR-004**: System MUST validate that title and content fields are present and non-empty before processing
- **FR-004a**: System MUST reject HTTP requests with payload size exceeding 10MB, returning HTTP 413 Payload Too Large status with error message
- **FR-005**: System MUST store documents received via HTTP POST in the same RAG system used by the Tauri GUI
- **FR-006**: System MUST return appropriate HTTP status codes (200 for success, 400 for validation errors, 413 for payload too large, 503 for service unavailable during initialization, 500 for server errors)
- **FR-007**: System MUST return JSON responses with success messages or error details in the format `{ message: "..." }` for both success and error cases
- **FR-007a**: System MUST return success responses in the format `{ message: "Document added successfully.", extractionMethod: extractionMethod || 'dom' }` where extractionMethod matches the value provided in the request or defaults to 'dom'
- **FR-007b**: System MUST return error responses in the format `{ message: "..." }` with descriptive error messages
- **FR-008**: System MUST handle CORS headers for browser extension requests, including Access-Control-Allow-Origin set to `*` (allow all origins), Access-Control-Allow-Methods, and Access-Control-Allow-Headers
- **FR-009**: System MUST respond to OPTIONS preflight requests with appropriate CORS headers, including Access-Control-Allow-Origin set to `*`
- **FR-010**: System MUST start the HTTP server automatically when the application launches
- **FR-011**: System MUST start the HTTP server after RAG system initialization completes
- **FR-012**: System MUST allow the HTTP server and Tauri GUI to run concurrently without interference
- **FR-013**: System MUST share the same database instance between HTTP server and Tauri GUI
- **FR-014**: System MUST share the same RAG state and processing pipeline between HTTP server and Tauri GUI
- **FR-015**: System MUST maintain API compatibility with the existing TypeScript backend contract for the POST /documents endpoint
- **FR-016**: System MUST handle the extractionMethod field from HTTP requests and preserve it as metadata
- **FR-017**: System MUST process documents through the same chunking and embedding pipeline regardless of source (HTTP or Tauri)
- **FR-018**: System MUST handle HTTP requests gracefully when the RAG system is still initializing, returning HTTP 503 Service Unavailable status with message "System initializing. Please wait a moment and try again."
- **FR-019**: System MUST log HTTP server startup, shutdown, and request processing events. Logging should use Rust's standard logging framework (e.g., `log` crate or `tracing`) with appropriate log levels: INFO for startup/shutdown, INFO for successful requests, ERROR for failures. Log messages should include relevant context (port number, request details, error messages) for debugging purposes.
- **FR-020**: System MUST handle port conflicts gracefully by automatically trying alternative ports (3001, 3002, 3003, etc.) if port 3000 is unavailable, stopping at the first available port
- **FR-020a**: System MUST log which port the HTTP server successfully bound to (for Chrome extension discovery)
- **FR-020b**: System MUST attempt ports sequentially starting from 3000, with a reasonable upper limit (e.g., stop after 10 attempts or reaching port 3010)
- **FR-021**: System MUST detect YouTube URLs (youtube.com/watch, youtu.be, m.youtube.com) in the url field of POST /documents requests
- **FR-022**: System MUST attempt to fetch YouTube video transcripts when a YouTube URL is detected, using the transcript as document content if successfully retrieved
- **FR-022a**: System MUST timeout YouTube transcript fetching after 30 seconds if no response is received
- **FR-023**: System MUST fall back to the provided content field if YouTube transcript fetching fails, times out, or is unavailable
- **FR-024**: System MUST clean YouTube video titles by removing bracketed number prefixes (e.g., "(1) Video Title" becomes "Video Title") before storing documents
- **FR-025**: System MUST log the extraction method used for each document submission for analytics purposes

### Key Entities *(include if feature involves data)*

- **Document**: Represents a webpage or content item stored in the knowledge base. Contains title (required), content (required), source URL (optional), extraction method metadata (optional), and timestamps. Documents are chunked and embedded for semantic search.
- **HTTP Request**: Represents a browser extension request to add a document. Contains JSON payload with document fields. Must be validated and processed through the RAG pipeline.
- **RAG State**: Shared application state containing the database connection, vector store, embedding client, and document processor. Both HTTP server and Tauri GUI access this shared state for consistent data operations.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Chrome extension can successfully send documents to the HTTP API without requiring any code modifications to the extension
- **SC-002**: HTTP server starts automatically within 5 seconds of application launch completion
- **SC-003**: Documents added via HTTP POST are searchable in the Tauri GUI within 2 seconds of successful API response
- **SC-004**: HTTP server handles at least 10 concurrent document submission requests without errors or data loss
- **SC-005**: API response time for POST /documents requests is under 5 seconds for documents up to 100KB in size
- **SC-006**: Zero data inconsistencies between documents added via HTTP API and documents visible in Tauri GUI
- **SC-007**: HTTP server remains available and responsive while Tauri GUI is actively being used for searches and document management
- **SC-008**: CORS headers are correctly set for 100% of browser extension requests, allowing successful cross-origin requests

## Assumptions

- Port 3000 or an alternative port (3001-3010) will be available on the user's machine (if none are available, the application will log an error but continue running)
- Chrome extension will implement port discovery logic to try multiple ports when connecting (this is out of scope for this HTTP server spec but required for full functionality)
- The Chrome extension will continue to use the existing API contract (POST /documents with title, content, url, extractionMethod fields)
- Users expect the HTTP server to run automatically without manual configuration
- The RAG system initialization time is acceptable for HTTP server startup delay
- Browser extensions making requests will include appropriate Origin headers for CORS handling
- The existing database and RAG state management can support concurrent access from both HTTP server and Tauri GUI
- Error responses should be user-friendly and help diagnose connection or data issues
- YouTube videos may not always have transcripts available (private videos, disabled captions, etc.), and the system should gracefully handle these cases
- YouTube transcript fetching may take additional time, and this delay is acceptable for the enhanced content quality

## Dependencies

- Existing RAG system initialization and state management
- Existing database schema and document storage mechanisms
- Existing document processing pipeline (chunking, embedding, vector storage)
- Tauri application lifecycle and setup hooks
- Chrome extension API contract (must remain compatible)
- YouTube transcript fetching capability (existing functionality in Rust codebase)
- YouTube URL detection and title cleanup utilities (existing functionality in Rust codebase)

## API Contract Details

### POST /documents Request Format
- **Method**: POST
- **Content-Type**: application/json
- **Body**: `{ title: string (required), content: string (required), url?: string (optional), extractionMethod?: string (optional) }`

### POST /documents Success Response (200 OK)
- **Format**: `{ message: "Document added successfully.", extractionMethod: string }`
- **extractionMethod**: The extraction method from the request, or 'dom' if not provided

### POST /documents Error Responses
- **400 Bad Request**: `{ message: "Title and content are required." }` - When title or content is missing
- **413 Payload Too Large**: `{ message: "Request payload exceeds 10MB limit." }` - When request body exceeds 10MB
- **503 Service Unavailable**: `{ message: "System initializing. Please wait a moment and try again." }` - When RAG system is still initializing
- **500 Internal Server Error**: `{ message: "Failed to add document." }` - When document processing fails

### YouTube URL Handling
- **Detection**: URLs containing "youtube.com/watch", "youtu.be", or "m.youtube.com" are treated as YouTube videos
- **Transcript Fetching**: System attempts to fetch video transcript and uses it as document content
- **Title Cleanup**: Titles starting with bracketed numbers (e.g., "(1) Video Title") have the prefix removed
- **Fallback**: If transcript fetching fails, system uses the provided content field

## Context: Desktop-Daemon Deprecation

The desktop-daemon (TypeScript HTTP server) and its web frontend are being deprecated. The Rust application with Tauri GUI is replacing the desktop-daemon entirely. The HTTP API server being added to the Rust application **only needs to support the Chrome extension**, which requires a single endpoint: `POST /documents`.

**Why only POST /documents is needed:**
- The Chrome extension only uses `POST /documents` to send documents from web pages
- All other functionality (search, document management, AI generation, bookmarks) is available through the Tauri GUI IPC commands
- No web frontend needs to be served, eliminating the need for other HTTP endpoints
- The unified backend experience is achieved by having the HTTP server share the same RAG state and database as the Tauri GUI

## Out of Scope

- **HTTP endpoints beyond POST /documents**: The Chrome extension only requires POST /documents. All other functionality (document retrieval, updates, deletions, search, AI generation, configuration) is available through Tauri GUI IPC commands. Since the desktop-daemon is being deprecated, there is no need for additional HTTP endpoints that were previously used by its web frontend.
- **Static file serving**: No web frontend needs to be served by the HTTP server
- **Additional REST API endpoints**: Endpoints like GET /documents/:id, DELETE /notes/:id, PUT /notes/:id, GET /ranked-chunks, GET /recent-notes, GET /search-stream, POST /stop-generation, POST /log-result-click, GET/POST /models, GET /status-stream are not needed
- Authentication or authorization for HTTP endpoints (assumes localhost-only access is sufficient)
- HTTPS/TLS support (localhost HTTP is sufficient for local extension communication)
- Rate limiting or request throttling beyond basic concurrency handling
- API versioning or multiple API endpoints beyond POST /documents
- WebSocket or Server-Sent Events for real-time updates to the extension
- HTTP server configuration options or user-configurable ports
- Migration or compatibility layer for existing TypeScript server data
