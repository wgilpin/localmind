<script lang="ts">
import { searchResults, vectorResults, showResultsSection, searchStatus, searchProgress, statusMessages, retrievedDocuments, type SearchStatus } from '$lib/stores';

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

    const eventSource = new EventSource(`/search-stream/${encodeURIComponent(searchQuery)}`);
    
    eventSource.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        
        if (data.status === 'retrieving' && data.documents) {
          retrievedDocuments.set(data.documents.map((doc: any) => ({ chunk_text: doc.content })));
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
        } else if (data.status === 'error') {
          searchResults.set(data.message || 'Search failed.');
          searchStatus.set('error');
          searchProgress.set(statusMessages.error);
          eventSource.close();
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
    };

  } catch (error) {
    console.error('Error during search:', error);
    searchStatus.set('error');
    searchProgress.set(statusMessages.error);
    searchResults.set('Error during search. See console for details.');
  }
}
</script>

<div class="flex space-x-2">
    <input type="text" id="search-input" placeholder="Enter search query" class="input" bind:value={searchQuery} on:keydown={(e) => e.key === 'Enter' && handleSearch()}>
    <button id="search-button" class="btn" on:click={handleSearch}>Search</button>
</div>