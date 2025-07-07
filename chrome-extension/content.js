(() => {
  const pageTitle = document.title;
  const pageContent = document.body.innerText.substring(0, 500); // Simplified content

  chrome.runtime.sendMessage({
    action: 'pageDetails',
    data: {
      title: pageTitle,
      content: pageContent
    }
  });
})();