# Tasks: Save Page to Chrome Bookmarks

**Feature Branch**: `001-bookmark-inbox-dedup`  
**Input**: Design documents from `/specs/001-bookmark-inbox-dedup/`  
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Manual testing only - no automated test tasks (not requested in spec)

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `- [ ] [ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

- **Chrome Extension**: `chrome-extension/` at repository root
- All JavaScript files use ES6+ syntax
- No bundling required for v1 (direct script loading)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and permissions setup

- [x] T001 Add "bookmarks" permission to chrome-extension/manifest.json
- [x] T002 [P] Create chrome-extension/bookmark-manager.js file with module structure and JSDoc template
- [x] T003 [P] Add debounce helper function to chrome-extension/popup.js

**Checkpoint**: Extension has bookmark permission, core files created

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: None - this feature has no blocking prerequisites beyond setup

**Note**: All user stories can begin immediately after Phase 1 setup

---

## Phase 3: User Story 1 - Save Page as Persistent Chrome Bookmark (Priority: P1) 🎯 MVP

**Goal**: User can click "Save Page" button and create a Chrome bookmark with the current page's URL and title

**Independent Test**: 
1. Click Save Page on any website → Bookmark created
2. Open chrome://bookmarks/ → New bookmark visible
3. Click saved bookmark → Original page loads

### Implementation for User Story 1

- [x] T004 [US1] Implement savePageAsBookmark() main function in chrome-extension/bookmark-manager.js (basic version without dedup/inbox)
- [x] T005 [US1] Implement getPageTitle() function with tab.title fallback in chrome-extension/bookmark-manager.js
- [x] T006 [US1] Implement TreeWalker content extraction as fallback in getPageTitle() in chrome-extension/bookmark-manager.js
- [x] T007 [P] [US1] Add Save Page button HTML to chrome-extension/popup.html (after existing buttons)
- [x] T008 [P] [US1] Add button styles to chrome-extension/popup.css
- [x] T009 [US1] Wire up Save Page button click handler with debouncing in chrome-extension/popup.js
- [x] T010 [US1] Add script tag for bookmark-manager.js to chrome-extension/popup.html
- [ ] T011 [US1] Test: Reload extension → Click Save Page on test site → Verify bookmark created in Chrome bookmark manager
- [ ] T012 [US1] Test: Save page with normal title → Uses page title
- [ ] T013 [US1] Test: Save page with empty title → Uses content fallback or 'Untitled'
- [ ] T014 [US1] Test: Click button rapidly 5x → Only one bookmark created (debouncing works)

**Checkpoint**: User Story 1 is fully functional - users can save pages as bookmarks

---

## Phase 4: User Story 2 - Auto-organize Bookmarks into Inbox Folder (Priority: P2)

**Goal**: Saved bookmarks automatically appear in "inbox" folder if it exists, otherwise at top level

**Independent Test**:
1. Create "inbox" folder in Chrome bookmarks
2. Save page → Bookmark appears in inbox folder
3. Delete inbox folder → Save page → Bookmark appears at top level

### Implementation for User Story 2

- [x] T015 [US2] Implement findInboxFolder() function in chrome-extension/bookmark-manager.js (search root level, case-insensitive)
- [x] T016 [US2] Update savePageAsBookmark() to use findInboxFolder() for parentId selection in chrome-extension/bookmark-manager.js
- [x] T017 [US2] Add fallback to parentId '1' (Bookmark Bar) if inbox not found in chrome-extension/bookmark-manager.js
- [ ] T018 [US2] Test: Create "inbox" folder → Save page → Verify bookmark in inbox folder
- [ ] T019 [US2] Test: Create "INBOX" folder (uppercase) → Save page → Verify case-insensitive match works
- [ ] T020 [US2] Test: Delete inbox folder → Save page → Verify bookmark at top level (Bookmark Bar)
- [ ] T021 [US2] Test: Create nested "Folder/inbox" → Save page → Should use root-level inbox or create one (depends on US4)

**Checkpoint**: User Story 2 is fully functional - bookmarks auto-organize into inbox folder

---

## Phase 5: User Story 3 - Prevent Duplicate Bookmarks with Smart URL Matching (Priority: P3)

**Goal**: Duplicate pages (same URL with different query params or Google Docs verbs) are detected and skipped

**Independent Test**:
1. Save example.com → Bookmark created
2. Save example.com?foo=bar → No new bookmark (duplicate detected)
3. Save example.com#section → No new bookmark (duplicate detected)
4. Verify only one example.com bookmark exists

### Implementation for User Story 3

- [x] T022 [P] [US3] Implement normalizeUrl() function in chrome-extension/bookmark-manager.js (strip query params and fragments)
- [x] T023 [P] [US3] Add Google Docs verb removal logic to normalizeUrl() in chrome-extension/bookmark-manager.js
- [x] T024 [US3] Implement findExistingBookmark() function in chrome-extension/bookmark-manager.js (search all bookmarks with normalization)
- [x] T025 [US3] Update savePageAsBookmark() to check for duplicates before creating in chrome-extension/bookmark-manager.js
- [x] T026 [US3] Add console.log for duplicate detection (silent skip per spec) in chrome-extension/bookmark-manager.js
- [ ] T027 [US3] Test: Save example.com → Bookmark created
- [ ] T028 [US3] Test: Save example.com?foo=bar → No new bookmark (duplicate detected)
- [ ] T029 [US3] Test: Save example.com#section → No new bookmark (duplicate detected)
- [ ] T030 [US3] Test: Open Google Doc /edit URL → Save → Save again with /view URL → Only one bookmark exists
- [ ] T031 [US3] Test: Save docs.google.com/document/d/ABC123/edit?usp=sharing#h.123 → Verify all normalization works (query, fragment, verb)

**Checkpoint**: User Story 3 is fully functional - duplicates are detected and skipped

---

## Phase 6: User Story 4 - Auto-create Inbox Folder When Missing (Priority: P4)

**Goal**: If inbox folder doesn't exist, system attempts to create it automatically

**Independent Test**:
1. Delete "inbox" folder from bookmarks
2. Save page → Inbox folder created automatically
3. Verify bookmark is inside newly created inbox folder

### Implementation for User Story 4

- [x] T032 [US4] Implement createInboxFolder() function in chrome-extension/bookmark-manager.js (with error handling)
- [x] T033 [US4] Update savePageAsBookmark() to call createInboxFolder() if findInboxFolder() returns null in chrome-extension/bookmark-manager.js
- [x] T034 [US4] Add error handling and fallback to top-level if folder creation fails in chrome-extension/bookmark-manager.js
- [ ] T035 [US4] Test: Delete inbox folder → Save page → Verify inbox folder created with bookmark inside
- [ ] T036 [US4] Test: Save another page → Verify uses existing inbox folder (not creating duplicate folders)
- [ ] T037 [US4] Test: Simulate folder creation failure → Verify fallback to top-level works gracefully

**Checkpoint**: User Story 4 is fully functional - inbox folder auto-creation works

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Code quality, documentation, and final validation

- [x] T038 [P] Add JSDoc comments to all functions in chrome-extension/bookmark-manager.js
- [x] T039 [P] Add inline comments for complex logic (URL normalization, TreeWalker) in chrome-extension/bookmark-manager.js
- [x] T040 [P] Add console.error logging for all error paths in chrome-extension/bookmark-manager.js
- [ ] T041 Review code against quickstart.md implementation guide
- [ ] T042 Test performance: Save bookmark with 5000 existing bookmarks → Should complete in <2s
- [ ] T043 Test error handling: Save on restricted page (chrome://extensions/) → Graceful failure
- [ ] T044 Test edge case: Save page with very long URL (2000 chars) → Should handle gracefully
- [ ] T045 Test edge case: Save page with very long title (300 chars) → Should truncate to 255 chars
- [x] T046 [P] Update manifest.json version from 1.0 to 1.1.0
- [ ] T047 Final validation: Run through all test scenarios in quickstart.md
- [ ] T048 Code cleanup: Remove any debug console.logs, format code consistently

**Checkpoint**: Feature is production-ready

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: N/A - no foundational tasks required
- **User Stories (Phase 3-6)**: All depend on Setup (Phase 1) completion
  - User Story 1 (P1): Can start after Setup - No dependencies on other stories
  - User Story 2 (P2): Can start after Setup OR after US1 - Integrates with US1 but independently testable
  - User Story 3 (P3): Can start after Setup OR after US1 - Integrates with US1 but independently testable
  - User Story 4 (P4): Depends on User Story 2 (needs findInboxFolder) - Should complete US2 first
- **Polish (Phase 7)**: Depends on all user stories being complete

### User Story Dependencies

```
Setup (Phase 1)
    ↓
