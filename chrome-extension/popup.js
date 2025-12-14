/**
 * Debounce helper function - prevents rapid repeated function calls
 * @param {Function} func - Function to debounce
 * @param {number} delayMs - Delay in milliseconds
 * @returns {Function} Debounced function
 */
function debounce(func, delayMs) {
  let isDebouncing = false;
  
  return function debounced(...args) {
    if (isDebouncing) return;
    
    isDebouncing = true;
    func.apply(this, args);
    
    setTimeout(() => {
      isDebouncing = false;
    }, delayMs);
  };
}

document.addEventListener('DOMContentLoaded', () => {
  const saveButton = document.getElementById('save-button');
  const showNoteInputButton = document.getElementById('show-note-input-button');
  const addNoteButton = document.getElementById('add-note-button');
  const noteContentArea = document.getElementById('note-content');
  const noteInputContainer = document.getElementById('note-input-container');
  const statusMessage = document.getElementById('status-message');

  // Save Page button handler - does BOTH: saves Chrome bookmark AND extracts/sends content to LocalMind
  const debouncedSavePage = debounce(async () => {
    try {
      // Step 1: Save as Chrome bookmark (with deduplication and inbox organization)
      const bookmarkSuccess = await savePageAsBookmark();
      if (bookmarkSuccess) {
        console.log('Bookmark saved successfully');
      } else {
        console.log('Bookmark save failed or skipped (duplicate)');
      }
      
      // Step 2: Extract content and send to LocalMind
      statusMessage.textContent = 'Extracting content...';
      statusMessage.style.color = 'blue';
      
      chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
        const activeTab = tabs[0];
        if (activeTab) {
          // Inject required scripts in sequence
          chrome.scripting.executeScript({
            target: { tabId: activeTab.id },
            files: ['config-manager.js', 'content-clipboard.js', 'content-google-docs.js', 'ui/dialogs.js', 'content.js']
          }, () => {
            console.log('Content scripts executed.');
          });
        }
      });
    } catch (error) {
      console.error('Error saving page:', error);
      statusMessage.textContent = 'Error saving page';
      statusMessage.style.color = 'red';
    }
  }, 1500); // 1.5 seconds debounce per spec

  saveButton.addEventListener('click', debouncedSavePage);

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
    
    // Check if extraction failed
    if (message.data.success === false) {
      const errorMsg = message.data.error || 'Extraction failed';
      statusMessage.textContent = `Extraction failed: ${errorMsg}`;
      statusMessage.style.color = 'red';
      console.error('Content extraction failed:', message.data);
      return;
    }
    
    // Check if content is empty
    if (!message.data.content || message.data.content.trim().length === 0) {
      statusMessage.textContent = 'Extraction returned empty content. Please try again or check if the document has text.';
      statusMessage.style.color = 'orange';
      console.warn('Empty content extracted:', message.data);
      return;
    }
    
    // Check extraction method and show appropriate status
    if (message.data.extractionMethod && message.data.extractionMethod.startsWith('google-docs-')) {
      const method = message.data.extractionMethod.replace('google-docs-', '');
      statusMessage.textContent = `Using Google Docs ${method} extraction...`;
      statusMessage.style.color = 'blue';
    } else if (message.data.extractionMethod === 'clipboard') {
      statusMessage.textContent = 'Using clipboard extraction for canvas content...';
      statusMessage.style.color = 'blue';
    } else if (message.data.fallback) {
      statusMessage.textContent = 'Using fallback extraction...';
      statusMessage.style.color = 'orange';
    }
    
    // Log what we're about to send
    console.log('Sending page data:', {
      title: message.data.title,
      contentLength: message.data.content?.length || 0,
      url: message.data.url,
      method: message.data.extractionMethod
    });
    
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