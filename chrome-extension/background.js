chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.action === 'pageDetails') {
    const { title, content, url } = message.data;
    fetch('http://localhost:3000/documents', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ title, content, url })
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