User Story 1 (P1) ← MVP
    ↓ (integrates with, but independently testable)
User Story 2 (P2) ← Can start after Setup if preferred
    ↓
User Story 4 (P4) ← Depends on US2's findInboxFolder function
    
User Story 3 (P3) ← Can start after Setup, integrates with US1
```

**Key Insight**: US2, US3, and US4 all integrate with US1's savePageAsBookmark() function, but should be independently testable by verifying their specific behavior (folder organization, deduplication, folder creation).

### Within Each User Story

- Implementation tasks before testing tasks
- Core logic before UI changes (where applicable)
- Integration with main function as last implementation step
- Manual testing after implementation complete

### Parallel Opportunities

**Phase 1 Setup**:
- T002 (create bookmark-manager.js) and T003 (add debounce helper) can run in parallel

**Phase 3 (User Story 1)**:
- T007 (add button HTML) and T008 (add button styles) can run in parallel
- T004-T006 (bookmark functions) should complete before T009 (wire up button)

**Phase 7 (Polish)**:
- T038 (JSDoc), T039 (comments), T040 (logging), T046 (version bump) can all run in parallel

**Multi-Story Parallelism** (if team capacity allows):
- After Setup (Phase 1), a team could work on US1, US2, and US3 in parallel, then merge them together
- However, for solo development, sequential execution (P1 → P2 → P3 → P4) is recommended

---

## Parallel Example: User Story 1

```bash
# These can be launched together (different files):
Task T007: "Add Save Page button HTML to chrome-extension/popup.html"
Task T008: "Add button styles to chrome-extension/popup.css"

