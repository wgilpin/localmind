# Quickstart: Save Page to Chrome Bookmarks

**Feature**: `001-bookmark-inbox-dedup`  
**Branch**: `001-bookmark-inbox-dedup`  
**Target**: LocalMind Chrome Extension  
**Estimated Dev Time**: 4-6 hours

## Overview

Add "Save Page" functionality to LocalMind Chrome extension that creates persistent Chrome bookmarks with smart URL deduplication and automatic inbox folder organization.

---

## Prerequisites

### Required

- Node.js 16+ (for webpack)
- Chrome browser 88+ (for Manifest V3)
- Git (feature branch already created)
- Text editor / IDE

### Optional

- Jest 29+ (for automated tests - v2 feature)
- Chrome DevTools (for debugging)

### Check Your Environment

```bash
# Verify Node.js
node --version  # Should be 16+

# Verify npm
npm --version

# Navigate to extension directory
cd chrome-extension/

# Install dependencies (if not already)
npm install
```

---

## Development Setup

### 1. Clone & Checkout Feature Branch

```bash
cd /path/to/localmind
git checkout 001-bookmark-inbox-dedup

# Verify you're on correct branch
git branch --show-current  # Should show: 001-bookmark-inbox-dedup
```

### 2. Load Extension in Chrome

```bash
# Build extension (if using webpack)
cd chrome-extension/
npm run build  # or npm run watch for auto-rebuild
```

Then in Chrome:
1. Navigate to `chrome://extensions/`
2. Enable "Developer mode" (toggle in top-right)
3. Click "Load unpacked"
4. Select `chrome-extension/` directory
5. Extension should appear with LocalMind icon

### 3. Verify Current Extension Works

1. Click LocalMind extension icon (should open popup)
2. Verify existing features work (clipboard, Google Docs extraction, etc.)
3. Keep extension loaded for development

---

## Implementation Guide

### Step 1: Add Bookmarks Permission

**File**: `chrome-extension/manifest.json`

**Change**:
```json
{
  "permissions": [
    "activeTab",
    "scripting",
    "clipboardRead",
    "clipboardWrite",
    "storage",
    "bookmarks"  // ADD THIS LINE
  ]
}
```

**Test**: Reload extension in `chrome://extensions/` → Chrome should prompt for new permission

---

### Step 2: Create Bookmark Manager Module

**File**: `chrome-extension/bookmark-manager.js` (NEW FILE)

