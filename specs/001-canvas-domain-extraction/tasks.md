# Tasks: Canvas-Based Domain Content Extraction

**Input**: Design documents from `/specs/001-canvas-domain-extraction/`  
**Prerequisites**: plan.md ✓, spec.md ✓

**Tests**: Not requested in specification - implementation-focused tasks only.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

This is a Chrome Extension project with backend integration:
- **Extension**: `chrome-extension/` (front-end JavaScript)
- **Backend**: `desktop-daemon/src/` (Node.js/TypeScript)
- **Build**: `chrome-extension/dist/` (webpack output)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and build system setup

- [x] T001 Create chrome-extension build configuration directory structure
- [x] T002 Initialize package.json with webpack dependencies in chrome-extension/package.json
- [x] T003 [P] Create webpack.config.js in chrome-extension/webpack.config.js
- [x] T004 [P] Create default JSON configuration file in chrome-extension/config/special-domains-default.json

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core configuration and domain matching infrastructure that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T005 Implement ConfigManager class in chrome-extension/config-manager.js
- [x] T006 Add loadConfig() method with chrome.storage.local integration in chrome-extension/config-manager.js
- [x] T007 Add saveConfig() method with validation in chrome-extension/config-manager.js
- [x] T008 Implement isSpecialDomain() URL matching logic in chrome-extension/config-manager.js
- [x] T009 Implement matchesDomain() with support for exact, subdomain, and pattern matching in chrome-extension/config-manager.js
- [x] T010 Implement validateConfig() with JSON schema validation in chrome-extension/config-manager.js
- [x] T011 Add getHardcodedDefaults() with docs.google.com preset in chrome-extension/config-manager.js
- [x] T012 Update manifest.json to add clipboardRead, clipboardWrite, and storage permissions in chrome-extension/manifest.json

**Checkpoint**: Configuration system ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Add Bookmark from Canvas-Rendered Domain (Priority: P1) 🎯 MVP

**Goal**: Users can bookmark content from canvas-rendered domains (like docs.google.com) using clipboard extraction, with automatic detection and fallback handling.

**Independent Test**: Install extension, navigate to docs.google.com, click Save Page button, verify content is extracted via clipboard and sent to backend successfully.

### Implementation for User Story 1

#### Clipboard Extraction Core

- [x] T013 [P] [US1] Implement checkClipboardPermission() function in chrome-extension/content-clipboard.js
- [x] T014 [P] [US1] Implement readClipboard() function in chrome-extension/content-clipboard.js
- [x] T015 [P] [US1] Implement writeClipboard() function in chrome-extension/content-clipboard.js
- [x] T016 [P] [US1] Implement isContentEmpty() validation function in chrome-extension/content-clipboard.js
- [x] T017 [US1] Implement performClipboardExtraction() main workflow in chrome-extension/content-clipboard.js
- [x] T018 [US1] Add clipboard save logic before extraction in chrome-extension/content-clipboard.js
- [x] T019 [US1] Add select-all and copy commands (document.execCommand) in chrome-extension/content-clipboard.js
- [x] T020 [US1] Add clipboard restore logic after extraction in chrome-extension/content-clipboard.js
- [x] T021 [US1] Add content validation with 10-character threshold in chrome-extension/content-clipboard.js

#### Error Handling Dialogs

- [x] T022 [P] [US1] Create ExtractionDialogs class structure in chrome-extension/ui/dialogs.js
- [x] T023 [P] [US1] Implement showPermissionDialog() with grant/fallback options in chrome-extension/ui/dialogs.js
- [x] T024 [P] [US1] Implement showEmptyContentDialog() with retry/save options in chrome-extension/ui/dialogs.js
- [x] T025 [P] [US1] Implement showProgressIndicator() for extraction feedback in chrome-extension/ui/dialogs.js
- [x] T026 [P] [US1] Implement showError() for general error messages in chrome-extension/ui/dialogs.js
- [x] T027 [P] [US1] Create modal CSS styles in chrome-extension/ui/dialogs.css
- [x] T028 [P] [US1] Add overlay CSS styles in chrome-extension/ui/dialogs.css
- [x] T029 [P] [US1] Add button and warning styles in chrome-extension/ui/dialogs.css

#### Error Handlers

