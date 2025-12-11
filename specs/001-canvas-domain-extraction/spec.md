# Feature Specification: Canvas-Based Domain Content Extraction

**Feature Branch**: `001-canvas-domain-extraction`  
**Created**: December 11, 2025  
**Status**: Draft  
**Input**: User description: "when a user attempts to add a bm for one of a configured list of special domains, including docs.google.com, the content needs to be extracted by the chrome extension using select all, copy as this domain uses canvas for all content and therefore can't be accessed by normal tokio operations. That content then needs to be passed to the backend for indexing etc. We dont want to have to implement oauth for every possible such site. The config doesn't need a UI - it should be in a config file that an advanced user could edit if they want."

## Clarifications

### Session 2025-12-11

- Q: What format should the special domains configuration file use? → A: JSON (stored in browser extension storage, native support, no extra dependencies)
- Q: Where should the special domains configuration file be located? → A: Extension storage directory
- Q: What should happen when clipboard permissions are denied or unavailable? → A: Show error with options: change permission or fallback to standard extraction
- Q: How should the system ensure content is fully loaded before extraction? → A: Not applicable - extraction is only triggered by explicit user click on bookmark button, ensuring user sees content as ready
- Q: What should happen when clipboard extraction returns empty or minimal content? → A: Warn and allow save - alert user, offer retry or save anyway

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Add Bookmark from Canvas-Rendered Domain (Priority: P1)

A user wants to bookmark and index content from a Google Doc (or similar canvas-rendered site). When they add the bookmark, the system automatically detects that this is a special domain requiring alternative content extraction, copies all visible content, and sends it to the backend for indexing - all without requiring the user to authenticate via OAuth or take any special action.

**Why this priority**: This is the core functionality. Without it, users cannot bookmark and search content from canvas-rendered sites like Google Docs, which is critical for knowledge management workflows.

**Independent Test**: Can be fully tested by installing the extension, navigating to docs.google.com, clicking the bookmark button, and verifying that the content is extracted and indexed in the backend without errors.

**Acceptance Scenarios**:

1. **Given** a user is viewing a Google Doc in their browser, **When** they click to add a bookmark, **Then** the extension detects this is a special domain, extracts content via clipboard operations, and successfully sends it to the backend for indexing
2. **Given** a user is on a special domain with complex formatting, **When** content is extracted, **Then** all visible text content is captured and preserved
3. **Given** a user has just added a bookmark from a special domain, **When** they search for content from that page, **Then** the indexed content is searchable and returns relevant results
4. **Given** a user attempts to bookmark a special domain but clipboard permissions are denied, **When** the extraction fails, **Then** system displays error dialog with options to grant permissions or use fallback extraction, and user can choose either option to proceed
5. **Given** a user clicks to bookmark a special domain before content fully renders, **When** clipboard extraction returns empty or minimal content, **Then** system displays warning and offers options to retry or save anyway, allowing user to make informed decision

---

### User Story 2 - Advanced User Configures Additional Special Domains (Priority: P2)

An advanced user discovers a new canvas-based site (e.g., Figma editor, Miro boards) that requires special content extraction. They open the configuration file, add the domain to the list using the documented format, and restart the extension. The system now automatically handles content extraction for this new domain.

**Why this priority**: Extends the feature to support user-discovered sites without requiring code changes. Important for power users but not critical for basic functionality.

**Independent Test**: Can be tested by modifying the config file to add a test domain, restarting the extension, and verifying that the domain is now treated as a special domain requiring clipboard extraction.

**Acceptance Scenarios**:

1. **Given** an advanced user has identified a new canvas-rendered domain, **When** they add it to the configuration file and restart the extension, **Then** bookmarks from that domain use clipboard-based extraction
2. **Given** a user has modified the config file with an invalid domain format, **When** the extension loads, **Then** clear error messaging indicates which configuration entry is invalid
3. **Given** a user wants to remove a domain from special handling, **When** they remove it from the config file and restart, **Then** that domain reverts to standard extraction methods

---

### User Story 3 - Automatic Fallback for Standard Content (Priority: P3)

A user bookmarks a mix of pages - some from special canvas-rendered domains and some from standard HTML sites. The system automatically determines which extraction method to use for each domain, ensuring all bookmarks work correctly regardless of the underlying rendering technology.

**Why this priority**: Provides seamless experience across different site types. Less critical as it's about refinement rather than core functionality.

**Independent Test**: Can be tested by bookmarking both a Google Doc and a standard website, then verifying that both are indexed correctly using their appropriate extraction methods.

**Acceptance Scenarios**:

1. **Given** a user bookmarks pages from both special domains (like docs.google.com) and standard domains, **When** content is extracted, **Then** each uses the appropriate method without user intervention
2. **Given** a user is on a subdomain of a special domain (e.g., docs.google.com/document/...), **When** they add a bookmark, **Then** the domain matching correctly identifies it as a special domain
3. **Given** a special domain starts supporting standard DOM access, **When** the user removes it from the config, **Then** the system seamlessly switches to standard extraction without data loss

---

### Edge Cases

