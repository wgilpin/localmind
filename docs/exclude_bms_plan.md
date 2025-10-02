# Bookmark Exclusion Feature - Implementation Plan

## Overview

Add functionality to allow users to exclude certain bookmarks from indexing based on folder paths and domain patterns. Users will manage exclusion rules via the UI, and changes to the exclusion list will trigger reprocessing of the bookmark file.

## Requirements

### Functional Requirements

1. **FR-1**: User can define a list of bookmark folder paths to exclude from indexing
2. **FR-2**: User can define a list of domain patterns (e.g., "private.com", "*.internal.org") to exclude from indexing
3. **FR-3**: User can add, edit, and delete domain patterns via the UI
4. **FR-4**: User can toggle folder exclusions via checkbox tree in the UI
5. **FR-5**: Exclusion rules are persisted across application restarts
6. **FR-6**: When exclusion rules change, the system automatically reprocesses the bookmark file
7. **FR-7**: Previously indexed bookmarks matching new exclusion rules are removed from the database
8. **FR-8**: Previously excluded bookmarks are added when removed from exclusion rules
9. **FR-9**: UI provides clear visibility of exclusion rules and their impact
10. **FR-10**: Excluded bookmarks are filtered before content fetching (to avoid unnecessary network requests)

### Non-Functional Requirements

1. **NFR-1**: Exclusion rule changes should complete within 10 seconds for typical bookmark files (< 1000 bookmarks)
2. **NFR-2**: UI should be responsive during reprocessing (background operation)
3. **NFR-3**: Exclusion patterns should support wildcards for flexible matching
4. **NFR-4**: Database operations should be atomic to prevent inconsistent state
5. **NFR-5**: Domain pattern validation provides immediate feedback

## Architecture

### Data Model

#### Configuration Storage (Database)

Add exclusion configuration to the existing `config` table:

- Key: `bookmark_exclude_folders` - JSON array of folder IDs/paths to exclude
- Key: `bookmark_exclude_domains` - JSON array of domain patterns (supports wildcards)

Example values:

```json
// bookmark_exclude_folders
["123456", "789012", "345678"]  // Bookmark folder IDs from Chrome

// bookmark_exclude_domains
["*.internal.com", "private.example.org", "localhost:*"]
```

#### Bookmark Structure Enhancement

The Chrome bookmark structure already contains folder information through the hierarchical `BookmarkItem` structure. We'll need to track the folder path during extraction.

### Components to Modify

#### 1. Database Layer (`src/db.rs`)

**Changes:**

- Add methods for exclusion configuration:
  - `get_excluded_folders() -> Result<Vec<String>>`
  - `set_excluded_folders(folders: &[String]) -> Result<()>`
  - `get_excluded_domains() -> Result<Vec<String>>`
  - `set_excluded_domains(domains: &[String]) -> Result<()>`
- Add method to delete bookmarks by URL patterns:
  - `delete_bookmarks_by_url_pattern(pattern: &str) -> Result<usize>`
- Add method to delete bookmarks by folder:
  - `delete_bookmarks_by_folder(folder_id: &str) -> Result<usize>`

#### 2. Bookmark Monitor (`src/bookmark.rs`)

**Changes:**

- Modify `BookmarkItem` to track folder path:

  ```rust
  pub struct BookmarkItemWithPath {
      pub item: BookmarkItem,
      pub folder_path: Vec<String>,  // e.g., ["Bookmark Bar", "Work", "Projects"]
      pub folder_id: String,
  }
  ```

- Add exclusion filter to `extract_bookmarks`:
  - Filter by folder ID during recursive traversal
  - Filter by domain pattern before adding to results
- Add new method `apply_exclusion_rules`:
  - Compare current bookmarks with database
  - Remove newly excluded bookmarks
  - Add newly included bookmarks
- Modify `get_bookmarks_for_ingestion` to respect exclusion rules
- Modify `get_bookmarks_metadata` to respect exclusion rules

#### 3. Exclusion Matcher (`src/bookmark_exclusion.rs` - NEW)

Create a new module for exclusion logic:

```rust
pub struct ExclusionRules {
    excluded_folders: Vec<String>,
    excluded_domain_patterns: Vec<String>,
}

impl ExclusionRules {
    pub fn new(folders: Vec<String>, domains: Vec<String>) -> Self;

    pub fn is_folder_excluded(&self, folder_id: &str) -> bool;

    pub fn is_url_excluded(&self, url: &str) -> bool;

    fn matches_domain_pattern(url: &str, pattern: &str) -> bool;

    pub fn validate_pattern(pattern: &str) -> Result<(), String>;
}
```

