<script lang="ts">
  /**
   * Results component for displaying search results.
   */
  import { searchResults, showResultsSection } from '../stores';
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