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
 * Read text from clipboard using Clipboard API
 * @returns {Promise<string>} Clipboard text content
 */
async function readClipboard() {
  try {
    // Ensure window/document is focused before reading clipboard
    // This is required for clipboard API access
    if (document.hasFocus && !document.hasFocus()) {
      window.focus();
      // Small delay to allow focus to take effect
      await sleep(50);
    }
    return await navigator.clipboard.readText();
  } catch (error) {
    // Check if error is due to focus/permission issues
    if (error.name === 'NotAllowedError' || error.message.includes('not focused')) {
      // Silently fail - will be handled by fallback logic
      throw new Error('Clipboard read failed: Document not focused');
    }
    console.error('Failed to read clipboard:', error);
    throw new Error('Clipboard read failed');
  }
}

/**
 * Read clipboard content via hidden textarea paste workaround
 * This can work even when Clipboard API requires focus, since paste events
 * can be triggered programmatically in some contexts
 * @returns {Promise<string>} Clipboard text content
 */
async function readClipboardViaTextarea() {
  return new Promise((resolve, reject) => {
    // Create a hidden textarea element
    const textarea = document.createElement('textarea');
    textarea.style.position = 'fixed';
    textarea.style.left = '-9999px';
    textarea.style.top = '-9999px';
    textarea.style.width = '1px';
    textarea.style.height = '1px';
    textarea.style.opacity = '0';
    textarea.setAttribute('readonly', '');
    document.body.appendChild(textarea);
    
    let resolved = false;
    const cleanup = () => {
      if (!resolved) {
        resolved = true;
        if (textarea.parentNode) {
          document.body.removeChild(textarea);
        }
        document.removeEventListener('paste', pasteHandler);
      }
    };
    
    // Try to paste (this might work even when clipboard.readText() doesn't)
    const pasteHandler = (e) => {
      e.preventDefault();
      e.stopPropagation();
      
      const pastedText = (e.clipboardData || window.clipboardData).getData('text');
      cleanup();
      
      if (pastedText && pastedText.trim().length > 0) {
        console.log(`Textarea paste workaround succeeded (${pastedText.length} chars)`);
        resolve(pastedText);
      } else {
        // Also check textarea.value as fallback
        const textareaValue = textarea.value;
        if (textareaValue && textareaValue.trim().length > 0) {
          console.log(`Textarea paste workaround succeeded via value (${textareaValue.length} chars)`);
          resolve(textareaValue);
        } else {
          reject(new Error('Paste returned empty content'));
        }
      }
    };
    
    document.addEventListener('paste', pasteHandler, { once: true, capture: true });
    
    // Focus the textarea and try paste command
    try {
      textarea.focus();
      textarea.select();
      
      // Small delay to ensure focus
      setTimeout(() => {
        try {
          const pasteSuccess = document.execCommand('paste');
          console.log(`execCommand('paste') result: ${pasteSuccess}`);
          
          // Wait a bit for paste event to fire
          setTimeout(() => {
            if (!resolved) {
              // Check if textarea has content even if event didn't fire
              const textareaValue = textarea.value;
              if (textareaValue && textareaValue.trim().length > 0) {
                console.log(`Textarea has content via direct value check (${textareaValue.length} chars)`);
                cleanup();
                resolve(textareaValue);
              } else if (!pasteSuccess) {
                cleanup();
                reject(new Error('Paste command failed and no content in textarea'));
              } else {
                // Give it a bit more time for async paste
                setTimeout(() => {
                  if (!resolved) {
                    cleanup();
                    reject(new Error('Paste command succeeded but no content received'));
                  }
                }, 200);
              }
            }
          }, 150);
        } catch (error) {
          cleanup();
          reject(error);
        }
      }, 50);
    } catch (error) {
      cleanup();
      reject(error);
    }
  });
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
      // If clipboard read fails due to focus/permission, note it but continue
      // We'll try the copy operation anyway - it might work with user interaction
      if (error.message && error.message.includes('not focused')) {
        console.log('Could not read original clipboard (focus issue) - will try copy anyway');
      }
      console.warn('Could not read original clipboard:', error);
      // Continue anyway - we'll just lose the original clipboard content
    }
    
    // Ensure document is focused before clipboard operations
    if (document.hasFocus && !document.hasFocus()) {
      window.focus();
      await sleep(100); // Allow focus to take effect
    }
    
    // Try to focus the document editing area for Google Docs
    // Google Docs uses iframes - try to find and focus the main editing iframe
    let docFocused = false;
    try {
      // Try to find the main document iframe
      const iframes = document.querySelectorAll('iframe');
      for (const iframe of iframes) {
        try {
          // Check if this is likely the document iframe (Google Docs uses specific IDs/classes)
          if (iframe.id && (iframe.id.includes('docs-texteventtarget') || iframe.id.includes('kix-'))) {
            iframe.focus();
            await sleep(50);
            docFocused = true;
            break;
          }
        } catch (e) {
          // Can't access iframe content (cross-origin) - that's expected
        }
      }
      
      // Also try clicking on the document area to ensure focus
      const docElement = document.querySelector('[role="textbox"]') || 
                        document.querySelector('.kix-page-content-wrapper') ||
                        document.querySelector('.kix-page');
      if (docElement) {
        docElement.focus();
        await sleep(50);
        docFocused = true;
      }
    } catch (e) {
      console.log('Could not focus document area, will try selectAll anyway');
    }
    
    // Try keyboard shortcut Ctrl+A (Cmd+A on Mac) which works better for Google Docs
    // First, ensure we're in the document area
    const docArea = document.querySelector('[role="textbox"]') || document.body;
    docArea.focus();
    await sleep(100);
    
    // Try using keyboard event for select all (more reliable for Google Docs)
    try {
      const selectAllEvent = new KeyboardEvent('keydown', {
        key: 'a',
        code: 'KeyA',
        ctrlKey: true,
        metaKey: navigator.platform.includes('Mac'),
        bubbles: true,
        cancelable: true
      });
      docArea.dispatchEvent(selectAllEvent);
      await sleep(100);
    } catch (e) {
      console.log('Keyboard event failed, falling back to execCommand');
    }
    
    // Select all content on the page
    const selectSuccess = document.execCommand('selectAll');
    if (!selectSuccess) {
      // Try again with a small delay
      await sleep(100);
      if (!document.execCommand('selectAll')) {
        throw new Error('Select all command failed');
      }
    }
    
    await sleep(100); // Give selection time to complete
    
    // Copy to clipboard
    const copySuccess = document.execCommand('copy');
    if (!copySuccess) {
      throw new Error('Copy command failed');
    }
    
    // Small delay to ensure clipboard operation completes
    await sleep(100);
    
    // Read extracted content from clipboard
    // Try Clipboard API first, then fallback to textarea paste method
    let extractedContent;
    try {
      extractedContent = await readClipboard();
      console.log(`Clipboard API read succeeded (${extractedContent.length} chars)`);
    } catch (error) {
      // If Clipboard API fails, try workaround: paste into hidden textarea
      console.log('Clipboard API read failed, trying textarea paste workaround:', error.message);
      try {
        extractedContent = await readClipboardViaTextarea();
        console.log(`Textarea paste workaround succeeded (${extractedContent.length} chars)`);
      } catch (textareaError) {
        // Both methods failed - this is a real error
        console.error('Both clipboard read methods failed:', {
          clipboardAPI: error.message,
          textarea: textareaError.message
        });
        if (error.message && error.message.includes('not focused')) {
          console.log('Could not read clipboard after copy - focus/permission issue');
          return await handleExtractionError(error, url);
        }
        throw error; // Re-throw original error
      }
    }
    
    // Restore original clipboard content (non-critical if it fails)
    if (originalClipboard) {
      try {
        await writeClipboard(originalClipboard);
        console.log('Original clipboard restored successfully');
      } catch (error) {
        // Not critical - user's clipboard now has the extracted content which is fine
        console.warn('Could not restore original clipboard (non-critical):', error.message);
      }
    } else {
      console.log('No original clipboard to restore');
    }
    
    // Deselect all content
    if (window.getSelection) {
      window.getSelection().removeAllRanges();
    }
    
    // Clean up extracted content - remove common Google Docs UI elements
    let cleanedContent = extractedContent;
    
    // Remove common Google Docs UI text patterns
    const uiPatterns = [
      /Turn on screen reader support.*?Banner hidden/gs,
      /To enable screen reader support.*?Ctrl\+slash/gs,
      /What can Gemini do in Google Docs/gs,
      /Refine this document/gs,
      /Rephrase part of this document/gs,
      /Gemini in Workspace can make mistakes.*?Learn more/gs,
      /Spark fresh ideas/gs,
      /Gemini/gs,
      /^\d+%$/gm, // Percentage values
      /^\d+$/gm, // Standalone numbers (might be page numbers)
    ];
    
    for (const pattern of uiPatterns) {
      cleanedContent = cleanedContent.replace(pattern, '');
    }
    
    // Clean up extra whitespace
    cleanedContent = cleanedContent
      .split('\n')
      .map(line => line.trim())
      .filter(line => line.length > 0)
      .join('\n')
      .trim();
    
    // Validate cleaned content
    if (isContentEmpty(cleanedContent)) {
      // If cleaned content is empty but original wasn't, warn user
      if (!isContentEmpty(extractedContent)) {
        console.warn('Content extraction captured mostly UI elements. Google Docs canvas content may not be accessible.');
        // Return original content with a note
        return {
          title: document.title,
          url: url,
          content: extractedContent,
          extractionMethod: 'clipboard',
          success: true,
          warning: 'Extracted content appears to be mostly UI elements. For better results, manually select the document text before saving.'
        };
      }
      return await handleEmptyContent(url, document.title);
    }
    
    return {
      title: document.title,
      url: url,
      content: cleanedContent,
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
  
  // For special domains (like Google Docs), DOM extraction won't work
  // So we need to inform the user and provide retry option
  if (typeof ExtractionDialogs !== 'undefined') {
    const dialogs = new ExtractionDialogs();
    
    return new Promise((resolve) => {
      // Show error with retry option
      dialogs.showError(
        `Clipboard extraction failed: ${error.message}\n\nFor canvas-rendered pages like Google Docs, clipboard access is required.\n\nPlease try:\n1. Clicking the bookmark button again (user interaction helps)\n2. Ensuring the page tab is focused\n3. Granting clipboard permissions if prompted`
      );
      
      // For now, return error result - user can retry manually
      // In the future, we could add a retry button to the dialog
      resolve({
        title: document.title,
        url: url,
        content: '',
        extractionMethod: 'clipboard',
        success: false,
        error: error.message,
        note: 'Clipboard extraction failed - please retry with user interaction'
      });
    });
  }
  
  // No dialogs available - return error result (don't fallback to DOM for special domains)
  return {
    title: document.title,
    url: url,
    content: '',
    extractionMethod: 'clipboard',
    success: false,
    error: error.message,
    note: 'Clipboard extraction failed - DOM extraction not available for canvas-rendered pages. Please retry with user interaction.'
  };
}


