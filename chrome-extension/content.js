(() => {
  const pageTitle = document.title;
  const pageUrl = window.location.href;
  const pageContent = document.body.innerText.substring(0, 500); // Simplified content

  chrome.runtime.sendMessage({
    action: 'pageDetails',
    data: {
      title: pageTitle,
      url: pageUrl,
      content: pageContent
    }
  });
})();