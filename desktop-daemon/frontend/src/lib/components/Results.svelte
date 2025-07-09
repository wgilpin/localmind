<script lang="ts">
  /**
   * Results component for displaying search results and progress.
   */
  import { searchResults, showResultsSection, searchStatus, searchProgress } from '../stores';
</script>

{#if $showResultsSection}
  <div id="results-container">
    {#if $searchStatus !== 'idle' && $searchStatus !== 'complete'}
      <div class="progress-indicator">
        <div class="progress-spinner"></div>
        <div class="progress-text">{$searchProgress}</div>
      </div>
    {/if}
    
    {#if $searchResults && ($searchStatus === 'complete' || $searchStatus === 'error')}
      <div class="search-result">{$searchResults}</div>
    {:else if $searchStatus === 'complete' && !$searchResults}
      <div class="no-results">No results found.</div>
    {/if}
  </div>
{/if}

<style>
  .progress-indicator {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    background: #f8f9fa;
    border-radius: 8px;
    margin-bottom: 16px;
  }

  .progress-spinner {
    width: 20px;
    height: 20px;
    border: 2px solid #e9ecef;
    border-top: 2px solid #007bff;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  .progress-text {
    color: #6c757d;
    font-size: 14px;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }
</style>