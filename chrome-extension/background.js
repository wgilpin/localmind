chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.action === 'sendPageData') {
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
        errorMessage = 'Cannot connect to LocalMind backend server. Please ensure the desktop-daemon server is running on http://localhost:3000. You can start it by running "npm run dev" from the desktop-daemon directory.';
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