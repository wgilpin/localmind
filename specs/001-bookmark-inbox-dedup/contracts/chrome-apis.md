# Chrome Extension API Contracts

**Feature**: `001-bookmark-inbox-dedup`  
**Date**: Saturday Dec 13, 2025  
**Target**: Chrome Extension Manifest V3

## Overview

This document defines the Chrome Extension APIs used by the Save Page to Bookmarks feature. These are contracts provided by the Chrome browser, not internal APIs.

---

## Required Permissions

Must be declared in `manifest.json`:

```json
{
  "permissions": [
    "bookmarks",     // NEW: Read/write bookmarks
    "tabs",          // EXISTING: Read active tab info
    "scripting"      // EXISTING: Inject content scripts
  ]
}
```

---

## Chrome Bookmarks API

**Namespace**: `chrome.bookmarks`

**Documentation**: https://developer.chrome.com/docs/extensions/reference/bookmarks/

### Types

#### BookmarkTreeNode

```typescript
interface BookmarkTreeNode {
  id: string;                    // Unique identifier (read-only)
  parentId?: string;             // Parent folder ID
  title: string;                 // Display title (max 255 chars)
  url?: string;                  // URL if bookmark (absent if folder)
  dateAdded?: number;            // Timestamp (ms since epoch)
  dateGroupModified?: number;    // Last modified (folders only)
  children?: BookmarkTreeNode[]; // Child nodes (folders only)
}
```

### Methods Used

#### `chrome.bookmarks.search(query)`

Search for bookmarks matching criteria.

**Signature**:
```typescript
function search(
  query: string | { query?: string; url?: string; title?: string }
): Promise<BookmarkTreeNode[]>
```

**Usage in Feature**:
```javascript
// Search all bookmarks (empty query returns all)
const allBookmarks = await chrome.bookmarks.search({});

// Search by exact URL (not used - doesn't support normalization)
const matches = await chrome.bookmarks.search({ url: 'https://example.com' });
```

**Performance**: O(n) scan, typically <100ms for 1-10k bookmarks

**Returns**: Array of matching BookmarkTreeNode objects

---

#### `chrome.bookmarks.getTree()`

Get the entire bookmark hierarchy.

**Signature**:
```typescript
function getTree(): Promise<BookmarkTreeNode[]>
```

**Usage in Feature**:
```javascript
// Get full tree to find "inbox" folder
const tree = await chrome.bookmarks.getTree();
// tree[0] is root, tree[0].children contains:
//   - id '1' = Bookmark Bar
//   - id '2' = Other Bookmarks
//   - id '3' = Mobile Bookmarks (if Chrome sync enabled)
```

**Performance**: Fast (<10ms), tree is cached by Chrome

**Returns**: Array with single root node containing full hierarchy

---

#### `chrome.bookmarks.create(bookmark)`

Create a new bookmark or folder.

**Signature**:
```typescript
function create(bookmark: {
  parentId?: string;  // Default: root ('0')
  title: string;      // Required
  url?: string;       // Optional, absence creates folder
  index?: number;     // Position in parent (default: end)
}): Promise<BookmarkTreeNode>
```

**Usage in Feature**:
```javascript
// Create bookmark in inbox folder
const newBookmark = await chrome.bookmarks.create({
  parentId: inboxFolder.id,
  title: pageTitle,
  url: pageUrl
});

// Create inbox folder if not exists
const inboxFolder = await chrome.bookmarks.create({
  parentId: '1',  // Bookmark Bar
  title: 'inbox'
  // No url = creates folder
});
```

**Error Handling**:
- Throws if `parentId` invalid
- Throws if `url` invalid format
- Throws if user denied permissions

**Returns**: Newly created BookmarkTreeNode

---

## Chrome Tabs API

**Namespace**: `chrome.tabs`

**Documentation**: https://developer.chrome.com/docs/extensions/reference/tabs/

### Types

#### Tab

```typescript
interface Tab {
  id: number;
  url: string;
  title: string;
  active: boolean;
  windowId: number;
  // ... many other fields
}
```

### Methods Used

#### `chrome.tabs.query(queryInfo)`

Find tabs matching criteria.

**Signature**:
```typescript
function query(queryInfo: {
  active?: boolean;
  currentWindow?: boolean;
  // ... many other filters
}): Promise<Tab[]>
```

**Usage in Feature**:
```javascript
// Get currently active tab
const [activeTab] = await chrome.tabs.query({ 
  active: true, 
  currentWindow: true 
});

const pageUrl = activeTab.url;
const pageTitle = activeTab.title;
```

**Performance**: Instant (<1ms)

**Returns**: Array of matching Tab objects

---

## Chrome Scripting API

**Namespace**: `chrome.scripting`

**Documentation**: https://developer.chrome.com/docs/extensions/reference/scripting/

### Methods Used

#### `chrome.scripting.executeScript(injection)`

Execute JavaScript in a page context.

**Signature**:
```typescript
function executeScript(injection: {
  target: { tabId: number };
  func: Function;       // Function to execute in page
  args?: any[];         // Arguments to pass to func
}): Promise<InjectionResult[]>
```

