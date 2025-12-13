# Feature Specification: Save Page to Chrome Bookmarks

**Feature Branch**: `001-bookmark-inbox-dedup`  
**Created**: Saturday Dec 13, 2025  
**Status**: Draft  
**Input**: User description: "When a doc is added using the chrome extension, check if a bookmark already exists (ignoring query params, and for docs.google.com ignoring the verb at the end if present like /edit)> If it exists, carry on. If not, add the bookmark to the folder `inbox` if present, if not add to top level. If it's possible, create the bm folder inbox if not present"

## Clarifications

### Session 2025-12-13

- Q: What happens when a bookmark with the same normalized URL already exists in a different folder (e.g., existing bookmark in "Work" folder, user tries to save to "inbox")? → A: Skip creation entirely and continue with extension's existing logic. Goal is ensuring at least one bookmark exists, not managing organization.
- Q: Should URL fragments (hash anchors like `#section-1`) be included or excluded in duplicate detection and saved bookmarks? → A: Exclude fragments entirely - strip them from both duplicate detection and saved bookmarks.
- Q: What happens if the user clicks "Save Page" rapidly multiple times? → A: Debounce the button - ignore subsequent clicks for 1.5 seconds after the first click.
- Q: How should the system handle pages with no title or empty title? → A: Use the first line or first few words of the document content as the bookmark title.
- Q: What if the user has multiple "inbox" folders in different locations? → A: Use the first "inbox" folder found at the root level; ignore nested ones.
- Q: Should the system provide user feedback for bookmark operations? → A: No - operate silently without user notifications or feedback messages.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Save Page as Persistent Chrome Bookmark (Priority: P1)

When a user clicks "Save Page" in the Chrome extension UI, the system should create a persistent Chrome bookmark for the current page. This allows users to save pages they want to keep for later reference, making them accessible through Chrome's native bookmark system.

**Why this priority**: This is the core feature - allowing users to save pages as Chrome bookmarks. Without this, the extension cannot fulfill its primary purpose.

**Independent Test**: Can be fully tested by clicking "Save Page" on any web page and verifying a Chrome bookmark is created and visible in Chrome's bookmark manager.

**Acceptance Scenarios**:

1. **Given** user is viewing a web page, **When** user clicks "Save Page" in the extension, **Then** a Chrome bookmark is created with the page's URL and title
2. **Given** user is viewing a Google Docs page, **When** user clicks "Save Page", **Then** a Chrome bookmark is created with the document title and URL
3. **Given** user has saved a page successfully, **When** user opens Chrome's bookmark manager, **Then** the saved bookmark is visible and clickable
4. **Given** user saves a page, **When** user clicks the saved bookmark in Chrome, **Then** the original page loads correctly

---

### User Story 2 - Auto-organize Bookmarks into Inbox Folder (Priority: P2)

When a user saves a page, the system should automatically place the bookmark in an "inbox" folder to help users maintain organized bookmarks. This provides a default location for incoming bookmarks that users can later triage.

**Why this priority**: Improves user experience by providing automatic organization, but the core feature (saving bookmarks) works without it.

**Independent Test**: Can be tested independently by saving a page when an "inbox" folder exists and verifying the bookmark appears in that folder rather than at the top level.

**Acceptance Scenarios**:

1. **Given** an "inbox" folder exists in bookmarks, **When** user saves a page, **Then** the bookmark is placed inside the "inbox" folder
2. **Given** no "inbox" folder exists, **When** user saves a page, **Then** the bookmark is placed at the top level of bookmarks
3. **Given** the "inbox" folder was just created by the system, **When** user saves a page, **Then** the bookmark is placed inside the newly created "inbox" folder

---

### User Story 3 - Prevent Duplicate Bookmarks with Smart URL Matching (Priority: P3)

When a user saves a page, the system should detect if the same document is already bookmarked by comparing normalized URLs (ignoring query parameters and Google Docs verbs). If a duplicate exists, the system should skip creation silently.