- [x] T030 [US1] Implement handlePermissionDenied() with dialog integration in chrome-extension/content-clipboard.js
- [x] T031 [US1] Implement handleEmptyContent() with dialog integration in chrome-extension/content-clipboard.js
- [x] T032 [US1] Implement handleExtractionError() for general failures in chrome-extension/content-clipboard.js
- [x] T033 [US1] Add fallback to standard DOM extraction when user chooses fallback in chrome-extension/content-clipboard.js

#### Content Script Integration

- [x] T034 [US1] Update content.js to load ConfigManager in chrome-extension/content.js
- [x] T035 [US1] Add special domain detection logic in chrome-extension/content.js
- [x] T036 [US1] Add clipboard extraction branch for special domains in chrome-extension/content.js
- [x] T037 [US1] Add standard DOM extraction branch for normal domains in chrome-extension/content.js
- [x] T038 [US1] Add extractionMethod field to message payload in chrome-extension/content.js

#### Popup Integration

- [x] T039 [US1] Update popup.js to inject content-clipboard.js script in chrome-extension/popup.js
- [x] T040 [US1] Update popup.js to inject config-manager.js script in chrome-extension/popup.js
- [x] T041 [US1] Add extraction method detection in message handler in chrome-extension/popup.js
- [x] T042 [US1] Add "Using clipboard extraction" status message in chrome-extension/popup.js

#### Backend Integration

- [x] T043 [P] [US1] Update /documents endpoint to accept extractionMethod parameter in desktop-daemon/src/index.ts
- [x] T044 [P] [US1] Add extraction method logging to /documents endpoint in desktop-daemon/src/index.ts
- [x] T045 [P] [US1] Update document metadata to include extractionMethod in desktop-daemon/src/index.ts
- [x] T046 [US1] Add extraction_method column to documents table schema in desktop-daemon/src/services/database.ts
- [x] T047 [US1] Create database migration for extraction_method column in desktop-daemon/src/services/database.ts

**Checkpoint**: At this point, User Story 1 should be fully functional - users can bookmark from docs.google.com with clipboard extraction

---

## Phase 4: User Story 2 - Advanced User Configures Additional Special Domains (Priority: P2)

**Goal**: Advanced users can export/import JSON configuration to add custom special domains without code changes.

**Independent Test**: Export config, add a test domain (e.g., "figma.com"), import config, restart extension, verify new domain uses clipboard extraction.

### Implementation for User Story 2

#### Config Export/Import

- [ ] T048 [P] [US2] Implement exportConfig() to generate formatted JSON string in chrome-extension/config-manager.js
- [ ] T049 [P] [US2] Implement importConfig() to parse and validate JSON in chrome-extension/config-manager.js
- [ ] T050 [P] [US2] Add getConfigLocation() helper message in chrome-extension/config-manager.js

#### Config UI in Popup

- [ ] T051 [US2] Add config panel HTML to popup.html in chrome-extension/popup.html
- [ ] T052 [US2] Add config button and panel toggle in popup.html in chrome-extension/popup.html
- [ ] T053 [US2] Add export/import buttons to config panel in chrome-extension/popup.html
- [ ] T054 [US2] Add domain list container to config panel in chrome-extension/popup.html
- [ ] T055 [US2] Implement config button click handler in chrome-extension/popup.js
- [ ] T056 [US2] Implement export config button handler in chrome-extension/popup.js
- [ ] T057 [US2] Implement import config button handler in chrome-extension/popup.js
- [ ] T058 [US2] Implement view location button handler in chrome-extension/popup.js
- [ ] T059 [US2] Implement renderConfigList() to display domains in chrome-extension/popup.js
- [ ] T060 [P] [US2] Add config panel styles to popup.css in chrome-extension/popup.css
- [ ] T061 [P] [US2] Add config button styles to popup.css in chrome-extension/popup.css
- [ ] T062 [P] [US2] Add domain list styles to popup.css in chrome-extension/popup.css

#### Config Validation UI

- [ ] T063 [US2] Add invalid config error messaging in import handler in chrome-extension/popup.js
- [ ] T064 [US2] Add success messaging for export/import operations in chrome-extension/popup.js
- [ ] T065 [US2] Add config reload after successful import in chrome-extension/popup.js

**Checkpoint**: At this point, User Story 2 should work - advanced users can manage special domains via export/import

