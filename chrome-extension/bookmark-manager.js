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
    
    // Strip query parameters (everything after ?)
    // Example: https://example.com/page?foo=bar → https://example.com/page
    url.search = '';
    
    // Strip fragments (everything after #)
    // Example: https://example.com/page#section → https://example.com/page
    url.hash = '';
    
    // Special handling for Google Docs (docs.google.com only)
    // Google Docs URLs have trailing verbs like /edit, /view, /preview, /copy
    // These should be removed for duplicate detection since they refer to the same document
    // Example: https://docs.google.com/document/d/ABC123/edit → https://docs.google.com/document/d/ABC123
    if (url.hostname === 'docs.google.com') {
      // Remove trailing verbs: /edit, /view, /preview, /copy (case-insensitive regex)
      url.pathname = url.pathname.replace(/\/(edit|view|preview|copy)$/i, '');
    }
    
    // Normalize trailing slashes (remove trailing slash from path, except root)
    // Ensures consistent comparison: https://example.com/page/ === https://example.com/page
    if (url.pathname !== '/') {
      url.pathname = url.pathname.replace(/\/$/, '');
    }
    
    return url.toString();
  } catch (error) {
    // Invalid URL format - return original string to avoid breaking bookmark creation
    // This handles edge cases like malformed URLs or relative URLs
    console.error('Invalid URL:', urlString, error);
    return urlString;
  }
}

/**
 * Finds existing bookmark matching normalized URL
 * Searches all bookmarks across all folders and compares normalized URLs
 * 
 * @param {string} normalizedUrl - Already-normalized URL to search for
 * @returns {Promise<chrome.bookmarks.BookmarkTreeNode|null>} Existing bookmark if found, null otherwise
 */
async function findExistingBookmark(normalizedUrl) {
  try {
    // Get all bookmarks (empty query returns all)
    const allBookmarks = await chrome.bookmarks.search({});
    
    // Search for bookmark with matching normalized URL
    for (const bookmark of allBookmarks) {
      // Only check bookmarks (not folders - folders don't have URLs)
      if (!bookmark.url) continue;
      
      // Normalize existing bookmark's URL and compare
      const existingNormalized = normalizeUrl(bookmark.url);
      if (existingNormalized === normalizedUrl) {
        return bookmark;
      }
    }
    
    return null;
  } catch (error) {
    console.error('Error searching for existing bookmark:', error);
    return null;
  }
}

/**
 * Finds "inbox" folder at root level (case-insensitive)
 * Searches only in Bookmark Bar and Other Bookmarks root folders, ignoring nested folders
 * 
 * @returns {Promise<chrome.bookmarks.BookmarkTreeNode|null>} Inbox folder if found, null otherwise
 */
async function findInboxFolder() {
  try {
    const tree = await chrome.bookmarks.getTree();
    
    // Get root level folders: Bookmark Bar (id '1') and Other Bookmarks (id '2')
    const bookmarkBar = tree[0].children?.find(n => n.id === '1');
    const otherBookmarks = tree[0].children?.find(n => n.id === '2');
    
    // Search root level only (ignore nested folders)
    const searchRoots = [bookmarkBar, otherBookmarks].filter(Boolean);
    
    for (const root of searchRoots) {
      if (!root.children) continue;
      
      // Find first folder with case-insensitive "inbox" name at root level
      const inbox = root.children.find(child => 
        child.title && 
        child.title.toLowerCase() === 'inbox' && 
        !child.url  // Must be a folder (no url property)
      );
      
      if (inbox) {
        return inbox;
      }
    }
    
    return null;
  } catch (error) {
    console.error('Error finding inbox folder:', error);
    return null;
  }
}

/**
 * Creates inbox folder if it doesn't exist
 * Attempts to create "inbox" folder in Bookmark Bar (id '1')
 * 
 * @returns {Promise<chrome.bookmarks.BookmarkTreeNode|null>} Created inbox folder if successful, null if creation fails
 */
