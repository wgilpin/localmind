/**
 * Clipboard Extraction Module
 * 
 * Handles clipboard-based content extraction for canvas-rendered domains
 * that don't support standard DOM text extraction.
 */

/**
 * Check if clipboard permissions are granted
 * @returns {Promise<boolean>} True if clipboard access is available
 */
async function checkClipboardPermission() {
  try {
    // Try to query clipboard permission
    const result = await navigator.permissions.query({ name: 'clipboard-read' });
    return result.state === 'granted' || result.state === 'prompt';
  } catch (error) {
    // Fallback: try to read clipboard directly
    try {
      await navigator.clipboard.readText();
      return true;
    } catch {
      return false;
    }
  }
}

/**
 * Read text from clipboard
 * @returns {Promise<string>} Clipboard text content
 */
async function readClipboard() {
  try {
    return await navigator.clipboard.readText();
  } catch (error) {
    console.error('Failed to read clipboard:', error);
    throw new Error('Clipboard read failed');
  }
}

/**
 * Write text to clipboard
 * @param {string} text - Text to write to clipboard
 * @returns {Promise<void>}
 */
async function writeClipboard(text) {
  try {
    await navigator.clipboard.writeText(text || '');
  } catch (error) {
    console.error('Failed to write clipboard:', error);
    throw new Error('Clipboard write failed');
  }
}

/**
 * Check if content is empty or too short
 * @param {string} content - Content to validate
 * @returns {boolean} True if content is empty/invalid
 */
function isContentEmpty(content) {
  return !content || content.trim().length < 10; // 10 character threshold
}

/**
 * Sleep utility for async delays
 * @param {number} ms - Milliseconds to sleep
 * @returns {Promise<void>}
 */
function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Main clipboard extraction workflow
 * @param {string} url - Current page URL
 * @param {number} retryCount - Number of retry attempts (prevents infinite recursion)
 * @returns {Promise<Object>} Extraction result with title, url, content, etc.
 */
async function performClipboardExtraction(url, retryCount = 0) {
  try {
    // Check clipboard permissions
    const hasPermission = await checkClipboardPermission();
    if (!hasPermission) {
      return await handlePermissionDenied(url, retryCount);
    }
    
    // Save current clipboard content
    let originalClipboard = '';
    try {
      originalClipboard = await readClipboard();
    } catch (error) {
      console.warn('Could not read original clipboard:', error);
      // Continue anyway - we'll just lose the original clipboard content
    }
    
    // Select all content on the page
    document.execCommand('selectAll');
    
    // Copy to clipboard
    const copySuccess = document.execCommand('copy');
    if (!copySuccess) {
      throw new Error('Copy command failed');
    }
    
    // Small delay to ensure clipboard operation completes
    await sleep(100);
    
    // Read extracted content from clipboard
    const extractedContent = await readClipboard();
    
    // Restore original clipboard content
    try {
      await writeClipboard(originalClipboard);
    } catch (error) {
      console.warn('Could not restore original clipboard:', error);
      // Not critical - continue
    }
    
    // Deselect all content
    if (window.getSelection) {
      window.getSelection().removeAllRanges();
    }
    
    // Validate extracted content
    if (isContentEmpty(extractedContent)) {
      return await handleEmptyContent(url, document.title);
    }
    
    return {
      title: document.title,
      url: url,
      content: extractedContent,
      extractionMethod: 'clipboard',
      success: true
    };
    
  } catch (error) {
    console.error('Clipboard extraction failed:', error);
    return await handleExtractionError(error, url);
  }
}

/**
 * Handle permission denied scenario
 * @param {string} url - Current page URL
 * @param {number} retryCount - Number of retry attempts already made
 * @returns {Promise<Object>} Result with error or fallback
 */
async function handlePermissionDenied(url, retryCount = 0) {
  const MAX_RETRIES = 1; // Allow only 1 retry to prevent stack overflow
  
  // If we've exceeded max retries, automatically fallback
  if (retryCount >= MAX_RETRIES) {
    console.warn(`Max permission retry attempts (${MAX_RETRIES}) reached, falling back to DOM extraction`);
    return {
      title: document.title,
      url: url,
      content: document.body.innerText,
      extractionMethod: 'dom',
      success: true,
      fallback: true,
      note: 'Clipboard permission denied after retries, used fallback'
    };
  }
  
  console.log(`Clipboard permission denied (attempt ${retryCount + 1}/${MAX_RETRIES + 1}), showing dialog`);
  
  // Import dialogs module if available
  if (typeof ExtractionDialogs !== 'undefined') {
    const dialogs = new ExtractionDialogs();
    
    return new Promise((resolve) => {
      dialogs.showPermissionDialog(
        // onGrant callback
        async () => {
          // User chose to grant permissions - retry extraction with incremented counter
          const result = await performClipboardExtraction(url, retryCount + 1);
          resolve(result);
        },
        // onFallback callback
        () => {
          // User chose fallback - use standard DOM extraction
          resolve({
            title: document.title,
            url: url,
            content: document.body.innerText,
            extractionMethod: 'dom',
            success: true,
            fallback: true
          });
        }
      );
    });
  } else {
    // Dialogs not available, fallback to DOM extraction
    return {
      title: document.title,
      url: url,
      content: document.body.innerText,
      extractionMethod: 'dom',
      success: true,
      fallback: true,
      note: 'Clipboard permission denied, used fallback'
    };
  }
}

/**
 * Handle empty content scenario
 * @param {string} url - Current page URL
 * @param {string} title - Page title
 * @returns {Promise<Object>} Result after user choice
 */
async function handleEmptyContent(url, title) {
  console.log('Empty content detected, showing dialog');
  
  if (typeof ExtractionDialogs !== 'undefined') {
    const dialogs = new ExtractionDialogs();
    
    return new Promise((resolve) => {
      dialogs.showEmptyContentDialog(
        // onRetry callback
        async () => {
          // User chose to retry
          const result = await performClipboardExtraction(url);
          resolve(result);
        },
        // onSaveAnyway callback
        async () => {
          // User chose to save anyway
          resolve({
            title: title,
            url: url,
            content: '', // Empty content
            extractionMethod: 'clipboard',
            success: true,
            warning: 'Saved with empty content'
          });
        }
      );
    });
  } else {
    // Dialogs not available, return empty with warning
    return {
      title: title,
      url: url,
      content: '',
      extractionMethod: 'clipboard',
      success: true,
      warning: 'Content was empty'
    };
  }
}

/**
 * Handle general extraction errors
 * @param {Error} error - The error that occurred
 * @param {string} url - Current page URL
 * @returns {Promise<Object>} Result with error info
 */
async function handleExtractionError(error, url) {
  console.error('Extraction error:', error);
  
  if (typeof ExtractionDialogs !== 'undefined') {
    const dialogs = new ExtractionDialogs();
    dialogs.showError(`Extraction failed: ${error.message}`);
  }
  
  // Fallback to DOM extraction
  return {
    title: document.title,
    url: url,
    content: document.body.innerText,
    extractionMethod: 'dom',
    success: true,
    fallback: true,
    error: error.message
  };
}


