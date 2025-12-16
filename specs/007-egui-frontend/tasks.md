# Tasks: Native egui Desktop GUI

**Input**: Design documents from `/specs/007-egui-frontend/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, quickstart.md

**Tests**: Not explicitly requested in specification. Manual testing per quickstart.md.

**Organization**: Tasks grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1-US5)
- Include exact file paths in descriptions

## Path Conventions

- **Source**: `localmind-rs/src/`
- **GUI Module**: `localmind-rs/src/gui/`
- **Delete**: `localmind-rs/src-ui/`, `localmind-rs/*.json`, `localmind-rs/*.js`

---

## Phase 1: Setup (Cleanup & Dependencies) ✅ COMPLETE

**Purpose**: Remove Svelte/Tauri artifacts and configure egui dependencies

- [x] T001 Delete `localmind-rs/src-ui/` directory (entire Svelte frontend)
- [x] T002 [P] Delete `localmind-rs/package.json`
- [x] T003 [P] Delete `localmind-rs/package-lock.json`
- [x] T004 [P] Delete `localmind-rs/vite.config.js`
- [x] T005 [P] Delete `localmind-rs/svelte.config.js`
- [x] T006 [P] Delete `localmind-rs/tauri.conf.json`
- [x] T007 [P] Delete `localmind-rs/build.rs`
- [x] T008 Update `localmind-rs/Cargo.toml`: Remove tauri, tauri-build dependencies
- [x] T009 Update `localmind-rs/Cargo.toml`: Add eframe, egui, egui_extras, open, html2text, poll-promise dependencies
- [x] T010 Run `cargo check` to verify dependency changes compile

**Checkpoint**: ✅ Project compiles with new dependencies, all JS/Svelte files removed

---

## Phase 2: Foundational (Core GUI Infrastructure) ✅ COMPLETE

**Purpose**: Create GUI module structure and shared state types - MUST complete before user stories

**CRITICAL**: No user story work can begin until this phase is complete

- [x] T011 Create `localmind-rs/src/gui/mod.rs` with module exports
- [x] T012 [P] Create `localmind-rs/src/gui/state.rs` with View, InitStatus, Toast, ToastType enums
- [x] T013 [P] Create `localmind-rs/src/gui/views/mod.rs` with view module exports
- [x] T014 [P] Create `localmind-rs/src/gui/widgets/mod.rs` with widget module exports
- [x] T015 Create `localmind-rs/src/gui/app.rs` with LocalMindApp struct skeleton (fields only, no impl)
- [x] T016 Update `localmind-rs/src/lib.rs` to export gui module
- [x] T017 Update `localmind-rs/src/http_server.rs`: Replace Tauri RagState type with standalone `Arc<RwLock<Option<RAG>>>`
- [x] T018 Add `get_recent_documents(limit: usize)` method to `localmind-rs/src/db.rs`
- [x] T019 Add `strip_html(content: &str) -> String` helper function in `localmind-rs/src/gui/app.rs`

**Checkpoint**: ✅ GUI module structure exists, compiles, http_server works without Tauri types

---

## Phase 3: User Story 5 - Application Launch & Initialization (Priority: P1) MVP ✅ COMPLETE

**Goal**: Application launches with egui window, shows initialization status, displays home screen with recent documents

**Independent Test**: Launch app, verify window appears within 3 seconds, shows "Initializing..." then recent documents list

### Implementation for User Story 5

- [x] T020 [US5] Replace `localmind-rs/src/main.rs` with eframe entry point (NativeOptions, run_native)
- [x] T021 [US5] Implement `LocalMindApp::new()` in `localmind-rs/src/gui/app.rs`: Create RAG state, spawn HTTP server thread, spawn RAG init task
- [x] T022 [US5] Implement `eframe::App` trait for LocalMindApp in `localmind-rs/src/gui/app.rs`: Basic update() with top panel
- [x] T023 [US5] Create `localmind-rs/src/gui/views/home.rs` with render_home_view() showing recent documents list
- [x] T024 [US5] Implement init status display in top panel (show "Initializing..." when not ready)
- [x] T025 [US5] Add channel for RAG init completion notification in `localmind-rs/src/gui/app.rs`
- [x] T026 [US5] Load recent documents when RAG becomes ready in `localmind-rs/src/gui/app.rs`
- [x] T027 [US5] Add window icon loading from `localmind-rs/icons/` in main.rs
- [x] T028 [US5] Verify window appears within 3 seconds, minimum size 800x600

**Checkpoint**: ✅ App launches with egui window, shows init status, displays recent documents on home screen

---

## Phase 4: User Story 1 - Semantic Search with Results Display (Priority: P1) ✅ COMPLETE

**Goal**: User can type query, press Enter, and see search results with titles and similarity scores

**Independent Test**: Enter query, verify results appear in scrollable list with similarity scores

### Implementation for User Story 1

- [x] T029 [US1] Add search input field to top panel in `localmind-rs/src/gui/app.rs`
- [x] T030 [US1] Create `localmind-rs/src/gui/views/search.rs` with render_search_results() function
- [x] T031 [US1] Implement async search using poll-promise pattern in `localmind-rs/src/gui/app.rs`
- [x] T032 [US1] Add Enter key handling to trigger search in `localmind-rs/src/gui/app.rs`
- [x] T033 [US1] Display search results as scrollable list with title and similarity score in `localmind-rs/src/gui/views/search.rs`
- [x] T034 [US1] Show "No results" message when search returns empty in `localmind-rs/src/gui/views/search.rs`
- [x] T035 [US1] Add similarity cutoff slider and "Load More" button in `localmind-rs/src/gui/views/search.rs`
- [x] T036 [US1] Implement view transition from Home to SearchResults in `localmind-rs/src/gui/app.rs`

**Checkpoint**: ✅ Search works end-to-end, results display with scores, scrolling works

---

## Phase 5: User Story 2 - Document Detail View (Priority: P1) ✅ COMPLETE

**Goal**: User clicks search result to view full document content, can navigate back

**Independent Test**: Click result, verify full content displays with URL, click back to return

### Implementation for User Story 2

- [x] T037 [US2] Make search result items clickable in `localmind-rs/src/gui/views/search.rs`
- [x] T038 [US2] Create `localmind-rs/src/gui/views/document.rs` with render_document_view() function
- [x] T039 [US2] Fetch and display full document content (HTML stripped) in `localmind-rs/src/gui/views/document.rs`
- [x] T040 [US2] Display document URL as clickable link using `open` crate in `localmind-rs/src/gui/views/document.rs`
- [x] T041 [US2] Add back button to document view in `localmind-rs/src/gui/views/document.rs`
- [x] T042 [US2] Implement Escape key to navigate back in `localmind-rs/src/gui/app.rs`
- [x] T043 [US2] Implement view transition to DocumentDetail and back in `localmind-rs/src/gui/app.rs`
- [x] T044 [US2] Ensure document content is scrollable for long documents in `localmind-rs/src/gui/views/document.rs`

**Checkpoint**: ✅ Document detail view works, back navigation works, URL opens in browser

---

## Phase 6: User Story 3 - Bookmark Ingestion Progress (Priority: P2) ✅ COMPLETE

**Goal**: Toast notifications show bookmark processing progress

**Independent Test**: Start app with unprocessed bookmarks, verify progress toasts appear

### Implementation for User Story 3

- [x] T045 [US3] Create `localmind-rs/src/gui/widgets/toast.rs` with Toast struct and render_toasts() function
- [x] T046 [US3] Implement toast auto-dismiss timer logic in `localmind-rs/src/gui/widgets/toast.rs`
- [x] T047 [US3] Add toast overlay rendering in bottom-right corner in `localmind-rs/src/gui/app.rs`
- [x] T048 [US3] Create channel for bookmark progress events in `localmind-rs/src/gui/app.rs`
- [x] T049 [US3] Update bookmark monitoring to send progress through channel instead of Tauri events
- [x] T050 [US3] Display "Processing bookmarks... X/Y (Z%)" toast during ingestion in `localmind-rs/src/gui/app.rs`. Only one toast of this type to be visible at any one time.
- [x] T051 [US3] Display completion toast "Completed! N bookmarks ingested" in `localmind-rs/src/gui/app.rs`
- [x] T052 [US3] Style toasts with appropriate colors (info=blue, success=green, error=red) in `localmind-rs/src/gui/widgets/toast.rs`

**Checkpoint**: ✅ Toast notifications appear during bookmark processing, auto-dismiss works

---

## Phase 7: User Story 4 - Settings: Exclusion Rules (Priority: P2) ✅ COMPLETE

**Goal**: Settings modal with folder tree and domain pattern management

**Independent Test**: Open settings, check folders, add domain, save, verify exclusions applied

### Implementation for User Story 4

- [x] T053 [US4] Add settings button (gear icon) to top panel in `localmind-rs/src/gui/app.rs`
- [x] T054 [US4] Create `localmind-rs/src/gui/widgets/settings.rs` with render_settings_modal() function
- [x] T055 [US4] Create `localmind-rs/src/gui/widgets/folder_tree.rs` with render_folder_tree() recursive function
- [x] T056 [US4] Load bookmark folders on settings open in `localmind-rs/src/gui/app.rs`
- [x] T057 [US4] Load current exclusion rules from database on settings open in `localmind-rs/src/gui/app.rs`
- [x] T058 [US4] Implement folder checkbox selection in `localmind-rs/src/gui/widgets/folder_tree.rs`
- [x] T059 [US4] Add domain pattern input field and list in `localmind-rs/src/gui/widgets/settings.rs`
- [x] T060 [US4] Implement domain pattern validation with error display in `localmind-rs/src/gui/widgets/settings.rs`
- [x] T061 [US4] Implement Save button: persist rules, remove matching bookmarks in `localmind-rs/src/gui/widgets/settings.rs`
- [x] T062 [US4] Implement Cancel button and Escape key to close without saving in `localmind-rs/src/gui/app.rs`
- [x] T063 [US4] Show confirmation message after successful save in `localmind-rs/src/gui/app.rs`

**Checkpoint**: ✅ Settings modal works, exclusion rules persist and apply

---

## Phase 8: Polish & Cross-Cutting Concerns ✅ COMPLETE

**Purpose**: Final cleanup and verification

- [x] T064 [P] Run `cargo fmt` on all new files
- [x] T065 [P] Run `cargo clippy` and fix all warnings
- [x] T066 Apply dark theme styling to match original design in `localmind-rs/src/gui/app.rs`
- [x] T067 [P] Add doc comments to all public functions in gui module
- [x] T068 Verify Chrome extension still works via HTTP API
- [x] T069 Test window resize behavior (minimum 800x600)
- [x] T070 Run `cargo build --release` and verify binary size < 50MB
- [x] T071 Test memory usage during idle (target < 200MB)
- [x] T072 Update `localmind-rs/README.md` with new build instructions
- [x] T073 Run full manual test per `specs/007-egui-frontend/quickstart.md`

**Checkpoint**: ✅ All quality gates pass, application ready for use

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 5 (Phase 3)**: Depends on Foundational - MVP entry point
- **User Story 1 (Phase 4)**: Depends on User Story 5 (needs app running)
- **User Story 2 (Phase 5)**: Depends on User Story 1 (needs search results)
- **User Story 3 (Phase 6)**: Depends on Foundational only (toast system independent)
- **User Story 4 (Phase 7)**: Depends on Foundational only (settings independent)
- **Polish (Phase 8)**: Depends on all user stories complete

### User Story Dependencies

```
Phase 1 (Setup)
    ↓
Phase 2 (Foundational)
    ↓
    ├──→ Phase 3 (US5: App Launch) ──→ Phase 4 (US1: Search) ──→ Phase 5 (US2: Document View)
    │
    ├──→ Phase 6 (US3: Toasts) [can parallel with US5]
    │
    └──→ Phase 7 (US4: Settings) [can parallel with US5]
         ↓
      Phase 8 (Polish)
```

### Within Each User Story

- Core infrastructure before UI components
- View rendering before interaction logic
- Basic functionality before polish

### Parallel Opportunities

**Phase 1 (Setup)**:
- T002, T003, T004, T005, T006, T007 can all run in parallel (independent file deletions)

**Phase 2 (Foundational)**:
- T012, T013, T014 can run in parallel (different module files)

**After Foundational**:
- US3 (Toasts) and US4 (Settings) can run in parallel with US5
- However, US1 depends on US5, and US2 depends on US1

**Phase 8 (Polish)**:
- T064, T065, T067 can run in parallel

---

## Parallel Example: Phase 2 Foundational

```bash
# Launch all independent module creation together:
Task: "Create localmind-rs/src/gui/state.rs with View, InitStatus, Toast, ToastType enums"
Task: "Create localmind-rs/src/gui/views/mod.rs with view module exports"
Task: "Create localmind-rs/src/gui/widgets/mod.rs with widget module exports"
```

---

## Implementation Strategy

### MVP First (User Story 5 + User Story 1)

1. Complete Phase 1: Setup (delete old files, update deps)
2. Complete Phase 2: Foundational (create gui module structure)
3. Complete Phase 3: User Story 5 (app launches, shows home)
4. Complete Phase 4: User Story 1 (search works)
5. **STOP and VALIDATE**: App launches, search works, results display
6. This is a functional MVP

### Incremental Delivery

1. Setup + Foundational → Project compiles with egui
2. Add US5 (App Launch) → Window appears, init works → MVP-0
3. Add US1 (Search) → Core search works → MVP-1
4. Add US2 (Document View) → Full search experience → MVP-2
5. Add US3 (Toasts) → Progress feedback → Enhanced UX
6. Add US4 (Settings) → Exclusion rules → Full feature parity
7. Polish → Production ready

### Suggested Order for Solo Developer

T001-T010 → T011-T019 → T020-T028 → T029-T036 → T037-T044 → T045-T052 → T053-T063 → T064-T073

---

## Notes

- [P] tasks = different files, no dependencies on incomplete tasks
- [US#] label maps task to specific user story for traceability
- Each user story can be demonstrated independently after completion
- Backend modules (db.rs, rag.rs, bookmark.rs) remain unchanged except where noted
- HTTP server must remain functional for Chrome extension compatibility
- All UI code is new in gui/ module - no modification of existing UI code needed
