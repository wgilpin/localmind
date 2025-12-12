# Tasks: HTTP REST API Server for Chrome Extension Integration

**Input**: Design documents from `/specs/001-http-api-server/`
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅

**Tests**: Tests are OPTIONAL per spec - not included unless explicitly requested. Focus on manual testing via quickstart.md procedures.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., [US1], [US2], [US3])
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `localmind-rs/src/`, `localmind-rs/tests/` at repository root
- Paths shown below use `localmind-rs/` prefix

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and dependency setup

- [ ] T001 Add HTTP server dependencies to `localmind-rs/Cargo.toml` (axum, tower, tower-http with cors feature)
- [ ] T002 [P] Verify existing dependencies (tokio, serde, serde_json) are compatible with axum requirements
- [ ] T003 [P] Run `cargo check` to verify project compiles with new dependencies

**Checkpoint**: Dependencies added and project compiles successfully

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core HTTP server infrastructure that MUST be complete before user stories can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T004 Create HTTP server module structure in `localmind-rs/src/http_server.rs` with module declaration
- [ ] T005 [P] Define `AppState` struct in `localmind-rs/src/http_server.rs` wrapping `RagState` for axum State extractor
- [ ] T006 [P] Define `ApiError` struct in `localmind-rs/src/http_server.rs` implementing `IntoResponse` trait for error handling
- [ ] T007 [P] Implement `find_available_port` function in `localmind-rs/src/http_server.rs` for port conflict handling (ports 3000-3010)
- [ ] T008 Create `start_http_server` function skeleton in `localmind-rs/src/http_server.rs` with basic axum Router setup
- [ ] T009 Add module declaration `mod http_server;` to `localmind-rs/src/main.rs`
- [ ] T010 Add import `use crate::http_server::start_http_server;` to `localmind-rs/src/main.rs`

**Checkpoint**: Foundation ready - HTTP server module structure exists and can be integrated into main.rs

---

## Phase 3: User Story 1 - Chrome Extension Sends Document via HTTP (Priority: P1) 🎯 MVP

**Goal**: Core functionality enabling Chrome extension to send documents via POST /documents endpoint. Document is stored in RAG system and becomes searchable.

**Independent Test**: Send POST request to `http://localhost:3000/documents` with valid document data and verify document appears in Tauri GUI search results.

### Implementation for User Story 1

- [ ] T011 [US1] Define `DocumentRequest` struct in `localmind-rs/src/http_server.rs` with Deserialize trait (title, content, url, extractionMethod fields)
- [ ] T012 [US1] Define `SuccessResponse` struct in `localmind-rs/src/http_server.rs` with Serialize trait (message, extractionMethod fields)
- [ ] T013 [US1] Implement `handle_post_documents` handler function in `localmind-rs/src/http_server.rs` with basic validation (title and content required)
- [ ] T014 [US1] Add RAG state initialization check in `handle_post_documents` in `localmind-rs/src/http_server.rs` (return HTTP 503 if not initialized)
- [ ] T015 [US1] Implement document ingestion call in `handle_post_documents` in `localmind-rs/src/http_server.rs` using `rag.ingest_document()` with source "chrome_extension", wrapping in error handling that returns HTTP 500 Internal Server Error with format `{ message: "Failed to add document: {error}" }` if ingestion fails
- [ ] T016 [US1] Add request validation error handling in `handle_post_documents` in `localmind-rs/src/http_server.rs` (return HTTP 400 for missing fields)
- [ ] T017 [US1] Add request size limit middleware to Router in `start_http_server` function in `localmind-rs/src/http_server.rs` (10MB limit via DefaultBodyLimit)
- [ ] T018 [US1] Add POST /documents route to Router in `start_http_server` function in `localmind-rs/src/http_server.rs`
- [ ] T019 [US1] Integrate HTTP server startup into RAG initialization flow in `localmind-rs/src/main.rs` (spawn after RAG system initializes)

**Checkpoint**: At this point, User Story 1 should be fully functional - Chrome extension can send documents and they appear in Tauri GUI search results

---

## Phase 4: User Story 4 - HTTP Server Shares RAG State and Database with Tauri GUI (Priority: P1)

**Goal**: Ensure HTTP server and Tauri GUI share the same RAG state and database, providing data consistency across interfaces.

**Independent Test**: Add document via HTTP POST, then immediately search for it in Tauri GUI. Add document via Tauri GUI, verify it's available to HTTP server.

**Note**: This story is primarily validated through architecture (same RagState instance) but requires explicit testing.

### Implementation for User Story 4

- [ ] T020 [US4] Verify `AppState` in `localmind-rs/src/http_server.rs` uses same `RagState` type as Tauri IPC commands
- [ ] T021 [US4] Ensure HTTP server receives `RagState` clone from same instance managed by Tauri in `localmind-rs/src/main.rs`
- [ ] T022 [US4] Add integration test or manual validation that documents added via HTTP are immediately searchable in Tauri GUI

**Checkpoint**: Data consistency verified - documents added via either interface are immediately available in both

