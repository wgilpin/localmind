// Import Tauri API - handle different Tauri versions
let invoke;
let listen;

function initializeTauriAPI(callback) {
    console.log('Initializing Tauri API...');

    // Wait a bit for the API to be injected
    setTimeout(() => {
        if (window.__TAURI__) {
            console.log('Found window.__TAURI__');
            console.log('__TAURI__ keys:', Object.keys(window.__TAURI__));

            // According to docs, when withGlobalTauri is true, invoke is at window.__TAURI__.core.invoke
            if (window.__TAURI__.core && window.__TAURI__.core.invoke) {
                console.log('Found invoke at window.__TAURI__.core.invoke');
                invoke = window.__TAURI__.core.invoke;
            } else if (window.__TAURI__.invoke) {
                console.log('Found invoke at window.__TAURI__.invoke');
                invoke = window.__TAURI__.invoke;
            } else {
                // Debug what's available
                for (const key of Object.keys(window.__TAURI__)) {
                    console.log(`__TAURI__.${key}:`, typeof window.__TAURI__[key]);
                    if (window.__TAURI__[key] && typeof window.__TAURI__[key] === 'object') {
                        console.log(`__TAURI__.${key} keys:`, Object.keys(window.__TAURI__[key]));
                    }
                }
            }

            // Get the event listener function
            if (window.__TAURI__.event && window.__TAURI__.event.listen) {
                console.log('Found event.listen at window.__TAURI__.event.listen');
                listen = window.__TAURI__.event.listen;
            } else {
                console.warn('Tauri event.listen not found');
            }

            if (invoke) {
                console.log('Successfully initialized Tauri API');
                // Call the callback now that we have invoke
                if (callback) callback();
            } else {
                console.error('Tauri invoke function not found');
                showMessage('Tauri API not properly initialized', 'error');
            }
        } else {
            console.error('window.__TAURI__ not available after timeout');
            showMessage('Tauri API not available', 'error');
        }
    }, 100); // Small delay to ensure API is injected

    return true; // Return true to continue initialization
}

