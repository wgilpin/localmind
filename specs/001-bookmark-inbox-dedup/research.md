# Research: Save Page to Chrome Bookmarks

**Feature**: `001-bookmark-inbox-dedup`  
**Date**: Saturday Dec 13, 2025  
**Phase**: 0 (Research & Discovery)

## Research Questions

This document consolidates research findings for unknowns identified in the Technical Context and feature requirements.

---

## 1. Chrome Bookmarks API Usage & Best Practices

### Decision

Use `chrome.bookmarks` API with these methods:
- `chrome.bookmarks.search()` - Find existing bookmarks (supports URL search)
- `chrome.bookmarks.create()` - Create new bookmarks
- `chrome.bookmarks.getTree()` - Traverse bookmark hierarchy to find folders
- `chrome.bookmarks.getFolders()` - Not available; use getTree() instead

### Rationale

Chrome's bookmarks API is stable (since Chrome 5) and well-documented. The `search()` method can filter by URL, but requires full URL match - we'll need to implement custom normalization logic. `getTree()` returns the entire bookmark structure, which we can traverse to find "inbox" folders.

**Permission Required**: `"bookmarks"` must be added to `manifest.json` permissions array.

### Implementation Pattern

```javascript
// Search for existing bookmark by normalized URL
async function findExistingBookmark(normalizedUrl) {
  const allBookmarks = await chrome.bookmarks.search({});
  return allBookmarks.find(bm => normalizeUrl(bm.url) === normalizedUrl);
}

// Find inbox folder at root level
async function findInboxFolder() {
  const tree = await chrome.bookmarks.getTree();
  const bookmarkBar = tree[0].children.find(n => n.id === '1'); // Bookmark bar
  const otherBookmarks = tree[0].children.find(n => n.id === '2'); // Other bookmarks
  
  // Search root level only
  const searchRoots = [bookmarkBar, otherBookmarks].filter(Boolean);
  for (const root of searchRoots) {
    const inbox = root.children?.find(c => 
      c.title && c.title.toLowerCase() === 'inbox' && !c.url
    );
    if (inbox) return inbox;
  }
  return null;
}
```

### Alternatives Considered

- **Using `chrome.bookmarks.search({url: exactUrl})`**: Only matches exact URLs, doesn't help with normalization
- **Storing bookmark IDs in extension storage**: Adds complexity, out of sync if user deletes bookmarks manually
- **Using bookmark folders as database**: Over-engineered, Chrome's API is sufficient

### References