- When a user attempts to bookmark a special domain but has disabled clipboard access in browser permissions, system presents error dialog with options to grant permissions or fallback to standard extraction
- Content loading timing is managed by user - extraction only occurs when user explicitly clicks bookmark button, ensuring they see content as ready before triggering extraction
- When clipboard extraction returns empty or minimal content (e.g., user clicked before any content rendered), system displays warning and offers user option to retry extraction or save bookmark anyway
- When content is behind authentication walls and isn't visible until login, system will capture whatever is visible (typically login prompt or empty content) - user responsibility to be logged in before bookmarking
- When a domain is added to the config file while the extension is running, config cache will be invalidated and reloaded on next bookmark operation
- When documents exceed clipboard size limits (5MB threshold), system provides graceful degradation with size warning and fallback option
- When backend is unreachable during content submission, system provides retry logic with clear error messaging
- System handles special characters, emojis, and non-Latin scripts in extracted content via native JavaScript string encoding

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST maintain a JSON configuration file in the extension storage directory listing domains that require clipboard-based content extraction
- **FR-002**: System MUST include docs.google.com in the default configuration of special domains
- **FR-003**: System MUST detect when a bookmark is being created from a domain in the special domains list
- **FR-003a**: Content extraction MUST only be triggered by explicit user action (clicking bookmark button), ensuring user controls timing
- **FR-004**: Extension MUST use "select all" and "copy" operations to extract content from special domains
- **FR-005**: Extension MUST capture clipboard content after the copy operation completes
- **FR-005a**: When clipboard extraction returns empty or minimal content (below reasonable threshold), system MUST display warning to user with options to retry extraction or proceed with save anyway
- **FR-006**: System MUST send extracted content to the backend for indexing in the same format as standard extraction
- **FR-007**: JSON configuration file MUST be human-readable and editable by advanced users through export/import functionality
- **FR-007a**: System MUST provide export/import UI for configuration file accessible from extension popup
- **FR-008**: Configuration file MUST support domain matching including subdomains (e.g., *.google.com pattern or exact domain matching)
- **FR-009**: System MUST validate JSON configuration file syntax and domain entries, logging errors for invalid formats
- **FR-010**: Extension MUST restore previous clipboard contents after extraction completes
- **FR-011**: System MUST handle cases where clipboard operations fail gracefully by presenting user with clear error message and two options: grant clipboard permissions or fallback to standard DOM-based extraction
- **FR-011a**: When user chooses fallback option, system MUST attempt standard extraction method even for special domains
- **FR-012**: System MUST NOT require OAuth authentication for special domain content extraction
- **FR-013**: Extension MUST indicate to users when content extraction is in progress for special domains
- **FR-014**: System MUST associate extracted content with the correct bookmark URL and metadata

### Key Entities

- **Special Domain Configuration**: JSON-formatted list of domain patterns that require alternative content extraction. Attributes include domain pattern (exact or wildcard), optional description, enabled/disabled status. Stored natively in browser extension storage for fast access.
- **Bookmark**: Existing bookmark entity extended to track extraction method used (standard vs clipboard-based)
- **Extracted Content**: Text content captured from special domains, associated with bookmark metadata (URL, title, timestamp, extraction method)

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can successfully bookmark and index content from docs.google.com and other canvas-rendered domains within 3 seconds
- **SC-002**: Content extraction succeeds for at least 95% of special domain bookmarks
- **SC-003**: Extracted content from special domains is searchable with same accuracy as standard HTML content
- **SC-004**: Advanced users can add a new special domain to the configuration and have it working within 2 minutes (edit config + restart)
- **SC-005**: System handles bookmarking mixed content (special and standard domains) without user awareness of extraction method differences
- **SC-006**: Zero OAuth implementation required for any special domain content access

## Assumptions *(optional)*

- Users accessing special domains are already authenticated to those sites in their browser (logged into Google for docs.google.com, etc.)
- Clipboard permissions are granted to the extension by default or users will grant them when prompted
- Content visible to the user on screen is sufficient for indexing purposes (no need to extract hidden or off-screen content)
- Configuration changes require extension restart to take effect (live reload not required for MVP)
- Special domains typically render all content to visible canvas elements (no hidden layers requiring special handling)
- Backend accepts extracted content as plain text (rich formatting preservation not required for MVP)
- Advanced users can export/import JSON configuration files to add custom special domains

## Dependencies *(optional)*

- Chrome extension must have clipboard read/write permissions declared in manifest
- Backend indexing service must accept content from extension regardless of extraction method
- Users must have clipboard access enabled in browser settings

## Out of Scope *(optional)*

- OAuth integration for accessing protected content on special domains
- Preservation of rich text formatting, images, or embedded media from canvas-rendered content
- Real-time synchronization of content changes from special domains
- Automatic detection of new canvas-rendered domains without manual configuration
- Full GUI for editing special domains configuration (MVP provides export/import only; inline editing UI is future enhancement)
- Content extraction from special domains that require additional user interaction (e.g., clicking to load more content)
- Offline content extraction or caching
- Content extraction from password-protected or paywalled sections within special domains
