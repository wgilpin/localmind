# Feature Specification: Folder Watch and Ingest

**Feature Branch**: `008-folder-watch-ingest`  
**Created**: 2026-04-07  
**Status**: Draft  
**Input**: User description: "the user can add folders to be followed. We will ingest any pdf, md or txt files found in those folders, and reingest if we spot any changes"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Add a Watched Folder (Priority: P1)

A user wants LocalMind to monitor a folder on their device so that documents in it are automatically available for search. They open the settings or management interface and add a folder path to the list of watched folders. The application immediately scans the folder for supported files (PDF, Markdown, plain text) and begins ingesting them.

**Why this priority**: This is the core entry point of the feature. Without the ability to add a folder, all other functionality is unreachable.

**Independent Test**: Can be fully tested by adding a folder containing known files, then verifying those files appear in the knowledge base and are searchable.

**Acceptance Scenarios**:

1. **Given** the user opens the folder management section, **When** they add a valid folder path, **Then** the folder appears in the watched folders list and ingestion of supported files begins.
2. **Given** a folder has been added, **When** ingestion completes, **Then** all PDF, MD, and TXT files from the folder are discoverable in search.
3. **Given** the user provides a path to a folder that does not exist, **When** they attempt to add it, **Then** an error message is shown and the folder is not added.

---

### User Story 2 - Automatic Re-ingestion on File Change (Priority: P2)

A user has already added a watched folder. They update one of the files inside it (or add a new supported file). Without any manual action, LocalMind detects the change and re-ingests the affected file so that the knowledge base reflects the latest content.

**Why this priority**: This ensures the knowledge base remains up to date without user intervention, which is the primary value proposition of watched folders over manual imports.

**Independent Test**: Can be tested by modifying a file inside a watched folder and confirming that a subsequent search reflects the updated content within a reasonable time window.

**Acceptance Scenarios**:

1. **Given** a folder is watched and a file inside it is modified, **When** the system detects the change, **Then** the file is re-ingested and updated content becomes searchable.
2. **Given** a folder is watched, **When** a new supported file (PDF, MD, or TXT) is added to the folder, **Then** the file is automatically ingested and becomes searchable.
3. **Given** a folder is watched, **When** a supported file is deleted from the folder, **Then** that file's content is removed from the knowledge base.
4. **Given** a folder is watched, **When** a file of an unsupported type is added, **Then** it is silently ignored with no error.

---

### User Story 3 - Remove a Watched Folder (Priority: P3)

A user wants to stop LocalMind from monitoring a folder. They remove it from the watched folders list. The application stops watching the folder and removes the previously ingested content from the knowledge base.

**Why this priority**: Users need a way to manage their watched folders over time. This is important for privacy and relevance but is secondary to the core ingest flow.

**Independent Test**: Can be tested by removing a watched folder and confirming that its files are no longer returned in search results.

**Acceptance Scenarios**:

1. **Given** a folder is in the watched list, **When** the user removes it, **Then** the folder is no longer monitored and its files are removed from the knowledge base.
2. **Given** a folder is removed, **When** a file in that folder is subsequently changed, **Then** no re-ingestion occurs.

---

### Edge Cases

- What happens when a watched folder contains a very large number of files (e.g., thousands)?
- How does the system handle a file that is currently being written to when a change is detected?
- What happens if the user adds the same folder twice?
- What if the watched folder is on removable storage that becomes temporarily unavailable?
- What happens when a PDF is password-protected or otherwise unreadable?
- What if two watched folders contain the same file (e.g., via symlinks or nested paths)?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Users MUST be able to add a local folder path to a list of watched folders via the application interface.
- **FR-002**: Users MUST be able to view all currently watched folders in a list within the application.
- **FR-003**: Users MUST be able to remove a folder from the watched list at any time.
- **FR-004**: The system MUST scan a newly added folder recursively and ingest all supported files (PDF, MD, TXT) found within it.
- **FR-005**: The system MUST monitor watched folders for changes and automatically re-ingest any supported file that is added or modified.
- **FR-006**: When a supported file is removed from a watched folder, the system MUST remove its content from the knowledge base.
- **FR-007**: The system MUST ignore files of unsupported types without generating user-facing errors.
- **FR-008**: When a folder is removed from the watched list, its previously ingested content MUST be removed from the knowledge base.
- **FR-009**: The system MUST display an error to the user if a folder path cannot be added (e.g., path does not exist, permission denied).
- **FR-010**: The system MUST prevent duplicate entries if the user attempts to add the same folder path more than once.

### Key Entities

- **Watched Folder**: A folder path registered by the user for monitoring; has a path, creation timestamp, and current status (active, error, unavailable).
- **Ingested File**: A file that has been processed and indexed into the knowledge base; associated with a watched folder, has a path, last-modified timestamp, and ingestion status.
- **Ingestion Event**: A record of a file being ingested or re-ingested; captures the trigger type (initial scan, change detected) and outcome (success, failure, skipped).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can add a folder and have its contents available for search within 60 seconds for folders containing up to 100 files.
- **SC-002**: Changes to files in watched folders are detected and re-ingested within 30 seconds of the file being saved.
- **SC-003**: 100% of supported files (PDF, MD, TXT) present in a watched folder at the time of addition are ingested on initial scan.
- **SC-004**: Removing a watched folder results in all its content being removed from search results within 10 seconds.
- **SC-005**: The application remains responsive during background ingestion; users can continue searching without interruption.

## Assumptions

- The application watches folders recursively (subdirectories are included).
- Change detection uses a file-system event mechanism available on the host OS, with a polling fallback of at most 30 seconds.
- Changes are determined by file modification timestamp and/or file size; content hashing is not required for the initial implementation.
- Ingestion failures for individual files are logged and surfaced per-file without blocking ingestion of other files in the same folder.
- There is no hard limit on the number of watched folders; performance on very large folder trees is a post-MVP concern.
- This spec is implementation-agnostic and applies to both the TypeScript and Rust implementations of LocalMind.