**Pattern Matching Logic:**

- `*.example.com` matches `foo.example.com`, `bar.example.com`
- `example.com` matches exactly `example.com` and `www.example.com`
- `*:8080` matches any host on port 8080
- `localhost*` matches `localhost`, `localhost:3000`, etc.

#### 4. Tauri Commands (`src/main.rs`)

Add new Tauri commands:

```rust
#[tauri::command]
async fn get_exclusion_rules(state: State<'_, RagState>)
    -> Result<ExclusionRulesResponse, String>;

#[tauri::command]
async fn set_exclusion_rules(
    folders: Vec<String>,
    domains: Vec<String>,
    state: State<'_, RagState>
) -> Result<ReprocessingStatus, String>;

#[tauri::command]
async fn get_bookmark_folders(state: State<'_, RagState>)
    -> Result<Vec<BookmarkFolder>, String>;

#[tauri::command]
async fn validate_domain_pattern(pattern: String)
    -> Result<ValidationResult, String>;

#[tauri::command]
async fn preview_exclusion_impact(
    folders: Vec<String>,
    domains: Vec<String>,
    state: State<'_, RagState>
) -> Result<PreviewResult, String>;
```

Response types:

```rust
#[derive(Serialize)]
struct ExclusionRulesResponse {
    excluded_folders: Vec<String>,
    excluded_domains: Vec<String>,
}

#[derive(Serialize)]
struct BookmarkFolder {
    id: String,
    name: String,
    path: Vec<String>,  // Breadcrumb path
    bookmark_count: usize,
}

#[derive(Serialize)]
struct ReprocessingStatus {
    bookmarks_removed: usize,
    bookmarks_added: usize,
}

#[derive(Serialize)]
struct ValidationResult {
    valid: bool,
    error_message: Option<String>,
}

#[derive(Serialize)]
struct PreviewResult {
    bookmarks_to_exclude: usize,
    bookmarks_to_include: usize,
}
```

#### 5. UI Layer (`src-ui/`)

**New Files:**

- `settings.html` - Settings page for managing exclusions
- `settings.js` - Settings page logic
- `settings.css` - Settings page styles

**New UI Components:**

##### Settings Page Layout

```text
┌───────────────────────────────────────────────┐
│ ⚙️ Settings                    [← Back]       │
├───────────────────────────────────────────────┤
│                                               │
│ 📁 Excluded Bookmark Folders                  │
│ ┌───────────────────────────────────────┐    │
│ │ ☐ Bookmark Bar (47)                   │    │
│ │   ☑ Work Stuff (23)                   │    │
│ │   ☐ Personal (24)                     │    │
│ │ ☐ Other Bookmarks (12)                │    │
│ │   ☐ Archive (8)                       │    │
│ │   ☐ Old Links (4)                     │    │
│ └───────────────────────────────────────┘    │
│                                               │
│ 🌐 Excluded Domain Patterns                   │
│ ┌───────────────────────────────────────┐    │
│ │ *.internal.company.com (5)       ✏️ ❌ │    │
│ │ private.example.org (2)          ✏️ ❌ │    │
│ │ localhost:* (1)                  ✏️ ❌ │    │
│ └───────────────────────────────────────┘    │
│                                               │
│ Add New Pattern:                              │
│ ┌───────────────────────────────────────┐    │
│ │ [Enter domain pattern...]         │    │
│ └───────────────────────────────────────┘    │
│ [+ Add Pattern]                               │
│                                               │
│ ℹ️ Preview: 28 bookmarks will be excluded     │
│            (23 from folders, 5 from domains)  │
│                                               │
│ [Cancel] [Save Changes]                      │
└───────────────────────────────────────────────┘
```

**UI Interaction Details:**

**Folder Management:**

- Folders displayed as checkbox tree (hierarchical)
- Checking a folder excludes all bookmarks within that folder and subfolders
- Unchecking a folder re-includes those bookmarks
- Folder names show bookmark count in parentheses: "Work Stuff (23)"
- Real-time preview updates as user toggles checkboxes
- Indentation shows hierarchy (2 spaces per level)

**Domain Pattern Management:**

*Adding a Pattern:*

