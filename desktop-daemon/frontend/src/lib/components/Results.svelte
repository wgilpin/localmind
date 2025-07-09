<script lang="ts">
  import { searchResults, showResultsSection } from './Search.svelte';
</script>

{#if $showResultsSection}
  <div id="results-container">
    {#if $searchResults}
      <div class="search-result">{$searchResults}</div>
    {:else}
      <div class="no-results">No results found.</div>
    {/if}
  </div>
{/if}