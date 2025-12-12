document.addEventListener('DOMContentLoaded', () => {
  const saveButton = document.getElementById('save-button');
  const showNoteInputButton = document.getElementById('show-note-input-button');
  const addNoteButton = document.getElementById('add-note-button');
  const noteContentArea = document.getElementById('note-content');
  const noteInputContainer = document.getElementById('note-input-container');
  const statusMessage = document.getElementById('status-message');

  saveButton.addEventListener('click', () => {
    statusMessage.textContent = 'Extracting content...'; // Clear previous messages
    statusMessage.style.color = 'blue';
    
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      const activeTab = tabs[0];
      if (activeTab) {
        // Inject required scripts in sequence
        chrome.scripting.executeScript({
          target: { tabId: activeTab.id },
          files: ['config-manager.js', 'content-clipboard.js', 'ui/dialogs.js', 'content.js']
        }, () => {
          console.log('Content scripts executed.');
        });
      }
    });
  });

  showNoteInputButton.addEventListener('click', () => {
    noteInputContainer.classList.remove('hidden');
    showNoteInputButton.style.display = 'none'; // Hide the "Add Note" button
    saveButton.style.display = 'none'; // Hide the "Save Page" button
    statusMessage.textContent = ''; // Clear previous messages
  });

  addNoteButton.addEventListener('click', () => {
    statusMessage.textContent = ''; // Clear previous messages
    const note = noteContentArea.value;
    if (note.trim() === '') {
      statusMessage.textContent = 'Note cannot be empty!';
      statusMessage.style.color = 'orange';
      return;
    }

    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      const activeTab = tabs[0];
      if (activeTab) {
        chrome.runtime.sendMessage({
          action: 'addNote',
          data: {
            url: activeTab.url,
            note: note
          }
        }, (response) => {
          if (response && response.success) {
            statusMessage.textContent = 'Note added successfully!';
            statusMessage.style.color = 'green';
            noteContentArea.value = ''; // Clear the textarea
            setTimeout(() => {
              window.close(); // Close the popup after a short delay
            }, 1000); // 1 second delay
          } else {
            statusMessage.textContent = `Error: ${response ? response.error : 'Unknown error'}`;
            statusMessage.style.color = 'red';
            console.error('Add note operation failed:', response ? response.error : 'No response');
          }
        });
      }
    });
  });
});

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.action === 'pageDetails') {
    const statusMessage = document.getElementById('status-message');
    
    // Check extraction method and show appropriate status
    if (message.data.extractionMethod === 'clipboard') {
      statusMessage.textContent = 'Using clipboard extraction for canvas content...';
      statusMessage.style.color = 'blue';
    } else if (message.data.fallback) {
      statusMessage.textContent = 'Using fallback extraction...';
      statusMessage.style.color = 'orange';
    }
    
    chrome.runtime.sendMessage({
      action: 'sendPageData',
      data: message.data
    }, (response) => {
      if (response && response.success) {
        statusMessage.textContent = 'Page saved successfully!';
        statusMessage.style.color = 'green';
        setTimeout(() => {
          window.close(); // Close the popup after a short delay
        }, 1000); // 1 second delay
      } else {
        statusMessage.textContent = `Error: ${response ? response.error : 'Unknown error'}`;
        statusMessage.style.color = 'red';
        console.error('Save operation failed:', response ? response.error : 'No response');
      }
    });
  }
});