<script lang="ts">
  /**
   * Results component for displaying search results and progress.
   */
  import { searchResults, showResultsSection, searchStatus, searchProgress } from '../stores';
  import { marked } from 'marked';

  /**
   * Converts markdown text to HTML using marked library.
   * @param text The markdown text to convert
   * @returns HTML string
   */
  const renderMarkdown = (text: string): string => {
    return marked.parse(text) as string;
  };
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
      <div class="search-result">{@html renderMarkdown($searchResults)}</div>
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

  .search-result :global(h1),
  .search-result :global(h2),
  .search-result :global(h3),
  .search-result :global(h4),
  .search-result :global(h5),
  .search-result :global(h6) {
    margin: 1em 0 0.5em 0;
    color: #333;
  }

  .search-result :global(h1) { font-size: 1.5em; }
  .search-result :global(h2) { font-size: 1.3em; }
  .search-result :global(h3) { font-size: 1.1em; }

  .search-result :global(p) {
    margin: 0.8em 0;
    line-height: 1.6;
  }

  .search-result :global(ul),
  .search-result :global(ol) {
    margin: 0.8em 0;
    padding-left: 2em;
  }

  .search-result :global(li) {
    margin: 0.3em 0;
  }

  .search-result :global(code) {
    background: #f4f4f4;
    padding: 0.2em 0.4em;
    border-radius: 3px;
    font-family: 'Courier New', monospace;
    font-size: 0.9em;
  }

  .search-result :global(pre) {
    background: #f4f4f4;
    padding: 1em;
    border-radius: 5px;
    overflow-x: auto;
    margin: 1em 0;
  }

  .search-result :global(pre code) {
    background: none;
    padding: 0;
  }

  .search-result :global(blockquote) {
    border-left: 4px solid #ddd;
    margin: 1em 0;
    padding-left: 1em;
    color: #666;
  }

  .search-result :global(strong) {
    font-weight: bold;
  }

  .search-result :global(em) {
    font-style: italic;
  }
</style>