**Why this priority**: Enhances user experience by preventing clutter, but the core feature works without it. Users can manage duplicates manually if needed.

**Independent Test**: Can be tested by attempting to save the same URL multiple times with different query parameters and verifying that only one bookmark exists after multiple save attempts.

**Acceptance Scenarios**:

1. **Given** a bookmark exists for `https://example.com/document`, **When** user saves `https://example.com/document?session=xyz`, **Then** no new bookmark is created (operation completes silently)
2. **Given** a bookmark exists for `https://docs.google.com/document/d/ABC123/edit`, **When** user saves `https://docs.google.com/document/d/ABC123/view`, **Then** no new bookmark is created (operation completes silently)
3. **Given** a bookmark exists for `https://docs.google.com/document/d/ABC123`, **When** user saves `https://docs.google.com/document/d/ABC123/edit`, **Then** no new bookmark is created (operation completes silently)
4. **Given** no bookmark exists for a URL, **When** user saves the page, **Then** a new bookmark is created (operation completes silently)

---

### User Story 4 - Auto-create Inbox Folder When Missing (Priority: P4)

When a user saves a page and no "inbox" folder exists, the system should attempt to create one automatically. This provides a better first-time experience and ensures consistent organization.

**Why this priority**: Nice-to-have enhancement that improves onboarding, but users can manually create the folder if needed.

**Independent Test**: Can be tested by removing the "inbox" folder, saving a page, and verifying that an "inbox" folder was created with the bookmark inside.

**Acceptance Scenarios**:

1. **Given** no "inbox" folder exists and browser API allows folder creation, **When** user saves a page, **Then** an "inbox" folder is created and the bookmark is placed inside
2. **Given** no "inbox" folder exists and browser API prevents folder creation, **When** user saves a page, **Then** the bookmark is placed at the top level (operation completes silently)

---

### Edge Cases

