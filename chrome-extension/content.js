(() => {
  const pageTitle = document.title;
  const pageUrl = window.location.href;
  const pageContent = document.body.innerText; // Simplified content

  chrome.runtime.sendMessage({
    action: 'pageDetails',
    data: {
      title: pageTitle,
      url: pageUrl,
      content: pageContent
    }
  });
})();