document.addEventListener('DOMContentLoaded', function() {
    const searchInput = document.getElementById('search-input');
    const searchBtn = document.getElementById('search-btn');
    const resultsDiv = document.getElementById('results');

    // Initialize Tauri API (now async with timeout)
    initializeTauriAPI(() => {
        loadStats();
        setupBookmarkProgressListener();
    });

    function setupBookmarkProgressListener() {
        if (!listen) {
            console.warn('Tauri event listener not available');
            return;
        }

        console.log('=== Setting up bookmark progress listener ===')
        console.log('listen function available:', !!listen)
        console.log('listen function type:', typeof listen);

        // Listen for bookmark progress events
        listen('bookmark-progress', (event) => {
            console.log('🎯 === BOOKMARK PROGRESS EVENT RECEIVED ===');
            console.log('📄 Raw event:', event);
            console.log('📦 Event payload:', event.payload);
            const progress = event.payload;

            if (!progress) {
                console.error('❌ No payload in bookmark progress event');
                return;
            }

            console.log('📊 Progress data:', {
                current: progress.current,
                total: progress.total,
                title: progress.current_title,
                completed: progress.completed
            });

            if (progress.completed) {
                console.log('✅ Bookmark processing completed!');
                showToast(progress.current_title, 'success', 3000);
                // Refresh stats after completion
                setTimeout(loadStats, 1000);
            } else {
                const percentage = Math.round((progress.current / progress.total) * 100);
                const message = `📚 Processing bookmarks... ${progress.current}/${progress.total} (${percentage}%)\nCurrent: ${progress.current_title}`;
                console.log('📋 Showing progress toast:', message);
                showToast(
                    message,
                    'info',
                    0 // Keep showing until replaced or completed
                );
            }
        }).then(() => {
            console.log('Successfully registered bookmark-progress listener');
        }).catch((error) => {
            console.error('Failed to register bookmark-progress listener:', error);
        });

        console.log('Bookmark progress listener setup complete');
    }

    async function loadStats() {
        if (!invoke) {
            showMessage('Tauri API not available', 'error');
            return;
        }

        try {
            const stats = await invoke('get_stats');
            console.log('System stats:', stats);

            if (stats.status === 'initializing') {
                showMessage('LocalMind is initializing...', 'info');
            } else if (stats.document_count === 0) {
                showMessage('No documents found. Add some documents to start searching!', 'info');
            }
        } catch (error) {
            console.error('Failed to load stats:', error);
            showMessage('Failed to connect to backend: ' + error, 'error');
        }
    }

    async function performSearch() {
        if (!invoke) {
            showMessage('Tauri API not available', 'error');
            return;
        }

        const query = searchInput.value.trim();
        if (!query) {
            showMessage('Please enter a search query.', 'warning');
            return;
        }

        try {
            console.log('Searching for:', query);

            // Step 1: Get search hits immediately
            showLoadingState(query);
            const searchHits = await invoke('search_hits', { query });
            console.log('Search hits:', searchHits);

            // Display search hits immediately
            displaySearchHits(searchHits);

            if (searchHits.has_results) {
                // Step 2: Generate AI response in background
                showGeneratingState();
                const documentIds = searchHits.sources.map(s => s.doc_id);
                const aiResponse = await invoke('generate_response', {
                    query,
                    contextSources: documentIds
                });
                console.log('AI response:', aiResponse);

                // Add AI response to existing search hits
                displayAIResponse(aiResponse);
            }

        } catch (error) {
            console.error('Search error:', error);
            showMessage('Search failed: ' + error, 'error');
        }
    }

    function showLoadingState(query) {
        resultsDiv.innerHTML = `
            <div class="search-status">
                <div class="loading">🔍 Searching for "${escapeHtml(query)}"...</div>
            </div>
        `;
    }

    function displaySearchHits(searchHits) {
        if (!searchHits.has_results) {
            resultsDiv.innerHTML = `
                <div class="search-status">
                    <div class="no-results">No documents found for "${escapeHtml(searchHits.query)}"</div>
                </div>
            `;
            return;
        }

        let sourcesHtml = searchHits.sources.map(source => `
            <div class="result-item">
                <div class="result-title">${escapeHtml(source.title)}</div>
                <div class="result-snippet">${escapeHtml(source.content_snippet)}</div>
                <div class="result-meta">
                    <span class="similarity">Similarity: ${(source.similarity * 100).toFixed(1)}%</span>
                    <span class="doc-id">ID: ${source.doc_id}</span>
                </div>
            </div>
        `).join('');

        resultsDiv.innerHTML = `
            <div class="sources-section">
                <h3>📚 Found ${searchHits.sources.length} relevant document(s):</h3>
                ${sourcesHtml}
            </div>
            <div id="ai-response-section"></div>
        `;
    }

    function showGeneratingState() {
        const aiSection = document.getElementById('ai-response-section');
        if (aiSection) {
            aiSection.innerHTML = `
                <div class="answer-section generating">
                    <h3>🤖 AI Response:</h3>
                    <div class="loading">Generating response...</div>
                </div>
            `;
        }
    }

    function displayAIResponse(aiResponse) {
        const aiSection = document.getElementById('ai-response-section');
        if (aiSection) {
            aiSection.innerHTML = `
                <div class="answer-section">
                    <h3>🤖 AI Response:</h3>
                    <p>${escapeHtml(aiResponse)}</p>
                </div>
            `;
        }
    }

    function displayResults(response) {
        if (!response.sources || response.sources.length === 0) {
            resultsDiv.innerHTML = `
                <div class="answer-section">
                    <h3>Answer:</h3>
                    <p>${response.answer}</p>
                </div>
                <div class="no-sources">
                    <p><em>No source documents found.</em></p>
                </div>
            `;
            return;
        }

        let sourcesHtml = response.sources.map(source => `
            <div class="result-item">
                <div class="result-title">${escapeHtml(source.title)}</div>
                <div class="result-snippet">${escapeHtml(source.content_snippet)}</div>
                <div class="result-meta">
                    <span class="similarity">Similarity: ${(source.similarity * 100).toFixed(1)}%</span>
                    <span class="doc-id">ID: ${source.doc_id}</span>
                </div>
            </div>
        `).join('');

        resultsDiv.innerHTML = `
            <div class="answer-section">
                <h3>Answer:</h3>
                <p>${escapeHtml(response.answer)}</p>
            </div>
            <div class="sources-section">
                <h3>Sources:</h3>
                ${sourcesHtml}
            </div>
        `;
    }

    function showMessage(message, type) {
        const className = type === 'error' ? 'error-message' :
                         type === 'warning' ? 'warning-message' :
                         'info-message';

        resultsDiv.innerHTML = `<div class="${className}">${escapeHtml(message)}</div>`;
    }

    let currentProgressToast = null;

    function showToast(message, type = 'info', duration = 5000) {
        console.log(`🍞 SHOWTOAST CALLED: [${type}] ${message} (duration: ${duration})`);

        const toastContainer = document.getElementById('toast-container');
        if (!toastContainer) {
            console.error('❌ Toast container not found!');
            return;
        }

        // Remove existing progress toast if this is a new progress message
        if (type === 'info' && message.includes('Processing bookmarks') && currentProgressToast) {
            currentProgressToast.remove();
            currentProgressToast = null;
        }

        const toast = document.createElement('div');
        toast.className = `toast ${type}`;
        toast.innerHTML = `
            ${escapeHtml(message).replace(/\n/g, '<br>')}
            <button class="close-btn" onclick="this.parentElement.remove()">×</button>
        `;

        toastContainer.appendChild(toast);

        // Keep reference to progress toasts
        if (type === 'info' && message.includes('Processing bookmarks')) {
            currentProgressToast = toast;
        }

        // Trigger animation
        setTimeout(() => {
            toast.classList.add('show');
        }, 10);

        // Auto remove after duration (unless duration is 0)
        if (duration > 0) {
            setTimeout(() => {
                toast.classList.remove('show');
                setTimeout(() => {
                    if (toast.parentElement) {
                        toast.remove();
                        if (toast === currentProgressToast) {
                            currentProgressToast = null;
                        }
                    }
                }, 300);
            }, duration);
        }
    }

    // Export showToast to global scope for testing
    window.showToast = showToast;

    function escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    // Event listeners
    searchBtn.addEventListener('click', performSearch);
    searchInput.addEventListener('keypress', function(e) {
        if (e.key === 'Enter') {
            performSearch();
        }
    });
});