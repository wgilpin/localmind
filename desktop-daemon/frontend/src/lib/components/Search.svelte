<script lang="ts">
  // @ts-nocheck
  import { searchResults, vectorResults, showResultsSection, searchStatus, searchProgress, statusMessages, retrievedDocuments, currentEventSource, stopCurrentGeneration, type SearchStatus } from '$lib/stores';

  let searchQuery = '';

  /**
   * Handles the search operation by connecting to the search-stream endpoint.
   * @returns {Promise<void>}
   */
  async function handleSearch() {
    if (!searchQuery) return;

    try {
      searchStatus.set('starting');
      searchProgress.set(statusMessages.starting);
      showResultsSection.set(true);
      searchResults.set('');
      vectorResults.set([]);
      retrievedDocuments.set([]);

      // Close any existing EventSource before opening a new one
      stopCurrentGeneration();

      const eventSource = new EventSource(`/search-stream/${encodeURIComponent(searchQuery)}`);
      currentEventSource.set(eventSource); // Store the current EventSource

      eventSource.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          
          if (data.status === 'retrieving' && data.documents) {
            retrievedDocuments.set(data.documents.map((doc: any) => ({ title: doc.title, content: doc.content, url: doc.url })));
            vectorResults.set(data.documents.map((doc: any) => ({
              id: doc.documentId,
              title: doc.title,
              url: doc.url,
              timestamp: doc.timestamp
            })));
          } else if (data.status === 'generating' && data.chunk) {
            searchResults.update(current => current + data.chunk);
          } else if (data.status === 'complete') {
            searchStatus.set('complete');
            searchProgress.set(statusMessages.complete);
            eventSource.close();
            currentEventSource.set(null); // Clear the stored EventSource
          } else if (data.status === 'error') {
            searchResults.set(data.message || 'Search failed.');
            searchStatus.set('error');
            searchProgress.set(statusMessages.error);
            eventSource.close();
            currentEventSource.set(null); // Clear the stored EventSource
          } else {
            const status = data.status as SearchStatus;
            searchStatus.set(status);
            searchProgress.set(data.message || statusMessages[status] || '');
          }
        } catch (parseError) {
          console.error('Error parsing SSE data:', parseError);
        }
      };

      eventSource.onerror = (error) => {
        console.error('SSE error:', error);
        searchStatus.set('error');
        searchProgress.set(statusMessages.error);
        searchResults.set('Connection error. Please try again.');
        eventSource.close();
        currentEventSource.set(null); // Clear the stored EventSource
      };

    } catch (error) {
      console.error('Error during search:', error);
      searchStatus.set('error');
      searchProgress.set(statusMessages.error);
      searchResults.set('Error during search. See console for details.');
      currentEventSource.set(null); // Clear the stored EventSource
    }
  }
</script>

<div class="flex space-x-2">
    <input type="text" id="search-input" placeholder="Enter search query" class="input" bind:value={searchQuery} on:keydown={(e) => e.key === 'Enter' && handleSearch()}>
    <button id="search-button" class="btn" on:click={handleSearch}>Search</button>
</div>