# These must be sequential (same file, building on each other):
Task T004: "Implement savePageAsBookmark() basic version"
Task T005: "Implement getPageTitle()"
Task T006: "Implement TreeWalker fallback"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

**Goal**: Ship minimal working feature quickly

1. Complete Phase 1: Setup (T001-T003) - ~30 mins
2. Complete Phase 3: User Story 1 (T004-T014) - ~2-3 hours
3. **STOP and VALIDATE**: 
   - Reload extension
   - Click Save Page on 5 different sites
   - Verify all bookmarks appear in Chrome bookmark manager
   - Test debouncing (rapid clicks)
   - Test title extraction (normal and empty titles)
4. If validation passes, User Story 1 is shippable as MVP ✅

**Why This Works**: User Story 1 provides complete value - users can save pages as bookmarks. All other stories are enhancements.

### Incremental Delivery

**Iteration 1 - MVP (User Story 1)**:
- Duration: ~3 hours
- Delivers: Basic bookmark saving
- Test: Manual validation per quickstart.md
- Deploy: Load in Chrome, use immediately

**Iteration 2 - Organization (User Story 2)**:
- Duration: ~1 hour
- Delivers: Auto-organization into inbox folder
- Test: Create inbox folder, save pages, verify organization
- Deploy: Reload extension
- **Value Add**: Bookmarks now auto-organize, reducing clutter

**Iteration 3 - Deduplication (User Story 3)**:
- Duration: ~1.5 hours
- Delivers: Smart duplicate detection
- Test: Save same URL with different params, verify dedup
- Deploy: Reload extension
- **Value Add**: No more duplicate bookmarks

**Iteration 4 - Automation (User Story 4)**:
- Duration: ~30 mins
- Delivers: Auto-create inbox folder
- Test: Delete inbox, save page, verify folder created
- Deploy: Reload extension
- **Value Add**: Seamless first-time experience

**Iteration 5 - Polish (Phase 7)**:
- Duration: ~1 hour
- Delivers: Documentation, performance validation, edge case handling
- Deploy: Final production release

**Total Estimated Time**: 6-8 hours (matches plan.md estimate)

### Parallel Team Strategy

**Not recommended for this feature** - Single developer can complete in 6-8 hours. If team is available:

1. **Week 1, Day 1**: Developer A completes Setup (Phase 1)
2. **Week 1, Day 2-3**: 
   - Developer A: User Story 1 (T004-T014)
   - Developer B: Start User Story 3 (T022-T024) in parallel (URL normalization functions)
   - Developer C: Start User Story 2 (T015-T017) in parallel (folder finding logic)
3. **Week 1, Day 4**: Integration
   - Merge all branches
   - Developer A integrates US2 and US3 into US1's savePageAsBookmark()
   - All developers test together
