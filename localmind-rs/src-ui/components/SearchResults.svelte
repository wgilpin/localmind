<script>
let { results = [], query = '', loading = false, aiResponse = '', streaming = false, onDocumentClick } = $props();

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}
</script>

<div class="results">
    {#if loading}
        <div class="search-status">
            <div class="loading">🔍 Searching for "{escapeHtml(query)}"...</div>
        </div>
    {:else if results.length === 0 && query}
        <div class="search-status">
            <div class="no-results">No documents found for "{escapeHtml(query)}"</div>
        </div>
    {:else if results.length > 0}
        <div class="sources-section">
            <h3>📚 Found {results.length} relevant document(s):</h3>
            {#each results as source}
                <div
                    class="result-item clickable"
                    data-doc-id={source.doc_id}
                    onclick={() => onDocumentClick?.(source.doc_id)}
                >
                    <div class="result-title">{escapeHtml(source.title)}</div>
                    <div class="result-snippet">{escapeHtml(source.content_snippet)}</div>
                    <div class="result-meta">
                        <span class="similarity">Similarity: {(source.similarity * 100).toFixed(1)}%</span>
                        <span class="doc-id">ID: {source.doc_id}</span>
                    </div>
                </div>
            {/each}
        </div>

        {#if aiResponse || streaming}
            <div class="answer-section" class:generating={streaming}>
                <h3>🤖 AI Response:</h3>
                {#if streaming && !aiResponse}
                    <div class="loading">Generating response...</div>
                {:else}
                    <p id="streaming-response">{aiResponse}</p>
                {/if}
            </div>
        {/if}
    {/if}
</div>

<style>
.results {
    background: white;
    border-radius: 8px;
    min-height: 200px;
    padding: 20px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.result-item {
    border-bottom: 1px solid #eee;
    padding: 15px 0;
}

.result-item:last-child {
    border-bottom: none;
}

.result-title {
    font-weight: 600;
    margin-bottom: 8px;
}

.result-snippet {
    color: #666;
    line-height: 1.4;
}

.answer-section {
    background: #f8f9fa;
    padding: 15px;
    border-radius: 6px;
    margin-top: 20px;
    border-left: 4px solid #2563eb;
}

.answer-section.generating {
    border-left: 4px solid #f59e0b;
}

.search-status {
    text-align: center;
    padding: 20px;
}

.loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: #666;
    font-style: italic;
}

.loading::after {
    content: '';
    width: 16px;
    height: 16px;
    border: 2px solid #ddd;
    border-top: 2px solid #2563eb;
    border-radius: 50%;
    animation: spin 1s linear infinite;
}

@keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
}

.no-results {
    color: #666;
    font-style: italic;
}

.sources-section h3 {
    color: #374151;
    margin: 0 0 15px 0;
    font-size: 1.1rem;
}

.answer-section h3 {
    margin: 0 0 10px 0;
    color: #2563eb;
    font-size: 1.1rem;
}

.answer-section p {
    margin: 0;
    line-height: 1.5;
    white-space: pre-wrap;
}

.result-meta {
    margin-top: 8px;
    font-size: 0.85rem;
    color: #9ca3af;
}

.similarity {
    background: #e5e7eb;
    padding: 2px 6px;
    border-radius: 3px;
    margin-right: 8px;
}

.doc-id {
    background: #f3f4f6;
    padding: 2px 6px;
    border-radius: 3px;
}

.result-item.clickable {
    cursor: pointer;
    transition: background-color 0.2s ease, transform 0.1s ease, box-shadow 0.2s ease;
    border-radius: 8px;
    margin: 8px 0;
    padding: 15px;
    border: 1px solid #e5e7eb;
    background: #ffffff;
}

.result-item.clickable:hover {
    background-color: #f8fafc;
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    border-color: #2563eb;
}
</style>
