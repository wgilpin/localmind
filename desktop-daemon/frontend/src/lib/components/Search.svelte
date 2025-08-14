<script lang="ts">
  // @ts-nocheck
  import { searchResults, vectorResults, showResultsSection, searchStatus, searchProgress, statusMessages, retrievedDocuments, currentEventSource, stopCurrentGeneration, currentSearchCutoff, resetSearchCutoff, increaseSearchCutoff, currentSearchTerm, type SearchStatus } from '$lib/stores';
  import { get } from 'svelte/store';

  let searchQuery = '';

  /**
   * Handles the search operation to retrieve relevant documents only.
   * @returns {Promise<void>}
   */
  async function handleSearch() {
    if (!searchQuery) return;

    try {
      // Reset cutoff to default when main search is triggered
      resetSearchCutoff();
      
      // Store the current search term
      currentSearchTerm.set(searchQuery);
      
      searchStatus.set('retrieving');
      searchProgress.set('Retrieving relevant documents...');
      showResultsSection.set(true);
      searchResults.set('');
      vectorResults.set([]);
      retrievedDocuments.set([]);

      // Fetch ranked chunks without generating AI answer
      const cutoff = get(currentSearchCutoff);
      const response = await fetch(`/ranked-chunks/${encodeURIComponent(searchQuery)}?cutoff=${cutoff}`);
      const data = await response.json();
      
      if (data.rankedChunks && data.rankedChunks.length > 0) {
        retrievedDocuments.set(data.rankedChunks.map((doc: any) => ({ 
          id: doc.documentId,
          title: doc.title, 
          content: doc.content, 
          url: doc.url,
          distance: doc.distance
        })));
        vectorResults.set(data.rankedChunks.map((doc: any) => ({
          id: doc.documentId,
          title: doc.title,
          url: doc.url,
          timestamp: doc.timestamp
        })));
        searchStatus.set('complete');
        searchProgress.set('Documents retrieved successfully.');
      } else {
        searchStatus.set('complete');
        searchProgress.set('No relevant documents found.');
      }
    } catch (error) {
      console.error('Error during search:', error);
      searchStatus.set('error');
      searchProgress.set('Error retrieving documents.');
    }
  }

  /**
   * Handles generating AI answer for the current search query.
   * @returns {Promise<void>}
   */
  async function handleAIAnswer() {
    if (!searchQuery || get(retrievedDocuments).length === 0) return;

    try {
      searchStatus.set('generating');
      searchProgress.set('Generating AI answer...');
      searchResults.set('');

      // Close any existing EventSource before opening a new one
      stopCurrentGeneration();

      const eventSource = new EventSource(`/search-stream/${encodeURIComponent(searchQuery)}`);
      currentEventSource.set(eventSource);

      eventSource.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          
          if (data.status === 'generating' && data.chunk) {
            searchResults.update(current => current + data.chunk);
          } else if (data.status === 'complete') {
            searchStatus.set('complete');
            searchProgress.set('AI answer generated.');
            eventSource.close();
            currentEventSource.set(null);
          } else if (data.status === 'error') {
            searchResults.set(data.message || 'AI generation failed.');
            searchStatus.set('error');
            searchProgress.set('Error generating AI answer.');
            eventSource.close();
            currentEventSource.set(null);
          }
        } catch (parseError) {
          console.error('Error parsing SSE data:', parseError);
        }
      };

      eventSource.onerror = (error) => {
        console.error('SSE error:', error);
        searchStatus.set('error');
        searchProgress.set('Connection error during AI generation.');
        searchResults.set('Connection error. Please try again.');
        eventSource.close();
        currentEventSource.set(null);
      };

    } catch (error) {
      console.error('Error during AI answer generation:', error);
      searchStatus.set('error');
      searchProgress.set('Error generating AI answer.');
      searchResults.set('Error during AI generation. See console for details.');
      currentEventSource.set(null);
    }
  }

  /**
   * Handles the "More..." search operation by increasing cutoff and rerunning search.
   * @returns {Promise<void>}
   */
  async function handleMoreSearch() {
    if (!searchQuery) return;

    const newCutoff = increaseSearchCutoff();
    if (newCutoff === null) return; // Already at maximum

    try {
      searchStatus.set('retrieving');
      searchProgress.set('Retrieving more documents...');
      
      // Clear current results to show we're updating
      vectorResults.set([]);
      retrievedDocuments.set([]);

      // Fetch ranked chunks with new cutoff
      const response = await fetch(`/ranked-chunks/${encodeURIComponent(searchQuery)}?cutoff=${newCutoff}`);
      const data = await response.json();
      
      if (data.rankedChunks && data.rankedChunks.length > 0) {
        retrievedDocuments.set(data.rankedChunks.map((doc: any) => ({ 
          id: doc.documentId,
          title: doc.title, 
          content: doc.content, 
          url: doc.url,
          distance: doc.distance
        })));
        vectorResults.set(data.rankedChunks.map((doc: any) => ({
          id: doc.documentId,
          title: doc.title,
          url: doc.url,
          timestamp: doc.timestamp
        })));
        searchStatus.set('complete');
        searchProgress.set('More documents retrieved successfully.');
      } else {
        searchStatus.set('complete');
        searchProgress.set('No additional documents found.');
      }
    } catch (error) {
      console.error('Error during more search:', error);
      searchStatus.set('error');
      searchProgress.set('Error retrieving more documents.');
    }
  }

  // Export the handleMoreSearch function so it can be called from Results component
  export { handleMoreSearch };
</script>

<div class="flex space-x-2">
    <input type="text" id="search-input" placeholder="Enter search query" class="input" bind:value={searchQuery} on:keydown={(e) => e.key === 'Enter' && handleSearch()}>
    <button id="search-button" class="btn" on:click={handleSearch}>Search</button>
    <button 
        id="ai-answer-button" 
        class="btn" 
        on:click={handleAIAnswer}
        title="Answer with AI"
        disabled={!searchQuery || get(retrievedDocuments).length === 0}
    >
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M15 6.5a1.5 1.5 0 1 0 3 0 1.5 1.5 0 1 0-3 0M4 6.5a1.5 1.5 0 1 0 3 0 1.5 1.5 0 1 0-3 0M10 12a1.5 1.5 0 1 0 3 0 1.5 1.5 0 1 0-3 0M7 18.5a1.5 1.5 0 1 0 3 0 1.5 1.5 0 1 0-3 0M15 18.5a1.5 1.5 0 1 0 3 0 1.5 1.5 0 1 0-3 0"/>
            <path d="m8.4 8.4 7.2 7.2M8.4 15.6l7.2-7.2"/>
        </svg>
    </button>
</div>