4. **Week 1, Day 5**: Polish (Phase 7) - all developers collaborate

**Trade-off**: Parallel development adds communication overhead for a small feature. Sequential development by one person is likely faster.

---

## Implementation Workflow

### For Solo Developer (Recommended)

```bash
# Start on feature branch
git checkout 001-bookmark-inbox-dedup

# Phase 1: Setup
# T001: Edit manifest.json
# T002: Create bookmark-manager.js
# T003: Add debounce to popup.js
git add chrome-extension/
git commit -m "Setup: Add bookmarks permission and create core files"

# Phase 3: User Story 1 (MVP)
# T004-T006: Implement bookmark functions
# T007-T008: Add UI button
# T009-T010: Wire up button
git commit -m "Implement User Story 1: Basic bookmark saving"

# T011-T014: Manual testing
# Document test results

# Phase 4: User Story 2
# T015-T017: Folder finding logic
git commit -m "Implement User Story 2: Inbox folder organization"

# T018-T021: Manual testing

# Phase 5: User Story 3
# T022-T026: URL normalization and deduplication
git commit -m "Implement User Story 3: Duplicate detection"

# T027-T031: Manual testing

# Phase 6: User Story 4
# T032-T034: Folder auto-creation
git commit -m "Implement User Story 4: Auto-create inbox folder"

# T035-T037: Manual testing

# Phase 7: Polish
# T038-T048: Documentation, cleanup, final validation
git commit -m "Polish: Add documentation and final cleanup"

# Final validation
# Run through quickstart.md test checklist
# Performance testing
# Edge case testing

# Ready for review/merge
git push origin 001-bookmark-inbox-dedup
```

### Commit Strategy

- Commit after each user story phase completes
- Commit after passing manual tests
- Keep commits atomic and descriptive
- Reference task IDs in commit messages

### Testing Strategy

**Manual Testing** (per quickstart.md):
- Basic functionality (T011-T014)
- Inbox folder organization (T018-T021)
- Deduplication (T027-T031)
- Folder auto-creation (T035-T037)
- Performance and edge cases (T042-T045)

**No Automated Tests** for v1 per spec requirements. Future enhancement could add Jest unit tests for URL normalization.

---

## Task Summary

**Total Tasks**: 48 tasks
- Phase 1 (Setup): 3 tasks
- Phase 2 (Foundational): 0 tasks (no blocking prerequisites)
- Phase 3 (User Story 1 - P1): 11 tasks
- Phase 4 (User Story 2 - P2): 7 tasks
- Phase 5 (User Story 3 - P3): 10 tasks
- Phase 6 (User Story 4 - P4): 6 tasks
- Phase 7 (Polish): 11 tasks

**Parallel Opportunities**: 6 tasks marked [P] (12.5% of tasks)

**Independent Test Criteria**:
- ✅ User Story 1: Save page → bookmark created
- ✅ User Story 2: Save page → bookmark in inbox folder
- ✅ User Story 3: Save duplicate URL → no new bookmark
- ✅ User Story 4: Delete inbox → save page → inbox created

**Suggested MVP Scope**: Phase 1 (Setup) + Phase 3 (User Story 1) = 14 tasks, ~3 hours

**Full Feature Completion**: All 48 tasks, ~6-8 hours

---

## Notes

- All tasks follow strict checklist format: `- [ ] [ID] [P?] [Story] Description with path`
- Each user story is independently testable
- No automated tests included (not requested in spec)
- Manual testing tasks included after each user story implementation
- Stop at any checkpoint to validate story independently
- Extension can be loaded and tested at any point after Phase 1
- Each user story adds value incrementally
- Silent operation per spec (no UI feedback except console logging)
- Zero external dependencies (uses only Chrome APIs)
- Estimated completion: 6-8 hours for complete feature

---

## Validation Checklist

Before considering this feature complete:

- [ ] All 48 tasks marked complete
- [ ] Extension loads without errors in chrome://extensions/
- [ ] All user stories tested independently per their test criteria
- [ ] All quickstart.md test scenarios pass
- [ ] Performance targets met (<2s bookmark creation, <500ms dedup)
- [ ] Edge cases handled gracefully
- [ ] Code has JSDoc comments on all functions
- [ ] Console logging added for debugging
- [ ] No implementation details leak into user-facing UI (silent operation maintained)
- [ ] manifest.json version updated to 1.1.0
- [ ] Ready for code review and merge
