<script lang="ts">
import { searchResults, vectorResults, showResultsSection, searchStatus, searchProgress, statusMessages, retrievedDocuments, type SearchStatus } from '$lib/stores';

let searchQuery = '';

/**
 * Handles the search operation, first retrieving documents and then triggering the AI response stream.
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

    // First, get documents from /search endpoint
    searchProgress.set('Retrieving documents...');
    const searchResponse = await fetch(`/search/${encodeURIComponent(searchQuery)}`);

    if (!searchResponse.ok) {
      throw new Error(`HTTP error! status: ${searchResponse.status}`);
    }

    const searchData = await searchResponse.json();
    vectorResults.set(searchData.vectorResults || []);
    retrievedDocuments.set(searchData.vectorResults.map((doc: { chunk_text: string; }) => ({ chunk_text: doc.chunk_text })) || []);
    
    searchProgress.set('Documents retrieved. Waiting for AI response...');

    // Then start the LLM search with streaming
    const eventSource = new EventSource(`/search-stream/${encodeURIComponent(searchQuery)}`);
    
    eventSource.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        
        if (data.status === 'result') {
          searchResults.set(data.result || 'No results found.');
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