---

## Phase 5: User Story 3 - Automatic Fallback for Standard Content (Priority: P3)

**Goal**: System automatically determines extraction method (clipboard vs DOM) for each domain, handling mixed content seamlessly.

**Independent Test**: Bookmark a mix of docs.google.com pages and standard HTML pages, verify each uses appropriate extraction without user intervention.

### Implementation for User Story 3

#### Extraction Method Routing

- [x] T066 [US3] Add extraction method logging to content.js in chrome-extension/content.js
- [x] T067 [US3] Add extraction method to returned data object in chrome-extension/content.js
- [x] T068 [US3] Verify standard DOM extraction path still works in chrome-extension/content.js

#### Domain Matching Edge Cases

- [x] T069 [US3] Test subdomain matching (docs.google.com/document/...) in chrome-extension/config-manager.js
- [x] T070 [US3] Test wildcard pattern matching (*.figma.com) in chrome-extension/config-manager.js
- [x] T071 [US3] Test exact domain matching (docs.google.com) in chrome-extension/config-manager.js

#### Mixed Content Handling

- [x] T072 [US3] Verify background.js handles both extraction methods in chrome-extension/background.js
- [x] T073 [US3] Verify backend accepts extractionMethod parameter as optional in desktop-daemon/src/index.ts
- [x] T074 [US3] Add default value handling for missing extractionMethod in desktop-daemon/src/index.ts

**Checkpoint**: All user stories complete - system handles any combination of special and standard domains

---

## Phase 6: Build System & Packaging (Priority: P3)

**Purpose**: Bundle extension for distribution

- [ ] T075 Configure webpack entry points for all scripts in chrome-extension/webpack.config.js
- [ ] T076 Configure webpack output directory as dist/ in chrome-extension/webpack.config.js
- [ ] T077 Add CopyWebpackPlugin configuration for static files in chrome-extension/webpack.config.js
- [ ] T078 Add copy pattern for manifest.json in chrome-extension/webpack.config.js
- [ ] T079 Add copy pattern for popup.html and popup.css in chrome-extension/webpack.config.js
- [ ] T080 Add copy pattern for config/special-domains-default.json in chrome-extension/webpack.config.js
- [ ] T081 Add copy pattern for images/ directory in chrome-extension/webpack.config.js
- [ ] T082 Add copy pattern for ui/dialogs.css in chrome-extension/webpack.config.js
- [ ] T083 Test npm run build produces working extension in dist/ directory
- [ ] T084 Load dist/ extension in Chrome and verify functionality

---

## Phase 7: Polish & Documentation

**Purpose**: User and developer documentation plus edge case handling

- [ ] T085 [P] Create README.md with installation instructions in chrome-extension/README.md
- [ ] T086 [P] Create CONFIG.md with JSON format documentation in chrome-extension/CONFIG.md
- [ ] T087 [P] Add example configurations to CONFIG.md in chrome-extension/CONFIG.md
- [ ] T088 [P] Add export/import workflow to CONFIG.md in chrome-extension/CONFIG.md
- [ ] T089 [P] Add troubleshooting section to README.md in chrome-extension/README.md
- [ ] T090 [P] Document clipboard permissions in README.md in chrome-extension/README.md
- [ ] T091 [P] Handle config changes while extension is running (cache invalidation) in chrome-extension/config-manager.js
- [ ] T092 [P] Add clipboard size limit detection (5MB threshold) and graceful degradation in chrome-extension/content-clipboard.js
- [ ] T093 [P] Add backend unreachable error handling with retry logic in chrome-extension/background.js
- [ ] T094 [P] Test and validate special character, emoji, and non-Latin script handling in chrome-extension/content-clipboard.js

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-5)**: All depend on Foundational phase completion
  - User stories can proceed in parallel (if staffed) after Phase 2
  - Or sequentially in priority order (US1 → US2 → US3)