---

## Phase 5: User Story 2 - HTTP Server Starts Automatically on Application Launch (Priority: P2)

**Goal**: HTTP server starts automatically when application launches, without manual configuration, on localhost:3000 (or alternative port).

**Independent Test**: Launch LocalMind application and immediately send test HTTP request to verify server is listening.

### Implementation for User Story 2

- [ ] T023 [US2] Implement port finding logic in `find_available_port` function in `localmind-rs/src/http_server.rs` (try ports 3000-3010 sequentially)
- [ ] T024 [US2] Add port binding logging in `start_http_server` function in `localmind-rs/src/http_server.rs` (log which port was successfully bound)
- [ ] T025 [US2] Add error handling for port unavailability in `start_http_server` function in `localmind-rs/src/http_server.rs` (log error but continue running Tauri GUI)
- [ ] T026 [US2] Ensure HTTP server starts after RAG initialization completes in `localmind-rs/src/main.rs` (spawn HTTP server task after RAG is stored in state)
- [ ] T027 [US2] Add startup logging in `start_http_server` function in `localmind-rs/src/http_server.rs` ("HTTP server listening on http://localhost:{port}")

**Checkpoint**: HTTP server starts automatically on application launch and is ready to accept requests

---

## Phase 6: User Story 3 - HTTP Server Handles CORS for Browser Extension Requests (Priority: P2)

**Goal**: HTTP server responds with appropriate CORS headers allowing browser extension requests to succeed.

**Independent Test**: Send POST request with Origin header and verify response includes Access-Control-Allow-Origin: * header.

### Implementation for User Story 3

- [ ] T028 [US3] Add CORS middleware to Router in `start_http_server` function in `localmind-rs/src/http_server.rs` using tower-http CorsLayer
- [ ] T029 [US3] Configure CORS to allow all origins (`Any`) in `start_http_server` function in `localmind-rs/src/http_server.rs`
- [ ] T030 [US3] Configure CORS to allow POST and OPTIONS methods in `start_http_server` function in `localmind-rs/src/http_server.rs`
- [ ] T031 [US3] Configure CORS to allow Content-Type header in `start_http_server` function in `localmind-rs/src/http_server.rs`
- [ ] T032 [US3] Verify OPTIONS preflight requests return appropriate CORS headers (axum handles automatically with CorsLayer)

**Checkpoint**: CORS headers correctly set - browser extension requests succeed

---

## Phase 7: User Story 5 - YouTube Video Transcript Enhancement (Priority: P3)

**Goal**: Automatically fetch YouTube video transcripts when YouTube URLs are detected, using transcript as document content.

**Independent Test**: Send POST request with YouTube URL and verify transcript is fetched and used as content, with title cleanup.

### Implementation for User Story 5

- [ ] T033 [US5] Add YouTube URL detection in `handle_post_documents` function in `localmind-rs/src/http_server.rs` using `YouTubeProcessor::is_youtube_url()`
- [ ] T034 [US5] Add YouTube transcript fetching logic in `handle_post_documents` function in `localmind-rs/src/http_server.rs` using `YouTubeProcessor::fetch_transcript()` with 30-second timeout
- [ ] T035 [US5] Add title cleanup logic in `handle_post_documents` function in `localmind-rs/src/http_server.rs` using `YouTubeProcessor::cleanup_title()` for YouTube URLs
- [ ] T036 [US5] Implement fallback to provided content if transcript fetch fails in `handle_post_documents` function in `localmind-rs/src/http_server.rs`
- [ ] T037 [US5] Add logging for extraction method in `handle_post_documents` function in `localmind-rs/src/http_server.rs` per FR-025

**Checkpoint**: YouTube transcript enhancement working - YouTube URLs automatically fetch transcripts with proper title cleanup

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Code quality, documentation, and validation improvements

- [ ] T038 [P] Add doc comments to all public functions in `localmind-rs/src/http_server.rs` per constitution requirements
- [ ] T039 [P] Run `cargo fmt` on `localmind-rs/src/http_server.rs` and `localmind-rs/src/main.rs`
- [ ] T040 [P] Run `cargo clippy` and fix all warnings in `localmind-rs/src/http_server.rs` and `localmind-rs/src/main.rs`
- [ ] T041 Add error logging for HTTP server startup failures in `localmind-rs/src/main.rs`
- [ ] T042 Add request processing logging in `handle_post_documents` function in `localmind-rs/src/http_server.rs` per FR-019
- [ ] T043 Validate all error responses match spec format `{ message: "..." }` in `localmind-rs/src/http_server.rs`
- [ ] T044 Validate success response format matches spec `{ message: "Document added successfully.", extractionMethod: string }` in `localmind-rs/src/http_server.rs`
- [ ] T045 Test all acceptance scenarios from spec.md using quickstart.md procedures
- [ ] T046 Verify HTTP server handles concurrent requests per SC-004 (test with 10+ simultaneous POST requests, verify no errors or data loss)
- [ ] T047 Verify HTTP server remains responsive while Tauri GUI performs searches per SC-007
- [ ] T048 Validate HTTP server startup timing per SC-002 (measure time from application launch completion to HTTP server listening, must be <5 seconds)
- [ ] T049 Validate document searchability timing per SC-003 (measure time from successful POST /documents response to document appearing in Tauri GUI search, must be <2 seconds)
- [ ] T050 Validate API response timing per SC-005 (measure POST /documents response time for documents up to 100KB, must be <5 seconds)