async function createInboxFolder() {
  try {
    // Create inbox folder in Bookmark Bar (id '1')
    const inboxFolder = await chrome.bookmarks.create({
      parentId: '1',  // Bookmark Bar
      title: 'inbox'
    });
    
    console.log('Inbox folder created:', inboxFolder.id);
    return inboxFolder;
  } catch (error) {
    // Folder creation failed (permissions, quota, etc.)
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
    // Truncate to 255 chars (Chrome bookmark title limit)
    return tab.title.trim().slice(0, 255);
  }
  
  // Fallback: Extract first text from page content using TreeWalker
  // This is used when document.title is empty or missing (e.g., SPA pages, canvas-only pages)
  try {
    const results = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: () => {
        // Use TreeWalker API to traverse DOM and find first meaningful text node
        // TreeWalker is efficient and skips script/style tags automatically
        const walker = document.createTreeWalker(
          document.body,
          NodeFilter.SHOW_TEXT,  // Only visit text nodes
          {
            acceptNode: (node) => {
              const text = node.textContent.trim();
              const parent = node.parentElement;
              
              // Accept text nodes that:
              // 1. Have meaningful length (>10 chars to skip single words/punctuation)
              // 2. Are not inside script, style, or noscript tags (invisible content)
              if (text.length > 10 && 
                  parent && 
                  !parent.closest('script, style, noscript')) {
                return NodeFilter.FILTER_ACCEPT;
              }
              return NodeFilter.FILTER_SKIP;
            }
          }
        );
        
        // Get first matching text node
        const firstText = walker.nextNode();
        // Return first ~50 chars of meaningful text (readable title length)
        return firstText ? firstText.textContent.trim().slice(0, 50) : null;
      }
    });
    
    if (results[0]?.result) {
      return results[0].result;
    }
  } catch (error) {
    // Content script injection may fail on restricted pages (chrome://, file://)
    // Log warning but don't throw - fall back to 'Untitled'
    console.warn('Could not extract page title from content:', error);
  }
  
  // Last resort: return 'Untitled'
  return 'Untitled';
}

/**
 * Main function: Save current page as bookmark with deduplication and inbox folder organization
 * 
 * @returns {Promise<boolean>} True if bookmark created, false if duplicate/error
 */
async function savePageAsBookmark() {
  try {
    // Get active tab
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (!tab || !tab.url) {
      console.error('No active tab found or tab has no URL');
      return false;
    }
    
    // Skip chrome:// and chrome-extension:// URLs (can't bookmark these)
    if (tab.url.startsWith('chrome://') || tab.url.startsWith('chrome-extension://')) {
      console.error('Cannot bookmark Chrome internal pages');
      return false;
    }
    
    const pageUrl = tab.url;
    
    // Normalize URL for duplicate detection
    const normalizedUrl = normalizeUrl(pageUrl);
    
    // Check for existing bookmark with same normalized URL
    const existingBookmark = await findExistingBookmark(normalizedUrl);
    if (existingBookmark) {
      // Duplicate detected - skip creation silently per spec
      console.log('Duplicate bookmark detected, skipping creation:', existingBookmark.title, existingBookmark.url);
      return false;
    }
    
    // Get page title (with fallback to content extraction)
    const pageTitle = await getPageTitle(tab);
    
    // Find inbox folder (searches root level only, case-insensitive)
    let inboxFolder = await findInboxFolder();
    
    // If inbox folder doesn't exist, attempt to create it
    if (!inboxFolder) {
      inboxFolder = await createInboxFolder();
    }
    
    // Determine parent ID: use inbox folder if found/created, otherwise fallback to Bookmark Bar (id '1')
    const parentId = inboxFolder ? inboxFolder.id : '1';
    
    // Create bookmark (use original URL, not normalized)
    const newBookmark = await chrome.bookmarks.create({
      parentId: parentId,
      title: pageTitle,
      url: pageUrl  // Save original URL with query params/fragments if present
    });
    
    console.log('Bookmark created:', newBookmark.title, 'in', inboxFolder ? 'inbox folder' : 'Bookmark Bar');
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
