<script>
import { getTauriAPI } from '../tauri.svelte.js';

let { document = null, onBack } = $props();

const tauri = getTauriAPI();

function escapeHtml(text) {
    const div = window.document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

async function openInBrowser(url) {
    if (!tauri.shell) {
        console.error('Tauri shell API not available');
        return;
    }

    try {
        await tauri.shell.open(url);
    } catch (error) {
        console.error('Failed to open URL:', error);
    }
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
                    <button class="open-link" onclick={() => openInBrowser(document.url)}>
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
                            <polyline points="15 3 21 3 21 9"></polyline>
                            <line x1="10" y1="14" x2="21" y2="3"></line>
                        </svg>
                        Open Original Page
                    </button>
                </div>
            {/if}
            <div class="document-meta">
                <span class="source">Source: {escapeHtml(document.source)}</span>
            </div>
            <div class="document-content">
                {@html escapeHtml(document.content).replace(/\n/g, '<br>')}
            </div>
        </div>
    </div>
{/if}

<style>
.results {
    background: #1a1f2e;
    border-radius: 12px;
    border: 1px solid #2d3548;
    min-height: calc(100vh - 200px);
    padding: 0;
    overflow: hidden;
}

.document-view {
    padding: 24px;
    height: 100%;
    overflow-y: auto;
}

.document-view::-webkit-scrollbar {
    width: 8px;
}

.document-view::-webkit-scrollbar-track {
    background: #1a1f2e;
}

.document-view::-webkit-scrollbar-thumb {
    background: #374151;
    border-radius: 4px;
}

.document-view::-webkit-scrollbar-thumb:hover {
    background: #4b5563;
}

.document-header {
    display: flex;
    align-items: center;
    gap: 15px;
    margin-bottom: 20px;
    padding-bottom: 15px;
    border-bottom: 2px solid #2d3548;
}

.back-button {
    background: #374151;
    color: #e5e7eb;
    border: 1px solid #4b5563;
    padding: 8px 16px;
    border-radius: 8px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    transition: all 0.2s ease;
    flex-shrink: 0;
}

.back-button:hover {
    background: #4b5563;
    border-color: #6b7280;
}

.document-header h2 {
    margin: 0;
    color: #e5e7eb;
    font-size: 1.5rem;
    line-height: 1.2;
    font-weight: 600;
}

.document-actions {
    margin-bottom: 20px;
}

.open-link {
    background: #3b82f6;
    color: white;
    border: none;
    padding: 10px 18px;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    transition: all 0.2s ease;
}

.open-link:hover {
    background: #2563eb;
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(59, 130, 246, 0.3);
}

.open-link svg {
    flex-shrink: 0;
}

.document-meta {
    margin-bottom: 20px;
    padding: 12px 0;
    border-bottom: 1px solid #2d3548;
    font-size: 0.9rem;
    color: #9ca3af;
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
}

.document-meta .source,
.document-meta .doc-id {
    background: #0f1419;
    border: 1px solid #2d3548;
    padding: 6px 12px;
    border-radius: 6px;
}

.document-content {
    line-height: 1.8;
    color: #d1d5db;
    font-size: 1rem;
    max-width: none;
    word-wrap: break-word;
}
</style>
