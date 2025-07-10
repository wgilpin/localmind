chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.action === 'sendPageData') {
    const { title, content, url } = message.data;
    fetch('http://localhost:3000/documents', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ title, content, url })
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
  }
});