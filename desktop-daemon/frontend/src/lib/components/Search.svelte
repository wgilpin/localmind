<script lang="ts">
import { searchResults, showResultsSection } from '$lib/stores';

let searchQuery = '';

async function handleSearch() {
  if (!searchQuery) return;

  try {
    const response = await fetch('/search', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ query: searchQuery }),
    });

    if (!response.ok) {
      throw new Error('Search request failed');
    }

    const data = await response.json();
    const result = data.result;

    if (result && typeof result === 'string' && result.trim().length > 0) {
      searchResults.set(result);
    } else {
      searchResults.set('No results found.');
    }
    showResultsSection.set(true);

  } catch (error) {
    console.error('Error during search:', error);
    searchResults.set('Error during search. See console for details.');
    showResultsSection.set(true);
  }
}
</script>

<div class="flex space-x-2">
    <input type="text" id="search-input" placeholder="Enter search query" class="input" bind:value={searchQuery} on:keydown={(e) => e.key === 'Enter' && handleSearch()}>
    <button id="search-button" class="btn" on:click={handleSearch}>Search</button>
</div>