**Usage in Feature**:
```javascript
// Extract first text from page if title empty
const results = await chrome.scripting.executeScript({
  target: { tabId: activeTab.id },
  func: () => {
    // This runs in page context
    const walker = document.createTreeWalker(
      document.body,
      NodeFilter.SHOW_TEXT,
      (node) => node.textContent.trim().length > 10 
        ? NodeFilter.FILTER_ACCEPT 
        : NodeFilter.FILTER_SKIP
    );
    const firstText = walker.nextNode();
    return firstText?.textContent.trim().slice(0, 50) || 'Untitled';
  }
});

const extractedTitle = results[0]?.result;
```

**Performance**: <50ms (depends on page size)

**Security**: Runs in isolated world (can't access page's JavaScript, but can access DOM)

**Returns**: Array of InjectionResult objects with `result` field

---

## Error Handling

### Common Errors

| API | Error | Cause | Handling |
|-----|-------|-------|----------|
| `bookmarks.create()` | `Error: Invalid parent ID` | Parent folder doesn't exist | Fall back to root level (id '1' or '2') |
| `bookmarks.create()` | `Error: Bookmark not created` | User denied permission | Log silently, fail gracefully |
| `tabs.query()` | Returns empty array | No active tab | Shouldn't happen in popup context, log error |
| `scripting.executeScript()` | `Error: Cannot access chrome://` | Page is restricted | Use tab.title fallback only |

### Error Handling Pattern

```javascript
async function safeCreateBookmark(bookmark) {
  try {
    return await chrome.bookmarks.create(bookmark);
  } catch (error) {
    console.error('Failed to create bookmark:', error);
    
    // Try fallback to top-level if parent folder failed
    if (error.message.includes('Invalid parent') && bookmark.parentId !== '1') {
      try {
        return await chrome.bookmarks.create({
          ...bookmark,
          parentId: '1'  // Bookmark Bar
        });
      } catch (fallbackError) {
        console.error('Fallback also failed:', fallbackError);
        return null;
      }
    }
    
    return null;
  }
}
```

---

## Permission Prompts

When extension requests `"bookmarks"` permission:

**Chrome displays**: "This extension can read and change your bookmarks"

**User must approve** before any bookmark operations work.

**Best Practice**: Explain in extension description or onboarding why bookmark permission is needed.

---

## Rate Limits & Quotas

Chrome Extension APIs don't have explicit rate limits, but:

- **Bookmarks API**: Max ~10,000 bookmarks practical limit (performance degrades)
- **URL length**: Max 2048 characters (Chrome limitation)
- **Title length**: Max 255 characters (Chrome limitation)
- **Create operations**: No limit, but flooding (1000s/sec) may trigger browser slowdown

**Feature Impact**: Debouncing (1.5s) prevents any rate limit concerns.

---

## Browser Compatibility

| API | Chrome Version | Edge | Brave | Opera |
|-----|----------------|------|-------|-------|
| `chrome.bookmarks` | 5+ | 79+ | All | 15+ |
| `chrome.tabs` | 5+ | 79+ | All | 15+ |
| `chrome.scripting` (MV3) | 88+ | 88+ | 1.20+ | 74+ |

**Minimum Chrome Version**: 88 (for `chrome.scripting` in Manifest V3)

**Feature Check**:
```javascript
if (!chrome.bookmarks) {
  console.error('Bookmarks API not available');
  // Disable Save Page button
}
```

---

## Testing Mocks

For unit tests, use `jest-chrome` to mock APIs:

```javascript
// tests/setup.js
import chrome from 'jest-chrome';
global.chrome = chrome;

// tests/bookmark-manager.test.js
describe('createBookmark', () => {
  beforeEach(() => {
    chrome.bookmarks.create.mockResolvedValue({
      id: 'new-bookmark-id',
      title: 'Test Page',
      url: 'https://example.com'
    });
  });
  
  test('creates bookmark in inbox folder', async () => {
    const result = await createBookmark({
      title: 'Test Page',
      url: 'https://example.com',
      parentId: 'inbox-folder-id'
    });
    
    expect(chrome.bookmarks.create).toHaveBeenCalledWith({
      title: 'Test Page',
      url: 'https://example.com',
      parentId: 'inbox-folder-id'
    });
  });
});
```

---

## Security Considerations

### Permissions Required

- **Bookmarks**: Broad access to all user bookmarks (read + write)
- **Tabs**: Read URLs of open tabs (could be sensitive)
- **Scripting**: Execute code in page context (powerful, restricted to user action)

### Security Best Practices

1. **Minimal permissions**: Only request what's needed ✅
2. **User-initiated**: Save Page triggered by explicit click ✅
3. **No external calls**: All processing local ✅
4. **Input validation**: Sanitize URLs via URL constructor ✅
5. **CSP compliance**: No eval(), inline scripts ✅

### Privacy Guarantees

- ✅ No bookmark data sent to external servers
- ✅ No analytics or telemetry
- ✅ No tracking of user behavior
- ✅ Chrome's built-in encryption at rest (if user enables)
- ✅ Respects Chrome sync settings (user-controlled)

---

## Manifest V3 Migration Notes

This feature is built for Manifest V3 from the start:

- ✅ Uses `chrome.bookmarks` (unchanged from MV2)
- ✅ Uses `chrome.scripting.executeScript()` (MV3 replacement for `chrome.tabs.executeScript()`)
- ✅ Uses service worker for background.js (MV3 requirement)
- ✅ No remote code execution (MV3 CSP requirement)

**No migration needed** - already MV3 compliant.
