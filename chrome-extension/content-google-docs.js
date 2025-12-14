/**
 * Google Docs Extraction Module
 * 
 * Handles content extraction from Google Docs using the export URL method
 * to bypass canvas rendering limitations.
 */

/**
 * Check if current URL is a Google Docs document
 * @param {string} url - URL to check
 * @returns {boolean} True if URL is a Google Docs document
 */
function isGoogleDocsUrl(url) {
  return url.includes('docs.google.com/document/d/');
}

/**
 * Extract document ID from Google Docs URL
 * @param {string} url - Google Docs URL
 * @returns {string|null} Document ID or null if not found
 */
function extractGoogleDocsId(url) {
  const match = url.match(/\/document\/d\/([a-zA-Z0-9-_]+)/);
  return match ? match[1] : null;
}

/**
 * Extract content from Google Docs using export URL method
 * @param {string} url - Current page URL
 * @returns {Promise<Object>} Extraction result with title, url, content, etc.
 */
async function performGoogleDocsExtraction(url) {
  try {
    // Extract document ID
    const docId = extractGoogleDocsId(url);
    
    if (!docId) {
      console.error('Could not extract document ID from URL:', url);
      return {
        title: document.title,
        url: url,
        content: '',
        extractionMethod: 'google-docs-export',
        success: false,
        error: 'Could not extract document ID from URL'
      };
    }
    
    console.log('Requesting Google Docs export for document:', docId);
    
    // Request background script to fetch export URL
    // Background script is needed because it can make cross-origin requests with credentials
    const response = await new Promise((resolve, reject) => {
      chrome.runtime.sendMessage(
        { action: 'extractGoogleDocsText', docId: docId },
        (response) => {
          if (chrome.runtime.lastError) {
            reject(new Error(chrome.runtime.lastError.message));
          } else {
            resolve(response);
          }
        }
      );
    });
    
    if (response.success) {
      const method = response.method || 'mobilebasic';
      
      // Parse HTML from mobile basic view
      if (!response.html) {
        console.error('No HTML in response');
        return {
          title: document.title,
          url: url,
          content: '',
          extractionMethod: `google-docs-${method}`,
          success: false,
          error: 'No HTML received from background script'
        };
      }
      
      console.log('Parsing HTML from mobile basic view...');
      console.log(`Raw HTML length: ${response.html.length} chars`);
      let content;
      try {
        const parser = new DOMParser();
        const doc = parser.parseFromString(response.html, 'text/html');
        content = doc.body.innerText || doc.body.textContent || '';
        console.log(`Parsed HTML to text (${content.length} chars)`);
        console.log(`First 500 chars: ${content.substring(0, 500)}`);
        console.log(`Last 500 chars: ${content.substring(Math.max(0, content.length - 500))}`);
      } catch (parseError) {
        console.error('Failed to parse HTML:', parseError);
        return {
          title: document.title,
          url: url,
          content: '',
          extractionMethod: `google-docs-${method}`,
          success: false,
          error: 'Failed to parse HTML in content script'
        };
      }
      
      console.log(`Google Docs extraction succeeded (${content.length} chars)`);
      console.log('First 200 chars of content:', content.substring(0, 200));
      
      return {
        title: document.title,
        url: url,
        content: content,
        extractionMethod: `google-docs-${method}`,
        success: true
      };
    } else {
      console.error('Google Docs extraction failed:', response.error);
      console.error('Full response:', response);
      return {
        title: document.title,
        url: url,
        content: '',
        extractionMethod: 'google-docs-export',
        success: false,
        error: response.error || 'Unknown extraction error'
      };
    }
    
  } catch (error) {
    console.error('Google Docs extraction error:', error);
    return {
      title: document.title,
      url: url,
      content: '',
      extractionMethod: 'google-docs-export',
      success: false,
      error: error.message
    };
  }
}
