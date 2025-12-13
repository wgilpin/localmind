chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.action === 'extractGoogleDocsText') {
    // Handle Google Docs export URL extraction
    const { docId } = message;
    
    if (!docId) {
      sendResponse({ success: false, error: 'No document ID provided' });
      return true;
    }
    
    console.log('Fetching Google Docs content for document:', docId);
    
    // Use mobile basic view - export URL doesn't work due to CORS restrictions
    // (it redirects to either login page or googleusercontent.com, both blocked by CORS)
    const mobileUrl = `https://docs.google.com/document/d/${docId}/mobilebasic`;
    
    console.log('Using mobile basic view (export URL blocked by CORS)');
    
    fetch(mobileUrl, {
      method: 'GET',
      credentials: 'include',  // Use user's Google session
      redirect: 'follow'
    })
    .then(response => {
      if (response.ok) {
        return response.text().then(html => {
          console.log(`Mobile basic view succeeded (${html.length} chars HTML)`);
          sendResponse({ 
            success: true, 
            html: html,  // HTML that will be parsed in content script
            method: 'mobilebasic' 
          });
        });
      } else {
        console.error('Mobile basic view failed:', response.status, response.statusText);
        sendResponse({ 
          success: false, 
          error: `Mobile view failed: ${response.status} ${response.statusText}. Make sure you're logged into Google Docs in this browser.`
        });
      }
    })
    .catch(error => {
      console.error('Google Docs export fetch error:', error);
      sendResponse({ 
        success: false, 
        error: error.message
      });
    });
    
    return true; // Indicates async response
  } else if (message.action === 'sendPageData') {
    const { title, content, url, extractionMethod } = message.data;
    
    // Log extraction method for debugging
    console.log(`Sending document via ${extractionMethod || 'unknown'} extraction: ${title}`);
    
    fetch('http://localhost:3000/documents', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ 
        title, 
        content, 
        url, 
        extractionMethod: extractionMethod || 'dom' 
      })
    })
    .then(response => {
      if (response.ok) {
        return response.json().then(data => {
          console.log('Success:', data);
          sendResponse({ success: true });
        });
      } else {
        return response.json().then(error => {
          console.error('Error:', error);
          const errorMessage = error.message || JSON.stringify(error);
          sendResponse({ success: false, error: errorMessage });
        }).catch(() => {
          // If response is not JSON, return status text
          sendResponse({ 
            success: false, 
            error: `Server returned error: ${response.status} ${response.statusText}` 
          });
        });
      }
    })
    .catch((error) => {
      console.error('Error:', error);
      let errorMessage = error.message || 'Unknown error';
      
      // Provide more helpful error messages for common issues
      if (errorMessage.includes('Failed to fetch') || 
          errorMessage.includes('NetworkError') ||
          errorMessage.includes('ERR_CONNECTION_REFUSED')) {
        errorMessage = 'Cannot connect to LocalMind HTTP server. Please ensure the LocalMind desktop app is running and the HTTP server is listening on http://localhost:3000. Check the app console for "HTTP server listening" message.';
      }
      
      sendResponse({ success: false, error: errorMessage });
    });
    return true; // Indicates that sendResponse will be called asynchronously
  } else if (message.action === 'addNote') {
    const { url, note } = message.data;
    fetch('http://localhost:3000/notes', { // Assuming a new endpoint for notes
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ url, note })
    })
    .then(response => {
      if (response.ok) {
        return response.json().then(data => {
          console.log('Note added successfully:', data);
          sendResponse({ success: true });
        });
      } else {
        return response.json().then(error => {
          console.error('Error adding note:', error);
          const errorMessage = error.message || JSON.stringify(error);
          sendResponse({ success: false, error: errorMessage });
        }).catch(() => {
          sendResponse({ 
            success: false, 
            error: `Server returned error: ${response.status} ${response.statusText}` 
          });
        });
      }
    })
    .catch((error) => {
      console.error('Error adding note:', error);
      let errorMessage = error.message || 'Unknown error';
      
      if (errorMessage.includes('Failed to fetch') || 
          errorMessage.includes('NetworkError') ||
          errorMessage.includes('ERR_CONNECTION_REFUSED')) {
        errorMessage = 'Cannot connect to LocalMind backend server. Please ensure the desktop-daemon server is running on http://localhost:3000.';
      }
      
      sendResponse({ success: false, error: errorMessage });
    });
    return true; // Indicates that sendResponse will be called asynchronously
  }
});