**Checkpoint**: Code quality validated, all acceptance scenarios pass, ready for integration testing

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational completion - Core MVP functionality
- **User Story 4 (Phase 4)**: Depends on User Story 1 completion - Validates shared state architecture
- **User Story 2 (Phase 5)**: Depends on Foundational completion - Can proceed in parallel with US1 after foundational
- **User Story 3 (Phase 6)**: Depends on Foundational completion - Can proceed in parallel with US1 after foundational
- **User Story 5 (Phase 7)**: Depends on User Story 1 completion - Enhances document processing
- **Polish (Phase 8)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories. **This is the MVP**.
- **User Story 4 (P1)**: Can start after User Story 1 - Validates architecture but doesn't require new implementation (shared state is architectural)
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Independent of other stories (server lifecycle)
- **User Story 3 (P2)**: Can start after Foundational (Phase 2) - Independent of other stories (CORS middleware)
- **User Story 5 (P3)**: Depends on User Story 1 - Enhances document processing with YouTube transcript fetching

### Within Each User Story

- Request/response structs before handler implementation
- Validation logic before document processing
- Error handling throughout implementation
- Story complete before moving to next priority

### Parallel Opportunities

- **Phase 1**: All setup tasks marked [P] can run in parallel
- **Phase 2**: Tasks T005, T006, T007 marked [P] can run in parallel (different structs/functions)
- **After Foundational**: User Stories 1, 2, and 3 can start in parallel (different concerns)
- **Phase 8**: All polish tasks marked [P] can run in parallel

---

## Parallel Example: User Story 1

```bash
# These can be done in parallel (different structs):
Task T011: Define DocumentRequest struct
Task T012: Define SuccessResponse struct

# These must be sequential (dependencies):
Task T013: Implement handler (depends on T011, T012)
Task T014: Add RAG check (depends on T013)
Task T015: Add ingestion (depends on T014)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T003)
2. Complete Phase 2: Foundational (T004-T010)
3. Complete Phase 3: User Story 1 (T011-T019)
4. **STOP and VALIDATE**: Test User Story 1 independently using quickstart.md procedures
5. Verify document appears in Tauri GUI search results
6. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (MVP!)
3. Add User Story 4 → Validate shared state → Deploy/Demo
4. Add User Story 2 → Test auto-start → Deploy/Demo
5. Add User Story 3 → Test CORS → Deploy/Demo
6. Add User Story 5 → Test YouTube enhancement → Deploy/Demo
7. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (MVP core endpoint)
   - Developer B: User Story 2 (auto-start) + User Story 3 (CORS) in parallel
   - Developer C: User Story 4 (validation) + User Story 5 (YouTube) after US1
3. Stories complete and integrate independently

---

## Task Summary

**Total Tasks**: 50

**Tasks by Phase**:
- Phase 1 (Setup): 3 tasks
- Phase 2 (Foundational): 7 tasks
- Phase 3 (User Story 1 - MVP): 9 tasks
- Phase 4 (User Story 4): 3 tasks
- Phase 5 (User Story 2): 5 tasks
- Phase 6 (User Story 3): 5 tasks
- Phase 7 (User Story 5): 5 tasks
- Phase 8 (Polish): 13 tasks

**Tasks by User Story**:
- User Story 1 (P1): 9 tasks - Core POST /documents endpoint
- User Story 2 (P2): 5 tasks - Auto-start server
- User Story 3 (P2): 5 tasks - CORS handling
- User Story 4 (P1): 3 tasks - Shared state validation
- User Story 5 (P3): 5 tasks - YouTube transcript enhancement

**Parallel Opportunities**: 15 tasks marked [P] can be executed in parallel

**Independent Test Criteria**:
- **US1**: Send POST request, verify document appears in Tauri GUI search
- **US2**: Launch app, immediately send HTTP request, verify server responds
- **US3**: Send request with Origin header, verify CORS headers in response
- **US4**: Add document via HTTP, search in Tauri GUI; add via Tauri, verify available to HTTP
- **US5**: Send POST with YouTube URL, verify transcript fetched and used

**Suggested MVP Scope**: Phases 1-3 (Setup + Foundational + User Story 1) = 19 tasks total. This delivers core Chrome extension integration functionality.

**Note**: Timing validation tasks (T048, T049, T050) can be performed during manual testing phase and don't block MVP delivery.

---

## Notes

- [P] tasks = different files or functions, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Manual testing via quickstart.md procedures (no automated test tasks per spec)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- HTTP server runs in same process as Tauri GUI (not separate executable)
- All tasks use existing RAG system - no database schema changes needed