- [Chrome Bookmarks API Documentation](https://developer.chrome.com/docs/extensions/reference/bookmarks/)
- [Manifest V3 Permissions](https://developer.chrome.com/docs/extensions/mv3/declare_permissions/)

---

## 2. URL Normalization Best Practices

### Decision

Implement URL normalization using the native `URL` API with custom logic for query params, fragments, and Google Docs verbs:

```javascript
function normalizeUrl(urlString) {
  try {
    const url = new URL(urlString);
    
    // Strip query parameters
    url.search = '';
    
    // Strip fragments
    url.hash = '';
    
    // Special handling for Google Docs
    if (url.hostname === 'docs.google.com') {
      // Remove trailing verbs: /edit, /view, /preview, /copy
      url.pathname = url.pathname.replace(/\/(edit|view|preview|copy)$/i, '');
    }
    
    // Ensure trailing slash consistency (optional, choose one)
    // Option A: Always remove trailing slash
    url.pathname = url.pathname.replace(/\/$/, '') || '/';
    
    return url.toString();
  } catch (e) {
    // Invalid URL, return original
    console.error('Invalid URL:', urlString, e);
    return urlString;
  }
}
```

### Rationale

JavaScript's `URL` API handles protocol, hostname, port normalization automatically. We only need to handle the specific requirements from the spec:
- Query params: `url.search = ''`
- Fragments: `url.hash = ''`  
- Google Docs verbs: Regex replacement on pathname

This approach is robust, handles edge cases (international characters, encoding), and is easily testable.

### Edge Cases Handled

- **Invalid URLs**: Wrapped in try-catch, returns original string
- **Relative URLs**: Not expected in bookmarks, but URL constructor would throw
- **International domains**: URL API handles IDN (Internationalized Domain Names) correctly
- **Encoded characters**: URL API normalizes encoding automatically (e.g., %20 vs +)

### Alternatives Considered

- **Regex-only parsing**: Fragile, doesn't handle edge cases (IDN, encoding, ports)
- **Third-party URL parsing library**: Overkill, adds dependency
- **Chrome's URL parsing**: No exposed API for extensions

### Test Cases

```javascript
// Query parameters
normalizeUrl('https://example.com/page?session=123') 
// → 'https://example.com/page'

// Fragments
normalizeUrl('https://example.com/page#section')
// → 'https://example.com/page'

// Google Docs verbs
normalizeUrl('https://docs.google.com/document/d/ABC123/edit')
// → 'https://docs.google.com/document/d/ABC123'

// Combined
normalizeUrl('https://docs.google.com/document/d/ABC123/edit?usp=sharing#heading=h.abc')
// → 'https://docs.google.com/document/d/ABC123'

// Trailing slashes (consistency)
normalizeUrl('https://example.com/page/')
// → 'https://example.com/page'
```

---

## 3. Debouncing in Vanilla JavaScript

### Decision

Implement a simple closure-based debounce for the Save Page button:

```javascript
function debounce(func, delayMs) {
  let timeoutId = null;
  let isDebouncing = false;
  
  return function debounced(...args) {
    if (isDebouncing) {
      return; // Ignore subsequent calls
    }
    
    isDebouncing = true;
    func.apply(this, args);
    
    timeoutId = setTimeout(() => {
      isDebouncing = false;
    }, delayMs);
  };
}

// Usage
const debouncedSaveBookmark = debounce(saveBookmark, 1500); // 1.5 seconds

// In popup.js
document.getElementById('save-page-btn').addEventListener('click', debouncedSaveBookmark);
```

### Rationale

Spec requires ignoring subsequent clicks for 1-2 seconds (FR-003a). A simple flag-based debounce is cleaner than timer-based debouncing for this use case - we want to execute immediately on first click, then block subsequent clicks.

Traditional debouncing (Lodash-style) delays execution until after the quiet period, which would add unwanted latency.

### Alternatives Considered

- **Lodash/Underscore debounce**: Adds dependency, delays execution
- **Button disabled state**: Requires DOM manipulation, spec says silent operation
- **Timestamp comparison**: More complex, achieves same result
- **Promise-based queue**: Over-engineered for single button

---

## 4. Google Docs URL Patterns

### Decision

Google Docs URLs follow this structure:

```
https://docs.google.com/{type}/d/{document-id}/{verb}[?query][#fragment]

Types: document, spreadsheets, presentation, forms
Verbs: edit, view, preview, copy, (none)
```

**Key Findings**:
- Only `docs.google.com` domain exists (no international variants like .co.uk)
- Document ID is the unique identifier (alphanumeric, typically 44 chars)
- Verbs are optional but commonly present
- Sheets, Slides, Forms follow same pattern but use different subdomains (sheets.google.com, slides.google.com, forms.google.com)

### Rationale

For deduplication, we normalize by:
1. Stripping query params (e.g., `?usp=sharing`)
2. Stripping fragments (e.g., `#heading=h.abc`)
3. Removing trailing verb (e.g., `/edit`, `/view`)

This ensures `docs.google.com/document/d/ABC123/edit` and `docs.google.com/document/d/ABC123/view` are treated as the same document.

### Extension to Other Google Products

**Out of scope for v1** per spec, but for future consideration:
- Google Sheets: `sheets.google.com/spreadsheets/d/{id}/{verb}`
- Google Slides: `slides.google.com/presentation/d/{id}/{verb}`
- Google Forms: `forms.google.com/forms/d/{id}/{verb}`

All follow the same verb pattern and can use the same normalization logic.

### Test URLs

```javascript
// All should normalize to same URL
const urls = [
  'https://docs.google.com/document/d/ABC123',
  'https://docs.google.com/document/d/ABC123/edit',
  'https://docs.google.com/document/d/ABC123/view',
  'https://docs.google.com/document/d/ABC123/edit?usp=sharing',
  'https://docs.google.com/document/d/ABC123/view#heading=h.1'
];

// Expected normalized form:
// 'https://docs.google.com/document/d/ABC123'
```

---

## 5. Testing Chrome Extensions

### Decision

**For v1**: Manual testing via Chrome's extension developer mode
- Load unpacked extension
- Test bookmark creation in various scenarios
- Verify via Chrome's bookmark manager (`chrome://bookmarks`)

**For v2/Future**: Add automated testing with Jest + `chrome-mock`

```json
// package.json (future)
{
  "devDependencies": {
    "jest": "^29.0.0",
    "jest-chrome": "^0.8.0"  // Mocks chrome.* APIs
  },
  "scripts": {
    "test": "jest",
    "test:watch": "jest --watch"
  }
}
```

```javascript
// tests/bookmark-manager.test.js (example structure)
const { normalizeUrl } = require('../bookmark-manager');

describe('URL Normalization', () => {
  test('strips query parameters', () => {
    expect(normalizeUrl('https://example.com/page?foo=bar'))
      .toBe('https://example.com/page');
  });
  
  test('strips fragments', () => {
    expect(normalizeUrl('https://example.com/page#section'))
      .toBe('https://example.com/page');
  });
  
  test('removes Google Docs verbs', () => {
    expect(normalizeUrl('https://docs.google.com/document/d/ABC123/edit'))
      .toBe('https://docs.google.com/document/d/ABC123');
  });
});
```

### Rationale

Chrome extensions are hard to test automatically due to Chrome API dependencies. Manual testing for v1 is pragmatic - the feature is small and critical paths are easily verified. Automated tests can be added later for regression protection.

**Test Plan for v1**:
1. Create bookmark → verify in Chrome bookmarks
2. Create duplicate → verify only one bookmark exists
3. Test with/without inbox folder → verify correct placement
4. Test query params → verify deduplication works
5. Test Google Docs URLs → verify verb normalization
6. Test rapid clicking → verify debouncing works
7. Test empty title → verify fallback to content

### Alternatives Considered

- **Puppeteer/Playwright**: Heavy, requires headless Chrome, overkill for unit testing
- **Sinon for mocking**: Requires complex setup for Chrome APIs
- **No tests**: Risky for URL parsing logic (many edge cases)

**Recommendation**: Add unit tests for `normalizeUrl()` function at minimum (pure function, easy to test). Defer Chrome API integration tests to v2.

---

## 6. Extracting Page Title and Content

### Decision

**Primary**: Use `document.title` from the current tab
**Fallback** (if empty): Use content script to extract first line of text

```javascript
// In popup.js or content script
async function getPageTitle() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  
  if (tab.title && tab.title.trim()) {
    return tab.title;
  }
  
  // Fallback: Inject content script to get first line
  const results = await chrome.scripting.executeScript({
    target: { tabId: tab.id },
    func: () => {
      // Get first meaningful text (skip scripts, styles)
      const walker = document.createTreeWalker(
        document.body,
        NodeFilter.SHOW_TEXT,
        {
          acceptNode: (node) => {
            const text = node.textContent.trim();
            if (text.length > 10 && 
                !node.parentElement.closest('script, style, noscript')) {
              return NodeFilter.FILTER_ACCEPT;
            }
            return NodeFilter.FILTER_SKIP;
          }
        }
      );
      
      const firstText = walker.nextNode();
      if (firstText) {
        // Return first ~50 chars of first meaningful text
        return firstText.textContent.trim().slice(0, 50);
      }
      return 'Untitled';
    }
  });
  
  return results[0]?.result || 'Untitled';
}
```

### Rationale

`document.title` is reliable for 99% of pages. For edge cases (missing title), extract first visible text as a meaningful fallback. This is better than using the URL (spec requirement FR-002a says "first line or first few words of document content").

The TreeWalker approach skips script/style nodes and finds actual visible text efficiently.

### Edge Cases

- **Dynamic pages** (SPAs): `document.title` is usually set by framework
- **Google Docs**: Has meaningful title (document name)
- **Canvas-only pages**: May have no text, fall back to 'Untitled'
- **Very long first line**: Truncate to 50 chars (bookmark title limit is ~255 chars, but 50 is more readable)

### Alternatives Considered

- **Use URL as title**: Rejected per spec clarification (user wants document content)
- **Meta tags**: Less reliable than document.title
- **h1/h2 tags**: May not exist or be meaningful
- **OpenGraph title**: Requires parsing, not always present

---

## Summary of Decisions

| Decision Area | Choice | Rationale |
|--------------|--------|-----------|
| Chrome API Methods | `search()`, `create()`, `getTree()` | Stable API, sufficient for all requirements |
| URL Normalization | Native `URL` API + custom logic | Robust, handles edge cases, zero dependencies |
| Debouncing | Closure-based flag debounce | Simple, executes immediately, blocks subsequent calls |
| Google Docs Handling | Strip verbs via regex | Matches known pattern, future-proof for other Google apps |
| Testing Approach | Manual (v1) + Jest/chrome-mock (v2) | Pragmatic for small feature, extensible later |
| Title Extraction | `document.title` + TreeWalker fallback | Reliable, matches spec requirements |
| Permissions | Add `"bookmarks"` to manifest | Required for chrome.bookmarks API |

**No Unresolved Questions** - All technical unknowns from Technical Context are now resolved.