**Content**:
```javascript
/**
 * Bookmark Manager - Handles Chrome bookmark operations with deduplication
 * @module bookmark-manager
 */

/**
 * Normalizes a URL for duplicate detection
 * Strips query params, fragments, and Google Docs trailing verbs
 * 
 * @param {string} urlString - URL to normalize
 * @returns {string} Normalized URL
 * 
 * @example
 * normalizeUrl('https://docs.google.com/document/d/ABC/edit?usp=sharing')
 * // Returns: 'https://docs.google.com/document/d/ABC'
 */
function normalizeUrl(urlString) {
  try {
    const url = new URL(urlString);
    
    // Strip query parameters
    url.search = '';
    
    // Strip fragments
    url.hash = '';
    
    // Remove Google Docs trailing verbs
    if (url.hostname === 'docs.google.com') {
      url.pathname = url.pathname.replace(/\/(edit|view|preview|copy)$/i, '');
    }
    
    // Normalize trailing slashes
    if (url.pathname !== '/') {
      url.pathname = url.pathname.replace(/\/$/, '');
    }
    
    return url.toString();
  } catch (error) {
    console.error('Invalid URL:', urlString, error);
    return urlString;  // Return original if parsing fails
  }
}

/**
 * Finds existing bookmark matching normalized URL
 * 
 * @param {string} normalizedUrl - Already-normalized URL to search for
 * @returns {Promise<chrome.bookmarks.BookmarkTreeNode|null>}
 */
async function findExistingBookmark(normalizedUrl) {
  const allBookmarks = await chrome.bookmarks.search({});
  return allBookmarks.find(bm => 
    bm.url && normalizeUrl(bm.url) === normalizedUrl
  ) || null;
}

/**
 * Finds "inbox" folder at root level (case-insensitive)
 * 
 * @returns {Promise<chrome.bookmarks.BookmarkTreeNode|null>}
 */
async function findInboxFolder() {
  const tree = await chrome.bookmarks.getTree();
  
  // Search in Bookmark Bar (id '1') and Other Bookmarks (id '2')
  const bookmarkBar = tree[0].children.find(n => n.id === '1');
  const otherBookmarks = tree[0].children.find(n => n.id === '2');
  
  const roots = [bookmarkBar, otherBookmarks].filter(Boolean);
  
  for (const root of roots) {
    if (!root.children) continue;
    
    const inbox = root.children.find(child => 
      child.title && 
      child.title.toLowerCase() === 'inbox' && 
      !child.url  // Must be folder (no url)
    );
    
    if (inbox) return inbox;
  }
  
  return null;
}

/**
 * Creates inbox folder if it doesn't exist
 * 
 * @returns {Promise<chrome.bookmarks.BookmarkTreeNode|null>}
 */
async function createInboxFolder() {
  try {
    return await chrome.bookmarks.create({
      parentId: '1',  // Bookmark Bar
      title: 'inbox'
    });
  } catch (error) {
    console.error('Could not create inbox folder:', error);
    return null;
  }
}

/**
 * Gets page title, with fallback to content extraction if empty
 * 
 * @param {chrome.tabs.Tab} tab - Active tab
 * @returns {Promise<string>} Page title (or 'Untitled' as last resort)
 */
async function getPageTitle(tab) {
  // Primary: Use tab title
  if (tab.title && tab.title.trim()) {
    return tab.title.trim().slice(0, 255);  // Max 255 chars
  }
  
  // Fallback: Extract first text from page
  try {
    const results = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: () => {
        const walker = document.createTreeWalker(
          document.body,
          NodeFilter.SHOW_TEXT,
          {
            acceptNode: (node) => {
              const text = node.textContent.trim();
              const parent = node.parentElement;
              if (text.length > 10 && 
                  parent && 
                  !parent.closest('script, style, noscript')) {
                return NodeFilter.FILTER_ACCEPT;
              }
              return NodeFilter.FILTER_SKIP;
            }
          }
        );
        
        const firstText = walker.nextNode();
        return firstText ? firstText.textContent.trim().slice(0, 50) : null;
      }
    });
    
    if (results[0]?.result) {
      return results[0].result;
    }
  } catch (error) {
    console.warn('Could not extract page title from content:', error);
  }
  
  return 'Untitled';
}

/**
 * Main function: Save current page as bookmark with deduplication
 * 
 * @returns {Promise<boolean>} True if bookmark created, false if duplicate/error
 */
async function savePageAsBookmark() {
  try {
    // Get active tab
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (!tab || !tab.url) {
      console.error('No active tab found');
      return false;
    }
    
    const pageUrl = tab.url;
    const normalizedUrl = normalizeUrl(pageUrl);
    
    // Check for duplicate
    const existingBookmark = await findExistingBookmark(normalizedUrl);
    if (existingBookmark) {
      console.log('Bookmark already exists:', existingBookmark.title);
      return false;  // Silent skip per spec
    }
    
    // Get page title
    const pageTitle = await getPageTitle(tab);
    
    // Find or create inbox folder
    let inboxFolder = await findInboxFolder();
    if (!inboxFolder) {
      inboxFolder = await createInboxFolder();
    }
    
    // Determine parent ID
    const parentId = inboxFolder ? inboxFolder.id : '1';  // Default to Bookmark Bar
    
    // Create bookmark
    const newBookmark = await chrome.bookmarks.create({
      parentId,
      title: pageTitle,
      url: pageUrl  // Use original URL, not normalized
    });
    
    console.log('Bookmark created:', newBookmark.title, 'in', parentId);
    return true;
    
  } catch (error) {
    console.error('Failed to save bookmark:', error);
    return false;
  }
}

// Export functions
if (typeof module !== 'undefined' && module.exports) {
  module.exports = {
    normalizeUrl,
    findExistingBookmark,
    findInboxFolder,
    createInboxFolder,
    getPageTitle,
    savePageAsBookmark
  };
}
```

---

### Step 3: Add Save Page Button to Popup

**File**: `chrome-extension/popup.html`

**Add button** (insert after existing buttons):
```html
<button id="save-page-btn" class="action-button">
  📌 Save Page
</button>
```

**File**: `chrome-extension/popup.css`

**Add styling** (optional, customize as needed):
```css
#save-page-btn {
  background-color: #4CAF50;
  color: white;
  padding: 10px 20px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
  margin: 5px 0;
}

#save-page-btn:hover {
  background-color: #45a049;
}

#save-page-btn:disabled {
  background-color: #cccccc;
  cursor: not-allowed;
}
```

---

### Step 4: Wire Up Button Click Handler

**File**: `chrome-extension/popup.js`

