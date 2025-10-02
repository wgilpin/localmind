<script>
let { document = null, onBack } = $props();

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}
</script>

{#if document}
    <div class="results">
        <div class="document-view">
            <div class="document-header">
                <button class="back-button" onclick={onBack}>← Back to Search</button>
                <h2>{escapeHtml(document.title)}</h2>
            </div>
            {#if document.url}
                <div class="document-actions">
                    <a href={document.url} target="_blank" class="open-link">
                        🔗 Open Original Page
                    </a>
                </div>
            {/if}
            <div class="document-meta">
                <span class="source">Source: {escapeHtml(document.source)}</span>
                <span class="doc-id">ID: {document.id}</span>
            </div>
            <div class="document-content">
                {@html escapeHtml(document.content).replace(/\n/g, '<br>')}
            </div>
        </div>
    </div>
{/if}

<style>
.results {
    background: white;
    border-radius: 8px;
    min-height: 200px;
    padding: 20px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.document-view {
    padding: 0;
}

.document-header {
    display: flex;
    align-items: center;
    gap: 15px;
    margin-bottom: 20px;
    padding-bottom: 15px;
    border-bottom: 2px solid #e5e7eb;
}

.back-button {
    background: #6b7280;
    color: white;
    border: none;
    padding: 8px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    transition: background-color 0.2s ease;
    flex-shrink: 0;
}

.back-button:hover {
    background: #374151;
}

.document-header h2 {
    margin: 0;
    color: #111827;
    font-size: 1.5rem;
    line-height: 1.2;
}

.document-actions {
    margin-bottom: 15px;
}

.open-link {
    background: #2563eb;
    color: white;
    text-decoration: none;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 14px;
    display: inline-block;
    transition: background-color 0.2s ease;
}

.open-link:hover {
    background: #1d4ed8;
}

.document-meta {
    margin-bottom: 20px;
    padding: 10px 0;
    border-bottom: 1px solid #e5e7eb;
    font-size: 0.9rem;
    color: #6b7280;
}

.document-meta .source,
.document-meta .doc-id {
    background: #f3f4f6;
    padding: 4px 8px;
    border-radius: 4px;
    margin-right: 10px;
}

.document-content {
    line-height: 1.6;
    color: #374151;
    font-size: 1rem;
    max-width: none;
    word-wrap: break-word;
}
</style>
