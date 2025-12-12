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
          sendResponse({ success: false, error: error });
        });
      }
    })
    .catch((error) => {
      console.error('Error:', error);
      sendResponse({ success: false, error: error.message });
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
          sendResponse({ success: false, error: error });
        });
      }
    })
    .catch((error) => {
      console.error('Error adding note:', error);
      sendResponse({ success: false, error: error.message });
    });
    return true; // Indicates that sendResponse will be called asynchronously
  }
});