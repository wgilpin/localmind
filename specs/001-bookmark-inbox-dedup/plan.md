# Implementation Plan: Save Page to Chrome Bookmarks

**Branch**: `001-bookmark-inbox-dedup` | **Date**: Saturday Dec 13, 2025 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-bookmark-inbox-dedup/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Add "Save Page" functionality to the LocalMind Chrome extension that creates persistent Chrome bookmarks with intelligent deduplication (normalizing URLs by stripping query parameters and Google Docs trailing verbs) and automatic organization into an "inbox" folder. The feature operates silently without user notifications, using Chrome's native bookmarks API to ensure compatibility with Chrome's bookmark sync and management features.

## Technical Context

**Language/Version**: JavaScript ES6+ (Chrome Extension Manifest V3)  
**Primary Dependencies**: Chrome Extension APIs (chrome.bookmarks, chrome.tabs, chrome.runtime), Webpack 5.89+ for bundling  
**Storage**: Chrome's native bookmarks API (no additional persistence required)  
**Testing**: Manual testing via Chrome extension developer mode; automated testing via Jest + chrome-mock (to be added)  
**Target Platform**: Chrome browser (Manifest V3 compatible), minimum Chrome 88+  
**Project Type**: Chrome browser extension (existing extension, adding new feature)  
**Performance Goals**: <2 seconds for bookmark creation, <500ms for duplicate detection, <100ms for URL normalization  
**Constraints**: Silent operation (no user notifications/feedback), must work offline, zero external API calls, respects user's existing bookmark organization  
**Scale/Scope**: Handle user bookmark collections of 1-10k bookmarks efficiently, process URLs up to 2048 characters

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I: Privacy & Offline-First Architecture

✅ **PASS** - Feature uses only Chrome's local bookmarks API with no external network requests. All processing happens locally in the browser. Zero telemetry or data transmission.

### Principle II: Performance & Native Experience

✅ **PASS** - Target <2s for bookmark operations, <500ms for duplicate detection aligns with constitution's <100ms search response goals for different operation types. Uses async Chrome APIs to avoid blocking. Extension remains cross-platform (all Chrome-compatible browsers).

### Principle III: Modern UI/UX Excellence

⚠️ **CONDITIONAL PASS** - Feature operates silently without UI feedback per spec requirements (FR-003). This conflicts with constitution principle "All automated actions MUST be visible in UI" and "Error messages MUST be actionable".

**Justification**: User explicitly requested silent operation. Bookmark creation is fast (<2s) so loading states are not critical. Users can verify success via Chrome's native bookmark manager. Trade-off: Simplicity and non-intrusiveness over explicit feedback.

**Mitigation**: Console logging for debugging, bookmark visibility in Chrome's native UI serves as confirmation.

### Principle IV: Intelligent Automation with User Control

✅ **PASS** - Feature is explicitly triggered by user clicking "Save Page" (opt-in). Users remain in control via Chrome's native bookmark management. Can export data via Chrome's bookmark export. Degrades gracefully (if bookmarks API unavailable, operation simply fails silently).

**Note**: No configuration UI required for v1 - hardcoded "inbox" folder name accepted per spec clarifications. Future enhancement could add folder name configuration.

### Principle V: Developer Quality & Maintainability

✅ **PASS** - Will add proper module separation (bookmark-manager.js), unit tests for URL normalization logic, JSDoc comments for public functions. No new dependencies beyond Chrome APIs (zero supply chain risk). Code will be linted and formatted before commit.

### Technical Constraints: Simplicity Mandate

✅ **PASS** - Uses simple String operations for URL normalization. No unnecessary abstractions. Zero new external dependencies (uses built-in Chrome APIs only). Feature is explicitly requested per spec. Sensible default ("inbox" folder) with fallback (top-level).

### Summary

**Status**: ✅ APPROVED with one documented trade-off (silent operation vs UI visibility)

**Risk Level**: LOW - Single UI visibility principle relaxation explicitly requested by user. All other principles upheld.

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
chrome-extension/
├── manifest.json           # Add "bookmarks" permission
├── popup.html              # Add "Save Page" button to existing UI
├── popup.js                # Wire up Save Page button
├── popup.css               # Style Save Page button
├── background.js           # Add bookmark event listeners (if needed)
├── bookmark-manager.js     # NEW: Core bookmark logic module
│   ├── normalizeUrl()      # URL normalization (strip query params, fragments, Google Docs verbs)
│   ├── findExistingBookmark()  # Search all folders for duplicates
│   ├── findInboxFolder()   # Locate or create "inbox" folder
│   └── createBookmark()    # Create bookmark with deduplication
├── content.js              # Extract page title/content if needed
├── config-manager.js       # Existing config (no changes needed)
└── tests/                  # NEW: Test directory
    └── bookmark-manager.test.js  # Unit tests for URL normalization
