(async () => {
  try {
    // Initialize ConfigManager
    const configManager = new ConfigManager();
    await configManager.loadConfig();
    
    const currentUrl = window.location.href;
    let extractionData;
    
    // Check if this is a Google Docs URL - prioritize export method
    if (typeof isGoogleDocsUrl === 'function' && isGoogleDocsUrl(currentUrl)) {
      console.log('Google Docs detected, using export URL extraction');
      extractionData = await performGoogleDocsExtraction(currentUrl);
    } else {
      // Check for other special domains
      const isSpecial = configManager.isSpecialDomain(currentUrl);
      
      if (isSpecial) {
        // Use clipboard-based extraction for special domains
        console.log('Special domain detected, using clipboard extraction');
        extractionData = await performClipboardExtraction(currentUrl);
      } else {
        // Standard DOM extraction for regular domains
        console.log('Standard domain, using DOM extraction');
        extractionData = {
          title: document.title,
          url: currentUrl,
          content: document.body.innerText,
          extractionMethod: 'dom',
          success: true
        };
      }
    }
    
    // Send extracted data to popup
    chrome.runtime.sendMessage({
      action: 'pageDetails',
      data: extractionData
    });
    
  } catch (error) {
    console.error('Content extraction failed:', error);
    
    // Fallback to basic extraction
    chrome.runtime.sendMessage({
      action: 'pageDetails',
      data: {
        title: document.title,
        url: window.location.href,
        content: document.body.innerText,
        extractionMethod: 'dom',
        success: true,
        error: error.message
      }
    });
  }
})();