**Add at top**:
```javascript
// Import bookmark manager (if using modules)
// Or include via script tag in popup.html

// Debounce helper
function debounce(func, delayMs) {
  let isDebouncing = false;
  
  return function debounced(...args) {
    if (isDebouncing) return;
    
    isDebouncing = true;
    func.apply(this, args);
    
    setTimeout(() => {
      isDebouncing = false;
    }, delayMs);
  };
}

// Load bookmark-manager.js if not using modules
// (Add <script src="bookmark-manager.js"></script> to popup.html)
```

**Add event listener** (after DOM loaded):
```javascript
document.getElementById('save-page-btn').addEventListener('click', 
  debounce(async () => {
    const button = document.getElementById('save-page-btn');
    button.disabled = true;  // Optional: visual feedback
    
    try {
      await savePageAsBookmark();  // From bookmark-manager.js
    } finally {
      // Re-enable after short delay (debounce handles timing)
      setTimeout(() => {
        button.disabled = false;
      }, 100);
    }
  }, 1500)  // 1.5 second debounce per spec
);
```

**Alternative**: Load via script tag in `popup.html`:
```html
<script src="bookmark-manager.js"></script>
<script src="popup.js"></script>
```

---

### Step 5: Update Webpack Config (If Using Bundler)

**File**: `chrome-extension/webpack.config.js`

**Add entry point** if using webpack:
```javascript
module.exports = {
  entry: {
    popup: './popup.js',
    background: './background.js',
    'content': './content.js',
    'bookmark-manager': './bookmark-manager.js'  // ADD THIS
  },
  // ... rest of config
};
```

Or simply load bookmark-manager.js directly without bundling (simpler for small feature).

---

## Testing

### Manual Testing Checklist

#### Basic Functionality

- [ ] Click Save Page on any website → Bookmark created
- [ ] Open `chrome://bookmarks/` → New bookmark visible
- [ ] Click saved bookmark → Original page loads

#### Deduplication

- [ ] Save `example.com` → Bookmark created
- [ ] Save `example.com?foo=bar` → No new bookmark (duplicate detected)
- [ ] Save `example.com#section` → No new bookmark (duplicate detected)
- [ ] Verify only one `example.com` bookmark exists

#### Google Docs

- [ ] Open Google Doc with `/edit` URL → Save Page
- [ ] Change URL to `/view` → Save Page again
- [ ] Verify only one bookmark for that doc exists

#### Inbox Folder

- [ ] Delete "inbox" folder from bookmarks (if exists)
- [ ] Save Page → Inbox folder created automatically
- [ ] Save another page → Bookmark appears in inbox folder
- [ ] Manually create nested "Folder/inbox" → Save Page → Should use root-level inbox (or create one)

#### Title Handling

- [ ] Save page with normal title → Uses page title
- [ ] Save page with very long title → Truncates to 255 chars
- [ ] Create HTML page with no title → Should use content fallback

#### Debouncing

- [ ] Click Save Page button rapidly 5x → Only one bookmark created
- [ ] Wait 2 seconds → Click again → New bookmark created (if URL different)

#### Error Handling

- [ ] Save on restricted page (`chrome://extensions/`) → Graceful failure
- [ ] Save with bookmark permission revoked → Error logged to console

---

### Automated Testing (Optional - v2)

#### Setup Jest

```bash
cd chrome-extension/
npm install --save-dev jest jest-chrome

# Add to package.json
"scripts": {
  "test": "jest",
  "test:watch": "jest --watch"
}
```

#### Create Test File

**File**: `chrome-extension/tests/bookmark-manager.test.js`

```javascript
const { normalizeUrl } = require('../bookmark-manager');

describe('normalizeUrl', () => {
  test('strips query parameters', () => {
    expect(normalizeUrl('https://example.com/page?foo=bar'))
      .toBe('https://example.com/page');
  });
  
  test('strips fragments', () => {
    expect(normalizeUrl('https://example.com/page#section'))
      .toBe('https://example.com/page');
  });
  
  test('removes Google Docs /edit verb', () => {
    expect(normalizeUrl('https://docs.google.com/document/d/ABC123/edit'))
      .toBe('https://docs.google.com/document/d/ABC123');
  });
  
  test('removes Google Docs /view verb', () => {
    expect(normalizeUrl('https://docs.google.com/document/d/ABC123/view'))
      .toBe('https://docs.google.com/document/d/ABC123');
  });
  
  test('handles combined normalization', () => {
    expect(normalizeUrl('https://docs.google.com/document/d/ABC/edit?usp=sharing#h.123'))
      .toBe('https://docs.google.com/document/d/ABC');
  });
  
  test('preserves protocol and domain', () => {
    const url = 'https://example.com/path';
    expect(normalizeUrl(url)).toBe(url);
  });
  
  test('handles invalid URLs gracefully', () => {
    const invalid = 'not-a-url';
    expect(normalizeUrl(invalid)).toBe(invalid);
  });
});
```

