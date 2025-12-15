<script>
let { results = [], query = '', loading = false, onDocumentClick, onLoadMore, hasMore = false } = $props();

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function getSimilarityColor(similarity) {
    if (similarity >= 0.7) return '#8b5cf6';
    if (similarity >= 0.5) return '#3b82f6';
    return '#6366f1';
}
</script>

<div class="panel retrieval-panel">
    <div class="panel-header">
        <h2>Search Results</h2>
        <div class="header-actions">
            {#if results.length > 0}
                <div class="result-count">{results.length} documents</div>
            {/if}
        </div>
    </div>

    <div class="panel-content">
        {#if loading}
            <div class="search-status">
                <div class="loading">
                    <div class="spinner"></div>
                    <span>Searching for "{escapeHtml(query)}"...</span>
                </div>
            </div>
        {:else if results.length === 0 && query}
            <div class="search-status">
                <div class="no-results">No documents found for "{escapeHtml(query)}"</div>
            </div>
        {:else if results.length > 0}
            <div class="results-list">
                {#each results as source}
                    <div class="result-card" onclick={() => onDocumentClick?.(source.doc_id)}>
                        <div class="card-header">
                            <div class="result-title">{escapeHtml(source.title)}</div>
                            <div class="similarity-badge" style="--sim-color: {getSimilarityColor(source.similarity)}">
                                <div class="similarity-bar" style="width: {source.similarity * 100}%"></div>
                                <span class="similarity-text">{(source.similarity * 100).toFixed(1)}%</span>
                            </div>
                        </div>
                        <div class="result-snippet">{escapeHtml(source.content_snippet)}</div>
                    </div>
                {/each}
            </div>

            <div class="action-bar">
                <button class="more-btn" onclick={onLoadMore} disabled={!hasMore}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="6 9 12 15 18 9"></polyline>
                    </svg>
                    {hasMore ? 'More...' : 'No more results'}
                </button>
            </div>
        {:else}
            <div class="empty-state">
                <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                    <circle cx="11" cy="11" r="8"></circle>
                    <path d="m21 21-4.35-4.35"></path>
                </svg>
                <p>Search your knowledge base to get started</p>
            </div>
        {/if}
    </div>
</div>

<style>
.panel {
    background: #1a1f2e;
    border-radius: 12px;
    border: 1px solid #2d3548;
    display: flex;
    flex-direction: column;
    height: calc(100vh - 200px);
    overflow: hidden;
}

.panel-header {
    padding: 20px;
    border-bottom: 1px solid #2d3548;
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.panel-header h2 {
    font-size: 1.25rem;
    font-weight: 600;
    color: #e5e7eb;
    margin: 0;
}

.header-actions {
    display: flex;
    align-items: center;
    gap: 12px;
}

.result-count {
    background: #374151;
    padding: 4px 12px;
    border-radius: 12px;
    font-size: 0.875rem;
    color: #9ca3af;
}

.panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
}

.panel-content::-webkit-scrollbar {
    width: 8px;
}

.panel-content::-webkit-scrollbar-track {
    background: #1a1f2e;
}

.panel-content::-webkit-scrollbar-thumb {
    background: #374151;
    border-radius: 4px;
}

.panel-content::-webkit-scrollbar-thumb:hover {
    background: #4b5563;
}

.results-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.result-card {
    background: #0f1419;
    border: 1px solid #2d3548;
    border-radius: 8px;
    padding: 16px;
    transition: all 0.2s ease;
    position: relative;
}

.result-card:hover {
    border-color: #3b82f6;
    box-shadow: 0 4px 12px rgba(59, 130, 246, 0.15);
}


.card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 12px;
}

.result-title {
    font-weight: 600;
    font-size: 1rem;
    color: #e5e7eb;
    line-height: 1.4;
    margin-bottom: 0;
    flex: 1;
}

.result-card:hover .result-title {
    color: #3b82f6;
}

.result-card {
    cursor: pointer;
}

.result-snippet {
    color: #9ca3af;
    line-height: 1.6;
    font-size: 0.9rem;
}

.similarity-badge {
    position: relative;
    background: #374151;
    padding: 4px 10px;
    border-radius: 12px;
    display: flex;
    align-items: center;
    overflow: hidden;
}

.similarity-bar {
    position: absolute;
    left: 0;
    top: 0;
    height: 100%;
    background: var(--sim-color);
    opacity: 0.2;
    transition: width 0.3s ease;
}

.similarity-text {
    position: relative;
    color: var(--sim-color);
    font-weight: 600;
    font-size: 0.75rem;
}


.search-status {
    text-align: center;
    padding: 40px 20px;
}

.loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    color: #9ca3af;
}

.spinner {
    width: 40px;
    height: 40px;
    border: 3px solid #374151;
    border-top: 3px solid #3b82f6;
    border-radius: 50%;
    animation: spin 1s linear infinite;
}

@keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
}

.no-results {
    color: #9ca3af;
    font-style: italic;
}

.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    color: #6b7280;
    text-align: center;
}

.empty-state svg {
    margin-bottom: 16px;
    opacity: 0.5;
}

.empty-state p {
    font-size: 1rem;
    margin: 0;
}

.action-bar {
    padding: 16px 0 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.more-btn {
    width: 100%;
    padding: 10px 16px;
    background: #374151;
    color: #9ca3af;
    border: 1px solid #4b5563;
    border-radius: 8px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    transition: all 0.2s ease;
}

.more-btn:hover {
    background: #4b5563;
    color: #e5e7eb;
    border-color: #6b7280;
}

.more-btn svg {
    transition: transform 0.2s ease;
}

.more-btn:hover svg {
    transform: translateY(2px);
}

.more-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.more-btn:disabled:hover {
    background: #374151;
    color: #9ca3af;
    border-color: #4b5563;
}

.more-btn:disabled:hover svg {
    transform: none;
}
</style>
