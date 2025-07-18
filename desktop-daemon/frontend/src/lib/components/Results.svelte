<script lang="ts">
  /**
   * Results component for displaying search results and progress.
   */
  import { searchResults, vectorResults, showResultsSection, searchStatus, searchProgress, retrievedDocuments, stopCurrentGeneration } from '../stores';
  import type { VectorSearchResult, SearchStatus, RetrievedDocument } from '../stores';
  import { marked } from 'marked';
  import Documents from './Documents.svelte';
  import EditModal from './EditModal.svelte';
  import { deleteNote, updateNote } from '../documentActions';

  let expandedResults: Set<string> = new Set();
  let documentContents: Map<string, string> = new Map();

  let showEditModal = false;
  let documentToEdit: RetrievedDocument | null = null;

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

  function handleDelete(event: CustomEvent<string>) {
    deleteNote(event.detail);
  }

  function handleEdit(event: CustomEvent<string>) {
    const docId = event.detail;
    const doc = ($retrievedDocuments as RetrievedDocument[]).find(d => d.id === docId);
    if (doc) {
      documentToEdit = doc;
      showEditModal = true;
    }
  }

  async function handleSaveEdit(event: CustomEvent<{ id: string; title: string; content: string }>) {
    const { id, title, content } = event.detail;
    await updateNote(id, { title, content });
    showEditModal = false;
    documentToEdit = null; // Clear the document to edit
  }

  $: showStopButton = $searchStatus === 'starting' || $searchStatus === 'embedding' || $searchStatus === 'searching' || $searchStatus === 'retrieving' || $searchStatus === 'generating';
</script>

{#if $showResultsSection}
  <div id="results-container">
    {#if $searchStatus !== 'idle' && $searchStatus !== 'complete' && $searchStatus !== 'stopped' && $searchStatus !== 'error'}
      <div class="progress-indicator">
        <div class="progress-spinner"></div>
        <div class="progress-text">{$searchProgress}</div>
        {#if showStopButton}
          <button class="stop-button" on:click={stopCurrentGeneration}>
            <div class="stop-icon"></div>
          </button>
        {/if}
      </div>
    {/if}
    
    {#if $retrievedDocuments && Array.isArray($retrievedDocuments) && $retrievedDocuments.length > 0}
      <Documents documents={$retrievedDocuments} on:delete={handleDelete} on:edit={handleEdit} />
    {/if}

    {#if $searchResults && ($searchStatus === 'generating' || $searchStatus === 'complete' || $searchStatus === 'error' || $searchStatus === 'stopped')}
      <div class="llm-result">
        <h3>AI Response</h3>
        <div class="search-result">{@html renderMarkdown($searchResults)}</div>
      </div>
    {:else if $searchStatus === 'complete' && !$searchResults && $vectorResults.length === 0}
      <div class="no-results">No results found.</div>
    {/if}
  </div>

  {#if showEditModal}
    <EditModal
      showModal={showEditModal}
      document={documentToEdit}
      on:save={handleSaveEdit}
      on:close={() => (showEditModal = false)}
    />
  {/if}
{/if}

<style>
  #results-container {
    position: relative; /* Needed for absolute positioning of the stop button */
  }

  .progress-indicator {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    background: #f8f9fa;
    border-radius: 8px;
    margin-bottom: 16px;
    position: relative; /* For positioning the stop button */
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
    flex-grow: 1; /* Allow text to take available space */
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  /* Removed unused CSS selectors */

  .llm-result {
    margin-top: 24px;
    position: relative; /* For positioning the stop button */
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

  .stop-button {
    background-color: #dc3545; /* Red color for stop */
    border: none;
    border-radius: 50%; /* Make it circular */
    width: 24px; /* Smaller size */
    height: 24px;
    display: flex;
    justify-content: center;
    align-items: center;
    cursor: pointer;
    box-shadow: 0 2px 4px rgba(0,0,0,0.2);
    transition: background-color 0.2s ease;
    margin-left: auto; /* Push to the right within flex container */
  }

  .stop-button:hover {
    background-color: #c82333;
  }

  .stop-icon {
    width: 8px; /* Smaller square */
    height: 8px;
    background-color: white; /* White square for the stop symbol */
  }
</style>