- What happens when the user clicks "Save Page" while the browser is offline?
- How does the system handle pages with dynamic titles that change after the bookmark is created?
- What happens when Chrome's bookmark API is temporarily unavailable?
- What if the browser bookmark storage is full or quota is exceeded?
- How does the system handle very long URLs or titles that might exceed browser limits?
- What happens if the user clicks "Save Page" rapidly multiple times? → Button is debounced to ignore subsequent clicks for 1.5 seconds (silent operation)
- How does the system handle pages with no title or empty title? → Use the first line or first few words of the document content as the bookmark title
- What happens when a bookmark exists with the same normalized URL but in a different folder? → Skip creation entirely; at least one bookmark exists, which satisfies the requirement
- What about other Google productivity URLs (sheets.google.com, slides.google.com, forms.google.com)?
- How does the system handle URL fragments (e.g., `#section-1`)? → Fragments are stripped entirely from both duplicate detection and saved bookmark URLs
- What if the user has multiple "inbox" folders in different locations? → Use the first "inbox" folder found at root level; ignore nested ones

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST create a persistent Chrome bookmark when user clicks "Save Page" in the extension UI
- **FR-002**: System MUST capture the current page's URL and title when creating a bookmark
- **FR-002a**: System MUST use the first line or first few words of the document content as the bookmark title when the page title is missing or empty
- **FR-003**: System MUST operate silently without providing user feedback for bookmark creation or duplicate detection
- **FR-003a**: System MUST debounce the "Save Page" button to ignore subsequent clicks for 1.5 seconds after the first click
- **FR-004**: System MUST add new bookmarks to a folder named "inbox" if one exists
- **FR-005**: System MUST search for the "inbox" folder case-insensitively at the root level only (ignore nested folders)
- **FR-005a**: System MUST use the first "inbox" folder found at root level if multiple exist
- **FR-006**: System MUST add bookmarks to the top level of bookmarks if no "inbox" folder exists
- **FR-007**: System MUST attempt to create an "inbox" folder if none exists and the browser API allows it
- **FR-008**: System MUST check if a bookmark already exists before creating a new one (deduplication)
- **FR-009**: System MUST normalize URLs by removing all query parameters (everything after `?`) when comparing for duplicates
- **FR-010**: System MUST normalize Google Docs URLs (docs.google.com) by removing trailing verbs (`/edit`, `/view`, `/preview`, `/copy`) when comparing for duplicates
- **FR-011**: System MUST check for existing bookmarks across all folders, not just the target folder
- **FR-011a**: System MUST skip bookmark creation entirely if a matching normalized URL exists in any folder (regardless of which folder it's in)
- **FR-012**: System MUST strip URL fragments (hash anchors like `#section-1`) from both duplicate detection and saved bookmark URLs
- **FR-013**: System MUST handle Google Docs URLs (docs.google.com only - international domains do not exist for Google Docs)
- **FR-014**: System MUST silently skip bookmark creation when a duplicate is detected (no user notification required)
- **FR-015**: System MUST handle bookmark creation errors gracefully by logging errors silently without user-facing messages

### Key Entities

- **Saved Page**: Represents the current web page being saved, including its URL and title
- **Chrome Bookmark**: A persistent browser bookmark with a URL, title, and folder location that appears in Chrome's native bookmark system
- **Inbox Folder**: A special bookmark folder with the name "inbox" (case-insensitive) that serves as the default destination for new bookmarks
- **Normalized URL**: The canonical form of a URL used for duplicate detection, consisting of the protocol, domain, and path, with query parameters, URL fragments, and Google Docs trailing verbs removed

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of "Save Page" actions successfully create a Chrome bookmark when no duplicate exists
- **SC-002**: Bookmarks are created and visible in Chrome's bookmark manager within 2 seconds of clicking "Save Page"
- **SC-003**: At least 90% of new bookmarks are automatically organized into the "inbox" folder when one exists
- **SC-004**: 100% of duplicate URLs are detected when URLs differ only by query parameters
- **SC-005**: 100% of duplicate Google Docs URLs are detected when URLs differ only by trailing verbs (`/edit`, `/view`, etc.)
- **SC-006**: Operation completes silently without disrupting user workflow

## Assumptions

- The Chrome extension has permission to access and modify the browser's bookmark system
- Users want their saved pages to persist as native Chrome bookmarks (not just in extension storage)
- The "Save Page" button/action is clearly visible in the extension UI
- The browser's bookmarks API supports creating, searching, and organizing bookmarks
- Users want a single "inbox" folder at the root level (not nested inside other folders); if multiple exist at root level, the first one found is used
- The hardcoded folder name "inbox" (lowercase) is acceptable for all users regardless of language or locale
- Users can manually rename the "inbox" folder in Chrome's bookmark manager if desired, and the extension will continue to use the original/renamed folder
- URL normalization is sufficient for duplicate detection (title differences are ignored)
- For Google Docs URLs, only the document ID portion matters for duplicate detection
- Users primarily save pages from their Chrome browser (not mobile or other browsers)
- Users value having their bookmarks accessible through Chrome's native bookmark system and sync

## Out of Scope

- Editing or modifying existing bookmarks after creation
- Removing existing duplicate bookmarks that were created before this feature
- Providing a custom UI for browsing or managing Chrome bookmarks (users should use Chrome's native bookmark manager)
- Merging or syncing bookmarks across multiple browsers or devices (handled by Chrome sync)
- Organizing bookmarks into folders other than "inbox" based on content, tags, or AI categorization
- Detecting duplicates based on page title or content similarity (only URL-based deduplication)
- Handling bookmark tags or other metadata beyond URL, title, and folder location
- Providing bulk import, export, or deduplication of existing bookmarks
- Custom folder naming or configuration UI for this version
- Auto-archiving or moving bookmarks out of the inbox folder after a certain time
- Offline operation handling (Chrome queues API calls automatically when offline; no special handling needed)
- Other Google productivity URLs (sheets.google.com, slides.google.com, forms.google.com) - deferred to v2
