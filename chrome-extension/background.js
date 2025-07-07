chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.action === 'sendPageData') {
    const { title, content } = message.data;
    fetch('http://localhost:3000/documents', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ title, content })
    })
    .then(response => response.json())
    .then(data => {
      console.log('Success:', data);
    })
    .catch((error) => {
      console.error('Error:', error);
    });
  }
});