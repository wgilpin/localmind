<script>
let { response = '', streaming = false, sources = [], query = '' } = $props();

function copyToClipboard() {
    navigator.clipboard.writeText(response);
}

function extractCitations(text) {
    const citations = text.match(/\[(\d+)\]/g);
    return citations ? [...new Set(citations.map(c => parseInt(c.match(/\d+/)[0])))] : [];
}

let citations = $derived(extractCitations(response));
</script>

<div class="panel synthesis-panel">
    <div class="panel-header">
        <h2>AI Synthesis</h2>
        {#if response && !streaming}
            <button class="copy-btn" onclick={copyToClipboard} title="Copy answer">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                </svg>
            </button>
        {/if}
    </div>

    <div class="panel-content">
        {#if !query}
            <div class="empty-state">
                <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                    <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"></path>
                </svg>
                <p>AI-generated synthesis will appear here</p>
                <small>Select documents from the left panel to generate a response</small>
            </div>
        {:else if streaming && !response}
            <div class="generating-state">
                <div class="pulse-loader">
                    <div class="pulse-dot"></div>
                    <div class="pulse-dot"></div>
                    <div class="pulse-dot"></div>
                </div>
                <p>Generating response...</p>
            </div>
        {:else if response || streaming}
            <div class="answer-content">
                <div class="answer-text" class:generating={streaming}>
                    {response}
                </div>

                {#if sources.length > 0}
                    <div class="sources-section">
                        <h3>Sources Used</h3>
                        <div class="sources-list">
                            {#each sources as source, index}
                                <div class="source-item">
                                    <span class="source-number">{index + 1}</span>
                                    <div class="source-content">
                                        <div class="source-title">{source.title}</div>
                                        {#if source.url}
                                            <a href={source.url} class="source-url" target="_blank" rel="noopener noreferrer">
                                                {source.url}
                                            </a>
                                        {/if}
                                    </div>
                                </div>
                            {/each}
                        </div>
                    </div>
                {/if}
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

.synthesis-panel {
    border-color: #8b5cf6;
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

.copy-btn {
    background: #374151;
    border: none;
    border-radius: 6px;
    padding: 8px;
    color: #9ca3af;
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
}

.copy-btn:hover {
    background: #4b5563;
    color: #3b82f6;
}

.panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
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

.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    color: #6b7280;
    text-align: center;
    height: 100%;
}

.empty-state svg {
    margin-bottom: 16px;
    opacity: 0.5;
    color: #8b5cf6;
}

.empty-state p {
    font-size: 1rem;
    margin: 0 0 8px 0;
    color: #9ca3af;
}

.empty-state small {
    font-size: 0.875rem;
    color: #6b7280;
}

.generating-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    height: 100%;
}

.pulse-loader {
    display: flex;
    gap: 8px;
    margin-bottom: 20px;
}

.pulse-dot {
    width: 12px;
    height: 12px;
    background: #8b5cf6;
    border-radius: 50%;
    animation: pulse 1.4s ease-in-out infinite;
}

.pulse-dot:nth-child(2) {
    animation-delay: 0.2s;
}

.pulse-dot:nth-child(3) {
    animation-delay: 0.4s;
}

@keyframes pulse {
    0%, 80%, 100% {
        opacity: 0.3;
        transform: scale(0.8);
    }
    40% {
        opacity: 1;
        transform: scale(1);
    }
}

.generating-state p {
    color: #9ca3af;
    font-size: 1rem;
}

.answer-content {
    display: flex;
    flex-direction: column;
    gap: 24px;
}

.answer-text {
    color: #e5e7eb;
    line-height: 1.8;
    font-size: 1rem;
    white-space: pre-wrap;
    word-wrap: break-word;
}

.answer-text.generating::after {
    content: '▊';
    color: #8b5cf6;
    animation: blink 1s step-end infinite;
}

@keyframes blink {
    50% {
        opacity: 0;
    }
}

.sources-section {
    border-top: 1px solid #2d3548;
    padding-top: 20px;
}

.sources-section h3 {
    font-size: 1rem;
    font-weight: 600;
    color: #9ca3af;
    margin: 0 0 16px 0;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-size: 0.875rem;
}

.sources-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.source-item {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    padding: 12px;
    background: #0f1419;
    border: 1px solid #2d3548;
    border-radius: 8px;
    transition: all 0.2s ease;
}

.source-item:hover {
    border-color: #8b5cf6;
}

.source-number {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    background: linear-gradient(135deg, #8b5cf6, #6366f1);
    color: white;
    border-radius: 50%;
    font-size: 0.75rem;
    font-weight: 600;
    flex-shrink: 0;
}

.source-content {
    flex: 1;
    min-width: 0;
}

.source-title {
    font-weight: 500;
    color: #e5e7eb;
    margin-bottom: 4px;
    line-height: 1.4;
}

.source-url {
    display: block;
    font-size: 0.875rem;
    color: #6366f1;
    text-decoration: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.source-url:hover {
    text-decoration: underline;
    color: #8b5cf6;
}
</style>