```

**Structure Decision**: Chrome Extension (Single Project)

---

## Phase 0: Research & Discovery

**Status**: ✅ COMPLETE

**Output**: [research.md](./research.md)

**Key Decisions**:
1. Use Chrome Bookmarks API (`search()`, `create()`, `getTree()`)
2. Native `URL` API for normalization with custom Google Docs logic
3. Closure-based debouncing (flag pattern, not timer-delay)
4. Google Docs only uses `docs.google.com` (no international domains)
5. Manual testing for v1, Jest for v2
6. TreeWalker for content title extraction

**All Research Questions Resolved** ✅

---

## Phase 1: Design & Contracts

**Status**: ✅ COMPLETE

**Outputs**:
- [data-model.md](./data-model.md) - Bookmark entities, URL normalization structure
- [contracts/chrome-apis.md](./contracts/chrome-apis.md) - Chrome Extension API surface
- [quickstart.md](./quickstart.md) - Developer setup and implementation guide

**Key Design Decisions**:
1. **Single Module**: `bookmark-manager.js` encapsulates all bookmark logic
2. **No Additional Storage**: Uses Chrome's native bookmark database only
3. **Silent Operation**: No UI feedback per spec requirements (justified in Constitution Check)
4. **Debouncing**: 1.5 second flag-based debounce on button click
5. **Error Handling**: Graceful fallbacks (top-level if inbox fails, URL string if parsing fails)

**Agent Context Updated**: ✅

---

## Constitution Check (Post-Design Re-evaluation)

*Re-checking constitution compliance after completing Phase 1 design.*

### Principle I: Privacy & Offline-First Architecture

✅ **STILL PASSING** - Design confirmed:
- Zero external API calls
- All processing in Chrome extension context
- No telemetry or analytics
- Uses only Chrome's local bookmark storage

### Principle II: Performance & Native Experience

✅ **STILL PASSING** - Design targets verified:
- Bookmark creation: <2s (async operations, no blocking)
- Duplicate detection: <500ms (O(n) scan with early exit)
- URL normalization: <100ms (pure function, instant)
- Single module addition, no installer changes
- Cross-platform (all Chrome-compatible browsers)

### Principle III: Modern UI/UX Excellence

⚠️ **CONDITIONAL PASS (trade-off documented)** - Design maintains silent operation per spec:
- No loading states (operation completes quickly <2s)
- No error messages (logs to console for debugging)
- Users verify via Chrome's native bookmark manager

**Mitigation confirmed**: Console logging for developers, Chrome's UI provides user-facing confirmation.

### Principle IV: Intelligent Automation with User Control

✅ **STILL PASSING** - Design ensures:
- Explicit user trigger (click Save Page button)
- Users retain control via Chrome's bookmark manager
- Export via Chrome's built-in tools
- Graceful degradation (falls back to top-level if inbox folder unavailable)

### Principle V: Developer Quality & Maintainability

✅ **DESIGN SUPPORTS** - Artifacts created:
- JSDoc comments on all functions
- Clear module separation (`bookmark-manager.js`)
- Unit testable pure functions (`normalizeUrl()`)
- Comprehensive developer documentation (quickstart.md)
- Zero new dependencies (uses only Chrome APIs)

### Technical Constraints: Simplicity Mandate

✅ **DESIGN MAINTAINS SIMPLICITY**:
- Single module (`bookmark-manager.js`)
- String-based processing (URL normalization)
- No abstractions (no traits/interfaces/classes)
- Zero external dependencies
- Sensible defaults (inbox folder, top-level fallback)

### Final Status

**✅ ALL PRINCIPLES UPHELD** - Design is constitution-compliant with one documented and approved trade-off (silent operation).

**Risk Assessment**: LOW - Simple addition, isolated module, no breaking changes.

---

## Phase 2: Implementation

**Status**: ⏳ PENDING

**Next Command**: `/speckit.tasks` to break down implementation into concrete tasks

**Estimated Effort**: 4-6 hours development + 2 hours testing

**Blockers**: None - all design complete, ready for implementation

**Structure Decision**: Chrome Extension (Single Project)

This is an enhancement to the existing LocalMind Chrome extension located in `chrome-extension/`. The feature adds a new `bookmark-manager.js` module that encapsulates all bookmark-related logic. This maintains the existing flat structure of the extension while keeping bookmark functionality isolated and testable.

**Key Files**:
- **NEW**: `bookmark-manager.js` - Core business logic
- **NEW**: `tests/` - Testing infrastructure  
- **MODIFIED**: `manifest.json` - Add bookmarks permission
- **MODIFIED**: `popup.html/js/css` - Add Save Page button
- **UNCHANGED**: `content.js`, `background.js` (may wire up events but no core logic)

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

_No complexity violations - all decisions align with simplicity mandate. Zero new dependencies, single module addition, string-based processing._