- **Build System (Phase 6)**: Depends on all desired user stories being complete
- **Documentation (Phase 7)**: Can proceed in parallel with implementation

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - INDEPENDENT
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - INDEPENDENT (extends US1 but doesn't modify it)
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - INDEPENDENT (validates US1 behavior)

### Within Each User Story

**User Story 1 Dependencies**:
- T013-T021: Clipboard core functions (parallel where marked [P])
- T022-T029: Dialog UI (parallel, independent of clipboard core)
- T030-T033: Error handlers (depend on T013-T029 completing)
- T034-T038: Content script (depends on T005-T011 ConfigManager, T013-T033 clipboard)
- T039-T042: Popup updates (depends on T034-T038)
- T043-T047: Backend (parallel, independent of frontend)

**User Story 2 Dependencies**:
- T048-T050: Export/import methods (parallel)
- T051-T065: Config UI (depends on T048-T050)

**User Story 3 Dependencies**:
- T066-T074: Validation tasks (can run once US1 and US2 complete)

### Parallel Opportunities

- **Phase 1**: All tasks can run in parallel (T001-T004 all marked [P] or independent)
- **Phase 2**: T005-T012 are sequential (ConfigManager methods depend on class structure)
- **User Story 1**: 
  - T013-T021 (clipboard functions) in parallel
  - T022-T029 (dialog UI) in parallel
  - T043-T047 (backend) in parallel with frontend
- **User Story 2**:
  - T048-T050 in parallel
  - T060-T062 (CSS) in parallel
- **Phase 7**: All documentation tasks T085-T090 in parallel

---

## Parallel Example: User Story 1 - Clipboard Core

```bash
# Launch all clipboard utility functions together:
Task: "T013 [P] [US1] Implement checkClipboardPermission()"
Task: "T014 [P] [US1] Implement readClipboard()"
Task: "T015 [P] [US1] Implement writeClipboard()"
Task: "T016 [P] [US1] Implement isContentEmpty()"

# Launch all dialog UI components together:
Task: "T022 [P] [US1] Create ExtractionDialogs class"
Task: "T023 [P] [US1] Implement showPermissionDialog()"
Task: "T024 [P] [US1] Implement showEmptyContentDialog()"
Task: "T025 [P] [US1] Implement showProgressIndicator()"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T004)
2. Complete Phase 2: Foundational (T005-T012) **← BLOCKS everything**
3. Complete Phase 3: User Story 1 (T013-T047)
4. **STOP and VALIDATE**: Test on docs.google.com
5. Build extension (Phase 6: T075-T084)
6. Deploy/demo MVP

**Result**: Users can bookmark from docs.google.com using clipboard extraction with error handling.

### Incremental Delivery

1. **Foundation** (Phases 1-2) → Config system ready
2. **+User Story 1** (Phase 3) → MVP! Basic clipboard extraction works
3. **+User Story 2** (Phase 4) → Power user feature: custom domains
4. **+User Story 3** (Phase 5) → Seamless mixed content handling
5. **Build** (Phase 6) → Package for distribution
6. **Docs** (Phase 7) → User-ready documentation

Each increment adds value without breaking previous functionality.

### Parallel Team Strategy

With multiple developers after Phase 2 completes:

1. **Team completes Setup + Foundational together** (Phases 1-2)
2. **Once Phase 2 done, parallelize**:
   - Developer A: User Story 1 (Phase 3)
   - Developer B: User Story 2 (Phase 4) 
   - Developer C: User Story 3 (Phase 5)
3. **Merge and test** each story independently
4. **Build and document** together (Phases 6-7)

---

## Task Summary

**Total Tasks**: 94
- **Phase 1 (Setup)**: 4 tasks
- **Phase 2 (Foundational)**: 8 tasks (BLOCKING)
- **Phase 3 (US1)**: 35 tasks ← MVP
- **Phase 4 (US2)**: 18 tasks
- **Phase 5 (US3)**: 9 tasks
- **Phase 6 (Build)**: 10 tasks
- **Phase 7 (Docs + Edge Cases)**: 10 tasks

**Parallel Opportunities**: 35 tasks marked [P] can run concurrently

**MVP Scope**: Phases 1-3 + Phase 6 (47 tasks) delivers core value

---

## Notes

- **[P] tasks**: Different files, no dependencies, safe to parallelize
- **[Story] labels**: Track which user story each task belongs to
- **File paths**: Every task includes exact file location
- **Independent stories**: Each user story can be completed and tested independently
- **Checkpoints**: Stop after each phase to validate functionality
- **MVP-ready**: Phase 1-3 delivers working clipboard extraction for docs.google.com
- **Build last**: Phase 6 bundles everything after implementation complete

