# Google Docs Extraction Strategy: Export URL Method

## Objective

Enable the Chrome Extension to extract full text from Google Docs without capturing UI noise, overcoming the Canvas-based rendering limitation.

## The Problem

Google Docs uses a Canvas element for rendering, which creates several challenges:

- **Canvas rendering**: Standard DOM scraping (`document.innerText`, `execCommand`) captures screen-reader artifacts, not the document body
- **Obfuscated model**: Client-side scripts cannot access the document model directly due to obfuscation
- **UI noise**: Select-all operations capture UI elements (menus, prompts, screen reader text) instead of document content

## The Solution

Use the **Mobile Basic View** endpoint:

```
https://docs.google.com/document/d/[DOC_ID]/mobilebasic
```

This provides a simplified HTML version of the document (meant for mobile devices) that can be parsed to extract clean text.

### Why Not the Export URL?

The export URL (`/export?format=txt`) would be ideal (returns plain text), but **it doesn't work in browser extensions** due to CORS:

- **Without credentials**: Redirects to `accounts.google.com/ServiceLogin` → CORS blocked
- **With credentials**: Redirects to `googleusercontent.com` → CORS blocked (doesn't allow credentials with wildcard CORS)

Both redirect targets block cross-origin requests from extensions.

### Why Mobile Basic View Works

- Direct HTML response (no redirect)
- Accepts `credentials: 'include'` without CORS issues
- Requires the user to be logged into Google in their browser
- Returns clean document content in simple HTML format

### Key Details

- **Auth Mechanism**: Session cookies (requires `credentials: 'include'` in fetch)
- **Execution Context**: Background Service Worker (to bypass CORS/Same-Origin Policy)
- **No OAuth required**: Uses the user's existing login session
- **Parsing**: HTML response parsed with `DOMParser`, text extracted from `body.innerText`

## Implementation Requirements

### 1. Manifest Update (`manifest.json`)

Add `host_permissions` for Google Docs to allow the background script to send cross-origin requests with cookies.

**Required permissions:**

```json
{
  "host_permissions": [
    "https://docs.google.com/*",
    "https://drive.google.com/*"
  ]
}
```

**Note**: `https://drive.google.com/*` handles some redirects.

### 2. Content Script Logic

**Steps:**

1. Parse the current URL to extract the `DOC_ID`
   - Regex: `/\/document\/d\/([a-zA-Z0-9-_]+)/`
2. Send a message to the Background Worker:
   ```javascript
   { action: 'extractDocsText', docId: '...' }
   ```

### 3. Background Script Logic (`background.js`)

**Steps:**

1. Listen for `extractGoogleDocsText` message with `docId`
2. Construct mobile basic URL
3. Fetch with `credentials: 'include'` to use user's Google session
4. Return raw HTML to content script
   - **Note**: Service workers don't have DOM APIs, so HTML parsing must happen in content script

### 4. Content Script Logic (`content-google-docs.js`)

**Steps:**

1. Receive raw HTML from background script
2. Parse HTML using `DOMParser`
3. Extract text from `body.innerText`
4. Return extraction result with clean text

## Constraint Checklist

- ❌ Do not use `document.execCommand('copy')`
- ❌ Do not attempt to parse the Canvas or Accessibility DOM
- ✅ Ensure the fetch request originates from the **Background Script**, not the Content Script
- ✅ Google Docs is handled separately from other special domains (no clipboard fallback)
- ✅ This method is Google Docs specific and not applicable to other canvas-based sites

## Implementation Status

- ✅ Updated `manifest.json` with Google Docs host permissions
- ✅ Added mobile basic view extraction logic to background script
- ✅ Modified content script to detect Google Docs URLs
- ✅ Implemented parsing of mobile basic view HTML
- ✅ Added error handling for 403/404 responses

## Testing Checklist

- [x] Extract from Google Doc while logged into Google account - ✅ Working (50,805 chars, 178 chunks)
- [x] Verify clean text without UI elements (no menus, prompts, etc.) - ✅ Clean extraction
- [x] Check that extraction works for private documents - ✅ Works with user's Google session
- [x] Verify no CORS errors in console - ✅ Mobile basic view bypasses CORS issues
- [ ] Test error handling when not logged in (should show helpful error message)
- [ ] Enterprise-restricted documents (may require additional permissions or fail gracefully)
- [ ] Verify extracted documents are searchable after restart (embeddings load correctly)
