document.addEventListener('DOMContentLoaded', () => {
  const saveButton = document.getElementById('save-button');
  saveButton.addEventListener('click', () => {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      const activeTab = tabs[0];
      if (activeTab) {
        chrome.scripting.executeScript({
          target: { tabId: activeTab.id },
          files: ['content.js']
        }, () => {
          console.log('Content script executed.');
        });
      }
    });
  });
});

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.action === 'pageDetails') {
    chrome.runtime.sendMessage({
      action: 'sendPageData',
      data: message.data
    }, (response) => {
      if (response && response.success) {
        window.close(); // Close the popup on successful save
      } else {
        console.error('Save operation failed:', response ? response.error : 'No response');
      }
    });
  }
});