1. User types pattern in input field: `*.private.com`
2. Input validates pattern format in real-time (shows ✓ or ✗ icon)
3. Shows inline validation errors below input if invalid
   - Examples: "Invalid pattern: cannot contain protocol", "Pattern cannot be empty"
4. User clicks "+ Add Pattern" button or presses Enter
5. If valid: Pattern appears in list above with matched count
6. Input field clears, ready for next pattern
7. Preview count updates showing total excluded bookmarks

*Editing a Pattern:*

1. User clicks edit icon (✏️) next to pattern
2. Pattern text becomes editable inline (or input field is populated)
3. User modifies pattern (e.g., change `*.private.com` to `private.*`)
4. Validation occurs in real-time during editing
5. User presses Enter or clicks checkmark icon to save
6. User presses Escape or clicks X icon to cancel without saving
7. On save: Pattern updates in list, preview count updates
8. Invalid patterns cannot be saved (save button disabled)

*Deleting a Pattern:*

1. User clicks delete icon (❌) next to pattern
2. Pattern is removed from list immediately
3. No confirmation modal (changes not persisted until "Save Changes")
4. Preview count updates to reflect removal
5. If user navigates away or clicks Cancel, deletion is reverted

*Pattern List Features:*

- Patterns displayed in alphabetical order
- Each pattern shows matched bookmark count in parentheses when available
- Hover tooltip shows example URLs that match the pattern
- Empty state message: "No domain patterns excluded. Add one below to get started."
- Maximum 100 patterns allowed (shows warning near limit)

**Validation Rules:**

Valid pattern must satisfy:

