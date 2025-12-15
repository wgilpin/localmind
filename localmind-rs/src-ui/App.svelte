<script>
import { onMount } from 'svelte';
import { initializeTauriAPI, getTauriAPI } from './tauri.svelte.js';
import SearchBar from './components/SearchBar.svelte';
import SearchResults from './components/SearchResults.svelte';
import DocumentView from './components/DocumentView.svelte';
import SettingsModal from './components/SettingsModal.svelte';
import Toast from './components/Toast.svelte';

const tauri = getTauriAPI();

let similarityCutoff = $state(0.3);
let searchResults = $state([]);
let allResults = $state([]);
let lastQuery = $state('');
let loading = $state(false);
let currentDocument = $state(null);
let showSettings = $state(false);
let toasts = $state([]);
let currentProgressToast = null;

onMount(async () => {
    try {
        await initializeTauriAPI();
        await loadStats();
        setupBookmarkProgressListener();
    } catch (error) {
        console.error('Failed to initialize Tauri:', error);
        showToast('Failed to initialize application: ' + error, 'error');
    }
});

async function loadStats() {
    if (!tauri.invoke) return;

    try {
        const stats = await tauri.invoke('get_stats');
        console.log('System stats:', stats);

        if (stats.status === 'initializing') {
            showToast('LocalMind is initializing...', 'info');
        } else if (stats.document_count === 0) {
            showToast('No documents found. Add some documents to start searching!', 'info');
        }
    } catch (error) {
        console.error('Failed to load stats:', error);
        showToast('Failed to connect to backend: ' + error, 'error');
    }
}

function setupBookmarkProgressListener() {
    if (!tauri.listen) {
        console.warn('Tauri event listener not available');
        return;
    }

    console.log('Setting up bookmark progress listener');

    tauri.listen('bookmark-progress', (event) => {
        console.log('Bookmark progress event received:', event);
        const progress = event.payload;

        if (!progress) {
            console.error('No payload in bookmark progress event');
            return;
        }

        if (progress.completed) {
            console.log('Bookmark processing completed!');
            if (currentProgressToast) {
                toasts = toasts.filter(t => t !== currentProgressToast);
                currentProgressToast = null;
            }
            showToast(progress.current_title, 'success', 5000);
            setTimeout(loadStats, 1000);
        } else {
            const percentage = Math.round((progress.current / progress.total) * 100);
            const message = `Processing bookmarks... ${progress.current}/${progress.total} (${percentage}%)\nCurrent: ${progress.current_title}`;
            console.log('Showing progress toast:', message);
            showToast(message, 'info', 0);
        }
    }).then(() => {
        console.log('Successfully registered bookmark-progress listener');
    }).catch((error) => {
        console.error('Failed to register bookmark-progress listener:', error);
    });
}

function calculateOptimalThreshold(results) {
    if (!results || results.length === 0) return 0.3;

    const sortedResults = [...results].sort((a, b) => b.similarity - a.similarity);
    let targetCount = Math.min(8, Math.max(5, Math.floor(sortedResults.length * 0.1)));

    if (sortedResults.length <= targetCount) {
        return Math.max(0.3, sortedResults[sortedResults.length - 1].similarity);
    }

    let threshold = sortedResults[targetCount - 1].similarity;
    threshold = Math.max(0.3, threshold);
    threshold = Math.floor(threshold * 100) / 100;

    console.log(`Calculated optimal threshold: ${threshold}`);
    return threshold;
}

async function handleSearch(query, cutoff) {
    if (!tauri.invoke) {
        showToast('Tauri API not available', 'error');
        return;
    }

    lastQuery = query;
    loading = true;

    try {
        console.log('Searching for:', query, 'with cutoff:', cutoff);

        const searchHits = await tauri.invoke('search_hits', { query, cutoff: 0.0 });
        console.log('Search hits:', searchHits);

        if (searchHits.sources) {
            allResults = searchHits.sources;
            console.log('Cached', allResults.length, 'results');

            const autoThreshold = calculateOptimalThreshold(allResults);
            if (autoThreshold !== cutoff) {
                console.log('Auto-setting threshold to', autoThreshold);
                similarityCutoff = autoThreshold;
            }

            const filteredSources = allResults.filter(s => s.similarity >= autoThreshold);
            searchResults = filteredSources;
        }

        loading = false;
    } catch (error) {
        console.error('Search error:', error);
        showToast('Search failed: ' + error, 'error');
        loading = false;
    }
}

function handleLoadMore() {
    const newCutoff = Math.max(0.0, similarityCutoff - 0.1);
    console.log('Loading more results, lowering threshold to:', newCutoff);
    similarityCutoff = newCutoff;

    if (lastQuery && allResults.length > 0) {
        const filteredResults = allResults.filter(s => s.similarity >= newCutoff);
        searchResults = filteredResults;
    }
}

async function handleDocumentClick(docId) {
    if (!tauri.invoke) {
        showToast('Tauri API not available', 'error');
        return;
    }

    try {
        console.log('Fetching document with id:', docId);
        const doc = await tauri.invoke('get_document', { id: docId });
        console.log('Document fetched:', doc);
        currentDocument = doc;
    } catch (error) {
        console.error('Failed to fetch document:', error);
        showToast('Failed to load document: ' + error, 'error');
    }
}

function handleBack() {
    currentDocument = null;
}

function handleSettingsSave(message) {
    showToast(message, 'success');
}

function showToast(message, type = 'info', duration = 5000) {
    console.log(`Toast: [${type}] ${message}`);

    if (type === 'info' && message.includes('Processing bookmarks') && currentProgressToast) {
        toasts = toasts.filter(t => t !== currentProgressToast);
        currentProgressToast = null;
    }

    const toast = { id: Date.now(), message, type, duration };

    if (type === 'info' && message.includes('Processing bookmarks')) {
        currentProgressToast = toast;
    }

    toasts = [...toasts, toast];
}

function removeToast(toastId) {
    toasts = toasts.filter(t => t.id !== toastId);
    if (currentProgressToast?.id === toastId) {
        currentProgressToast = null;
    }
}

function handleKeydown(e) {
    if (e.key === 'Escape' && currentDocument) {
        handleBack();
    }
}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="app-container">
    <SearchBar
        onSearch={handleSearch}
        onSettingsClick={() => showSettings = true}
    />

    {#if currentDocument}
        <DocumentView document={currentDocument} onBack={handleBack} />
    {:else if lastQuery}
        <div class="search-results-container">
            <SearchResults
                results={searchResults}
                query={lastQuery}
                loading={loading}
                onDocumentClick={handleDocumentClick}
                onLoadMore={handleLoadMore}
                hasMore={similarityCutoff > 0.0 && allResults.length > searchResults.length}
            />
        </div>
    {/if}
</div>

<div class="toast-container">
    {#each toasts as toast (toast.id)}
        <Toast
            message={toast.message}
            type={toast.type}
            duration={toast.duration}
            onClose={() => removeToast(toast.id)}
        />
    {/each}
</div>

<SettingsModal
    show={showSettings}
    onClose={() => showSettings = false}
    onSave={handleSettingsSave}
/>

<style>
:global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

:global(body) {
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background-color: #0f1419;
    color: #e5e7eb;
    line-height: 1.6;
}

.app-container {
    max-width: 1800px;
    margin: 0 auto;
    padding: 20px;
    min-height: 100vh;
}

.search-results-container {
    margin-top: 20px;
}

.toast-container {
    position: fixed;
    bottom: 20px;
    right: 20px;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-width: 400px;
}
</style>
