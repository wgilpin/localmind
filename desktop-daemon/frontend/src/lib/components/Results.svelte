<script lang="ts">
  /**
   * Results component for displaying search results and progress.
   */
  import { searchResults, vectorResults, showResultsSection, searchStatus, searchProgress } from '../stores';
  import type { VectorSearchResult } from '../stores';
  import { marked } from 'marked';

  let expandedResults: Set<string> = new Set();
  let documentContents: Map<string, string> = new Map();

  /**
   * Converts markdown text to HTML using marked library.
   * @param text The markdown text to convert
   * @returns HTML string
   */
  const renderMarkdown = (text: string): string => {
    return marked.parse(text) as string;
  };

  /**
   * Handles clicking on a vector search result.
   * @param result The vector search result that was clicked
   */
  const handleResultClick = async (result: VectorSearchResult) => {
    if (result.url && isValidUrl(result.url)) {
      window.open(result.url, '_blank');
      return;
    }

    // Toggle expansion for non-URL results
    if (expandedResults.has(result.id)) {
      expandedResults.delete(result.id);
      expandedResults = new Set(expandedResults);
    } else {
      expandedResults.add(result.id);
      expandedResults = new Set(expandedResults);
      
      // Fetch document content if not already loaded
      if (!documentContents.has(result.id)) {
        try {
          const response = await fetch(`/documents/${result.id}`);
          if (response.ok) {
            const document = await response.json();
            documentContents.set(result.id, document.content);
            documentContents = new Map(documentContents);
          }
        } catch (error) {
          console.error('Error fetching document content:', error);
        }
      }
    }
  };

  /**
   * Checks if a string is a valid HTTP/HTTPS URL.
   * @param string The string to check
   * @returns True if the string is a valid HTTP/HTTPS URL
   */
  const isValidUrl = (string: string): boolean => {
    try {
      const url = new URL(string);
      return url.protocol === 'http:' || url.protocol === 'https:';
    } catch (_) {
      return false;
    }
  };

  /**
   * Formats a timestamp into a readable date string.
   * @param timestamp The timestamp to format
   * @returns Formatted date string
   */
  const formatDate = (timestamp: number): string => {
    return new Date(timestamp).toLocaleDateString();
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
    
    {#if $vectorResults.length > 0}
      <div class="vector-results">
        <h3>Related Documents</h3>
        <div class="vector-results-list">
          {#each $vectorResults as result}
            <div
              class="vector-result-item"
              class:clickable={true}
              class:expanded={expandedResults.has(result.id)}
              on:click={() => handleResultClick(result)}
              on:keydown={(e) => e.key === 'Enter' && handleResultClick(result)}
              tabindex="0"
              role="button"
            >
              <div class="result-title">
                {result.title}
                {#if result.url && isValidUrl(result.url)}
                  <span class="external-link-icon">🔗</span>
                {:else}
                  <span class="expand-icon" class:expanded={expandedResults.has(result.id)}>
                    {expandedResults.has(result.id) ? '▼' : '▶'}
                  </span>
                {/if}
              </div>
              <div class="result-meta">
                {formatDate(result.timestamp)}
              </div>
              {#if expandedResults.has(result.id) && documentContents.has(result.id)}
                <div class="result-content">
                  {@html renderMarkdown(documentContents.get(result.id) || '')}
                </div>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}
    
    {#if $searchResults && ($searchStatus === 'complete' || $searchStatus === 'error')}
      <div class="llm-result">
        <h3>AI Response</h3>
        <div class="search-result">{@html renderMarkdown($searchResults)}</div>
      </div>
    {:else if $searchStatus === 'complete' && !$searchResults && $vectorResults.length === 0}
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

  .vector-results {
    margin-bottom: 24px;
  }

  .vector-results h3 {
    margin: 0 0 16px 0;
    color: #333;
    font-size: 1.2em;
  }

  .vector-results-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .vector-result-item {
    padding: 12px 16px;
    border: 1px solid #e0d4b6;
    border-radius: 8px;
    background: #f5deb3;
    color: #4A4A4A;
    transition: all 0.2s ease;
  }

  .vector-result-item.clickable {
    cursor: pointer;
  }

  .vector-result-item.clickable:hover {
    background: #faf0e6;
    border-color: #d2b48c;
    transform: translateY(-1px);
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  }

  .vector-result-item:focus {
    outline: 2px solid #d2b48c;
    outline-offset: 2px;
  }

  .result-title {
    font-weight: 600;
    color: #333;
    margin-bottom: 4px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .external-link-icon {
    font-size: 0.9em;
    opacity: 0.7;
  }

  .result-meta {
    font-size: 0.85em;
    color: #555555;
  }

  .expand-icon {
    font-size: 0.8em;
    opacity: 0.7;
    transition: transform 0.2s ease;
  }

  .expand-icon.expanded {
    transform: rotate(0deg);
  }

  .result-content {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid #e9ecef;
    font-size: 0.9em;
    line-height: 1.5;
  }

  .vector-result-item.expanded {
    border-color: #d2b48c;
    background: #faf0e6;
  }

  .llm-result {
    margin-top: 24px;
  }

  .llm-result h3 {
    margin: 0 0 16px 0;
    color: #333;
    font-size: 1.2em;
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

  .no-results {
    text-align: center;
    padding: 40px;
    color: #6c757d;
  }
</style>