- Not empty
- No protocol prefix (http://, https://)
- No path segments (/, /admin, etc.)
- Valid domain characters: letters, numbers, dots, hyphens, asterisks, colons
- Wildcards (*) allowed in specific positions
- Maximum length: 253 characters (DNS limit)

**Valid Pattern Examples:**

- `example.com` - Exact domain
- `*.example.com` - All subdomains of example.com
- `*example.com` - Any domain ending with example.com
- `example.*` - Any domain starting with example
- `localhost` - Exact match for localhost
- `localhost:*` - Localhost on any port
- `*:8080` - Any host on port 8080
- `192.168.*.*` - IP range patterns

**Invalid Pattern Examples:**

- `` (empty)
- `**example.com` (double wildcard)
- `http://example.com` (includes protocol)
- `example.com/path` (includes path)
- `example com` (space in domain)
- `.example.com` (starts with dot)

**Preview Calculation:**

- Runs in real-time as user makes changes
- Shows breakdown: "(X from folders, Y from domains)"
- Debounced (500ms) to avoid excessive calculations
- Shows loading spinner during calculation
- Caches results for performance

**Save/Cancel Behavior:**

- "Cancel" button: Discards all changes, returns to main view
- "Save Changes" button:
  - Disabled until changes are made
  - Disabled if any validation errors exist
  - Shows loading state during save
  - On success: Shows toast "Exclusion rules updated. X bookmarks excluded."
  - On error: Shows error message, keeps user on settings page
- Keyboard shortcut: Ctrl+S / Cmd+S to save
- Unsaved changes warning if user tries to navigate away

**Modifications to `index.html`:**

- Add "⚙️ Settings" button in top-right corner (or sidebar)
- Navigation shows/hides settings panel (modal or page replacement)

**Modifications to `app.js`:**

- Add navigation handler for settings page
- Add state management for settings view
- Add toast/notification support for success/error messages

## Implementation Flow

### User Story 1: View Current Exclusion Rules

1. User clicks "Settings" button in main UI
2. Settings page loads
3. UI calls `get_exclusion_rules()` command → receives folders and domains
4. UI calls `get_bookmark_folders()` → receives complete folder tree with counts
5. UI renders folder tree with excluded folders checked
6. UI renders domain pattern list with matched counts
7. UI shows current excluded bookmark count

### User Story 2: Add Folder Exclusion

1. User browses folder tree in settings
2. User checks checkbox next to "Work Stuff" folder
3. UI marks folder as excluded in local state (not saved)
4. UI calls `preview_exclusion_impact()` with updated folders
5. UI shows preview: "28 bookmarks will be excluded (23 from folders, 5 from domains)"
6. User clicks "Save Changes"
7. UI calls `set_exclusion_rules(folders, domains)` with full rule set
8. Backend:
   - Validates input
   - Saves new rules to database config
   - Identifies bookmarks in excluded folders
   - Deletes matching bookmarks and their embeddings in transaction
   - Returns `ReprocessingStatus { bookmarks_removed: 23, bookmarks_added: 0 }`
9. UI shows success toast: "✅ Exclusion rules updated. 23 bookmarks excluded."
10. UI updates document count in main view
11. Settings page returns to main view or stays open

### User Story 3: Add Domain Exclusion

1. User clicks in "Add New Pattern" input field
2. User types: `*.private.com`
3. UI validates in real-time, shows ✓ icon
4. UI shows preview: "5 URLs match this pattern"
5. User presses Enter or clicks "+ Add Pattern"
6. Pattern appears in list above: `*.private.com (5) ✏️ ❌`
7. UI calls `preview_exclusion_impact()` with updated domains
8. Preview updates: "33 bookmarks will be excluded (23 from folders, 10 from domains)"
9. User clicks "Save Changes"
10. Same backend flow as folder exclusion
11. UI shows success toast: "✅ Exclusion rules updated. 5 bookmarks excluded."

### User Story 4: Edit Domain Pattern

1. User clicks edit icon (✏️) next to pattern `*.private.com`
2. Pattern becomes inline editable or input field is populated
3. User changes pattern to `private.*`
4. UI validates in real-time
5. Preview updates showing new impact
6. User presses Enter to save
7. Pattern updates in list
8. User clicks "Save Changes" to persist
9. Backend reprocesses with new pattern

### User Story 5: Delete Domain Pattern

1. User clicks delete icon (❌) next to pattern `localhost:*`
2. Pattern is removed from list immediately
3. Preview updates: "27 bookmarks will be excluded (23 from folders, 4 from domains)"
4. User sees pattern is gone but changes not saved
5. User can click "Cancel" to revert, or "Save Changes" to persist
6. On save: Backend flow runs, affected bookmarks may be re-included

### User Story 6: Remove Folder Exclusion (Re-index)

1. User unchecks "Work Stuff" folder
2. UI updates local state
3. UI calls `preview_exclusion_impact()`
4. Preview shows: "5 bookmarks will be excluded, 23 will be re-indexed"
5. User clicks "Save Changes"
6. UI shows loading state: "Re-indexing bookmarks..."
7. Backend:
   - Saves new rules
   - Re-parses Chrome bookmarks file
   - Filters by new exclusion rules
   - Identifies newly allowed bookmarks (23)
   - Fetches content for each bookmark (shows progress)
   - Generates embeddings
   - Inserts into database
   - Returns `ReprocessingStatus { bookmarks_removed: 0, bookmarks_added: 23 }`
8. UI shows progress toast: "Processing bookmarks... 5/23"
9. UI shows success toast: "✅ Exclusion rules updated. 23 bookmarks added."
10. UI refreshes document count

## Database Schema Changes

No schema changes required - use existing `config` table:

```sql
-- Example config entries
INSERT INTO config (key, value) VALUES
  ('bookmark_exclude_folders', '["123456", "789012"]'),
  ('bookmark_exclude_domains', '["*.internal.com", "private.example.org"]');
```

Optional: Add index for faster URL pattern matching:

```sql
CREATE INDEX IF NOT EXISTS idx_documents_url ON documents(url);
```

## Testing Strategy

### Unit Tests

1. **Domain Pattern Matching** (`bookmark_exclusion.rs`)
   - Test wildcard patterns: `*.example.com`
   - Test exact matches: `example.com`
   - Test port patterns: `*:8080`
   - Test prefix patterns: `localhost*`
   - Test IP patterns: `192.168.*.*`

2. **Pattern Validation** (`bookmark_exclusion.rs`)
   - Test valid patterns return Ok
   - Test invalid patterns return appropriate errors
   - Test edge cases (empty, too long, special chars)

3. **Folder Path Extraction** (`bookmark.rs`)
   - Test recursive folder path tracking
   - Test folder ID extraction
   - Test nested folder handling

4. **Exclusion Filtering** (`bookmark.rs`)
   - Test bookmark filtering by single folder
   - Test bookmark filtering by nested folders
   - Test bookmark filtering by domain pattern
   - Test combined folder + domain filtering

### Integration Tests

1. **Configuration Persistence**
   - Save exclusion rules, restart app, verify rules persisted
   - Modify rules, verify changes saved correctly

2. **Reprocessing Flow**
   - Add folder exclusion, verify bookmarks removed from DB
   - Remove folder exclusion, verify bookmarks re-added
   - Add domain exclusion, verify matching URLs excluded
   - Remove domain exclusion, verify URLs re-indexed

3. **UI Integration**
   - Test folder tree rendering with counts
   - Test pattern validation in real-time
   - Test preview count accuracy
   - Test add/edit/delete pattern flows
   - Test save/cancel behavior
   - Test unsaved changes warning

### Manual Testing Checklist

- [ ] Add folder exclusion, verify bookmarks disappear from search
- [ ] Remove folder exclusion, verify bookmarks reappear
- [ ] Add domain pattern, verify matching bookmarks excluded
- [ ] Edit domain pattern, verify exclusions update
- [ ] Delete domain pattern, verify bookmarks re-indexed
- [ ] Test with empty bookmark file
- [ ] Test with all bookmarks excluded
- [ ] Test with invalid patterns
- [ ] Test preview accuracy with various combinations
- [ ] Test UI responsiveness during reprocessing
- [ ] Test persistence across app restarts
- [ ] Test concurrent changes (folder + domain simultaneously)

## Edge Cases & Error Handling

### Edge Cases

1. **Empty bookmark file**: Show message, disable exclusion UI
2. **All bookmarks excluded**: Valid state, document count = 0, show info message
3. **Invalid folder ID in saved config**: Ignore invalid IDs, log warning, show toast
4. **Malformed domain pattern in saved config**: Ignore invalid patterns, log warning, show toast
5. **Bookmark in multiple folders**: Exclude if ANY parent folder is excluded
6. **Duplicate domain patterns**: Deduplicate on save, show warning
7. **Chrome bookmarks file doesn't exist**: Show error, disable exclusion features
8. **Folder deleted from Chrome**: Remove from exclusion list, show info message
9. **Pattern matches no bookmarks**: Allow saving, show "(0)" count
10. **Very long folder/domain lists**: Paginate or virtualize lists for performance

### Error Handling

1. **Database errors during reprocessing**:
   - Rollback via transaction
   - Show error toast: "Failed to save exclusion rules: [error]"
   - Keep user on settings page with changes intact

2. **Network errors during re-indexing**:
   - Mark bookmarks as failed
   - Continue processing others
   - Show partial success: "Added 18/23 bookmarks (5 failed)"

3. **Invalid pattern format**:
   - Show inline validation error below input
   - Prevent adding to list
   - Disable save button if any invalid patterns

4. **Concurrent bookmark updates**:
   - Queue exclusion rule changes
   - Process sequentially
   - Show "Processing..." state

5. **Chrome bookmark file locked/inaccessible**:
   - Show error: "Cannot access Chrome bookmarks. Close Chrome and try again."
   - Disable save button

6. **Tauri command timeout**:
   - Show error after 30 seconds
   - Allow user to retry or cancel

## Performance Considerations

### Optimization Strategies

1. **Lazy Reprocessing**:
   - Only reprocess changed bookmarks, not entire file
   - Track which folders/patterns changed
   - Calculate diff: (current - previous) rules

2. **Batch Operations**:
   - Use batch delete for excluded bookmarks
   - Use batch insert for re-indexed bookmarks
   - Wrap in transactions for atomicity

3. **Cached Folder Tree**:
   - Cache bookmark folder structure on settings page load
   - Invalidate cache only when Chrome bookmarks file changes
   - Store in Tauri state for fast access

4. **Background Processing**:
   - Run reprocessing in async background task
   - Send progress events to UI
   - Allow cancellation

5. **Debounced Preview**:
   - Debounce preview calculation (500ms)
   - Cancel previous calculation if new change occurs
   - Show loading spinner during calculation

6. **Database Indexes**:
   - Add index on `documents.url` for pattern matching
   - Add index on `documents.source` to filter bookmarks

### Estimated Impact

- **Folder exclusion check**: O(1) per bookmark (hash set lookup)
- **Domain pattern check**: O(p × u) where p=patterns, u=URL length (typically fast)
- **Reprocessing time**:
  - Deletion: ~100ms for 100 bookmarks
  - Re-indexing: ~1-5 seconds for 100 bookmarks (network-bound)
- **Preview calculation**: ~50-200ms for typical bookmark files
- **UI render time**: <100ms for folder tree (up to 1000 folders)

## Migration Plan

### Phase 1: Backend Foundation (Day 1-2)

1. Create `bookmark_exclusion.rs` module with pattern matching
2. Add unit tests for pattern matching (10+ test cases)
3. Add database config methods (get/set for folders and domains)
4. Modify `extract_bookmarks` to track folder paths and IDs
5. Add unit tests for folder path tracking

### Phase 2: Reprocessing Logic (Day 2-3)

1. Implement exclusion filtering in `BookmarkMonitor`
2. Add `apply_exclusion_rules` method
3. Add database methods for deleting by folder/pattern
4. Add Tauri commands (get/set rules, get folders, validate, preview)
5. Add integration tests for reprocessing flows

### Phase 3: UI Implementation (Day 3-4)

1. Create `settings.html` with layout
2. Create `settings.css` with styling
3. Create `settings.js` with state management
4. Implement folder tree rendering with checkboxes
5. Implement domain pattern list with add/edit/delete
6. Add real-time validation
7. Add preview calculation with debouncing
8. Wire up all Tauri command calls
9. Add navigation to/from settings page

### Phase 4: Testing & Polish (Day 4-5)

1. End-to-end testing of all user stories
2. Manual testing checklist
3. Error handling refinement
4. UI/UX polish (animations, loading states, error messages)
5. Performance testing with large bookmark files
6. Documentation updates

## Future Enhancements

1. **Import/Export exclusion rules**: JSON file export/import for sharing configs
2. **URL path pattern exclusions**: Pattern like `*/admin/*` or `*/login`
3. **Temporary exclusions**: Soft-exclude without deleting (toggle on/off)
4. **Exclusion reasons/notes**: Add text note for why pattern was excluded
5. **Smart suggestions**: Auto-suggest patterns based on bookmark analysis
6. **Regex support**: Advanced pattern matching with full regex
7. **Bookmark tagging**: Tag-based exclusion instead of folder-based
8. **Exclusion history**: Track when rules were added/modified/removed
9. **Bulk operations**: Select multiple patterns to edit/delete at once
10. **Exclusion templates**: Predefined pattern sets (e.g., "Exclude Local Development")

## Open Questions & Decisions

### Q1: Should we soft-delete excluded bookmarks or hard-delete?

**Decision**: Hard delete from `documents` table but keep track via exclusion rules. Rationale:

- Saves database space
- Simplifies queries (no need to filter out soft-deleted)
- Can always re-index by removing exclusion rule
- Embeddings are expensive to store, no need to keep if excluded

### Q2: Should exclusion rules apply to future bookmark syncs automatically?

**Decision**: Yes, always apply on bookmark file changes. Rationale:

- Expected behavior (user sets rules once)
- Prevents re-indexing excluded bookmarks
- Consistent with "set and forget" UX

### Q3: Should we show excluded bookmark count in stats?

**Decision**: Yes, show in settings page and optionally in main stats. Rationale:

- Transparency for user
- Helps user understand why search results may be limited
- Shows impact of exclusion rules

### Q4: How to handle bookmarks that match both include and exclude rules?

**Decision**: Exclude takes precedence (deny-by-default). Rationale:

- Safer default (privacy/security)
- User explicitly set exclusion rules
- Easier to reason about (no conflict resolution logic)

### Q5: Should we allow regex patterns in addition to wildcards?

**Decision**: Start with wildcards only, add regex as future enhancement. Rationale:

- Simpler UX for most users
- Wildcards cover 90% of use cases
- Can add regex as "advanced" mode later
- Easier to validate and provide clear error messages

### Q6: Should editing a pattern require explicit save, or save immediately?

**Decision**: Changes require explicit "Save Changes" click (staging area). Rationale:

- Allows users to make multiple changes and preview impact
- Prevents accidental saves
- Consistent with folder exclusion UX (both use same save button)
- Allows cancel/revert

## Success Criteria

- ✅ User can exclude bookmarks by folder via checkbox tree in UI
- ✅ User can add domain patterns via input field with validation
- ✅ User can edit domain patterns inline
- ✅ User can delete domain patterns with single click
- ✅ Real-time preview shows impact of changes before saving
- ✅ Exclusion rules persist across application restarts
- ✅ Changing rules triggers automatic reprocessing
- ✅ Previously indexed bookmarks are removed when excluded
- ✅ Previously excluded bookmarks are added when rules change
- ✅ No data loss during reprocessing (transactions used)
- ✅ UI remains responsive during reprocessing (background task)
- ✅ Progress shown for long-running operations
- ✅ All unit and integration tests pass
- ✅ Manual testing checklist completed