#### Run Tests

```bash
npm test
```

---

## Debugging

### Enable Console Logging

All functions already include `console.log` and `console.error` statements.

**View logs**:
1. Open `chrome://extensions/`
2. Find LocalMind extension
3. Click "service worker" (for background logs)
4. Right-click extension icon → Inspect popup (for popup logs)

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Button does nothing | Permission not granted | Reload extension after adding `"bookmarks"` to manifest |
| "Cannot read property..." | Module not loaded | Add `<script src="bookmark-manager.js"></script>` to popup.html |
| Duplicate still created | Debounce not working | Check debounce implementation, verify 1.5s delay |
| Inbox folder not found | Folder name mismatch | Case-insensitive search should work, check console logs |
| Title is 'Untitled' | Content extraction failed | Check scripting permission, verify page has text content |

### Debug Workflow

1. Open Chrome DevTools (F12)
2. Set breakpoints in `bookmark-manager.js`
3. Click Save Page button
4. Step through execution
5. Check Chrome Bookmark Manager (`chrome://bookmarks/`) for results

---

## Performance Verification

### Benchmarks

Run these tests to verify performance targets:

```javascript
// In console, with extension loaded
async function benchmark() {
  console.time('Save Page (no duplicate)');
  await savePageAsBookmark();
  console.timeEnd('Save Page (no duplicate)');
  // Should be <2000ms
  
  console.time('Save Page (duplicate detected)');
  await savePageAsBookmark();
  console.timeEnd('Save Page (duplicate detected)');
  // Should be <500ms
}
```

**Targets**:
- Bookmark creation (new): <2 seconds
- Duplicate detection: <500ms
- URL normalization: <100ms (should be instant)

---

## Code Quality

### Before Committing

```bash
# Format code (if using Prettier)
npx prettier --write chrome-extension/bookmark-manager.js
npx prettier --write chrome-extension/popup.js

# Lint (if using ESLint)
npx eslint chrome-extension/bookmark-manager.js

# Run tests (if added)
npm test
```

### Documentation

- ✅ All functions have JSDoc comments
- ✅ Complex logic has inline comments
- ✅ README updated with new feature (if applicable)

---

## Deployment

### Package Extension

```bash
cd chrome-extension/
npm run build

# Create zip for Chrome Web Store (if publishing)
zip -r localmind-extension.zip . -x "node_modules/*" -x "*.git*"
```

### Versioning

Update `manifest.json` version:
```json
{
  "version": "1.1.0"  // Increment from 1.0.0
}
```

### Git Commit

```bash
git add chrome-extension/
git commit -m "Add Save Page to Bookmarks feature with deduplication"
git push origin 001-bookmark-inbox-dedup
```

---

## Troubleshooting

### Extension Not Loading

```bash
# Check for manifest errors
cd chrome-extension/
cat manifest.json  # Verify JSON is valid

# Rebuild if using webpack
npm run build
```

### Permissions Issues

- Reload extension after manifest changes
- Check Chrome console for permission errors
- Manually grant permissions in `chrome://extensions/`

### Bookmark Not Created

- Check console for errors
- Verify `chrome.bookmarks` API is available
- Test with simple bookmark (no special URL normalization)

---

## Next Steps

After implementing this feature:

1. **Test thoroughly** using checklist above
2. **Gather feedback** from users
3. **Monitor performance** in production
4. **Consider v2 enhancements**:
   - Automated tests
   - Custom folder configuration
   - Bulk deduplication tool
   - Support for other Google products (Sheets, Slides, Forms)

---

## Resources

- [Chrome Bookmarks API Docs](https://developer.chrome.com/docs/extensions/reference/bookmarks/)
- [Manifest V3 Migration Guide](https://developer.chrome.com/docs/extensions/mv3/intro/)
- [LocalMind Constitution](../../../.specify/memory/constitution.md)
- [Feature Spec](./spec.md)
- [Data Model](./data-model.md)
- [Chrome API Contracts](./contracts/chrome-apis.md)

---

## Support

Questions or issues? Check:
- Feature spec: `specs/001-bookmark-inbox-dedup/spec.md`
- Chrome extension docs: https://developer.chrome.com/docs/extensions/
- LocalMind repo README: `README.md`
