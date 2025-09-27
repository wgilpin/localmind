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
    const similarityCutoff = document.getElementById('similarity-cutoff');
    const similarityValue = document.getElementById('similarity-value');

    // Global state to track last search query and all results
    let lastSearchQuery = '';
    let allSearchResults = []; // Cache ALL results from backend

    // Initialize Tauri API (now async with timeout)
    initializeTauriAPI(() => {
        loadStats();
        setupBookmarkProgressListener();
    });

    // Handle similarity cutoff changes
    similarityCutoff.addEventListener('input', function() {
        const value = parseFloat(this.value);
        similarityValue.textContent = value.toFixed(1);

        // If we have cached results, just filter them client-side
        if (lastSearchQuery && allSearchResults.length > 0) {
            console.log('Similarity cutoff changed to', value, '- filtering cached results');
            filterAndDisplayResults(value);
        }
    });

    function calculateOptimalThreshold(results) {
        if (!results || results.length === 0) return 0.3; // Minimum 30%

        // Sort results by similarity (highest first)
        const sortedResults = [...results].sort((a, b) => b.similarity - a.similarity);

        // Find threshold that gives us 5-10 results
        let targetCount = Math.min(8, Math.max(5, Math.floor(sortedResults.length * 0.1)));

        if (sortedResults.length <= targetCount) {
            // If we have fewer results than target, use a low threshold
            return Math.max(0.3, sortedResults[sortedResults.length - 1].similarity);
        }

        // Get the threshold at the target position
        let threshold = sortedResults[targetCount - 1].similarity;

        // Ensure we don't go below 30%
        threshold = Math.max(0.3, threshold);

        // Round down to 2 decimal places for more inclusive results
        threshold = Math.floor(threshold * 100) / 100;

        console.log(`Calculated optimal threshold: ${threshold} (targets ${targetCount} results from ${sortedResults.length} total)`);
        return threshold;
    }

    function filterAndDisplayResults(cutoff) {
        console.log('Filtering', allSearchResults.length, 'cached results with cutoff:', cutoff);

        // Filter the cached results based on the similarity cutoff
        const filteredResults = allSearchResults.filter(source => source.similarity >= cutoff);
        console.log('Found', filteredResults.length, 'results above cutoff');

        // Display the filtered results without making a backend call
        if (filteredResults.length === 0) {
            resultsDiv.innerHTML = `
                <div class="search-status">
                    <div class="no-results">No documents found above ${(cutoff * 100).toFixed(0)}% similarity for "${escapeHtml(lastSearchQuery)}"</div>
                    <div class="result-meta" style="margin-top: 10px; opacity: 0.7;">
                        ${allSearchResults.length} total results cached. Try lowering the similarity threshold.
                    </div>
                </div>
            `;
        } else {
            let sourcesHtml = filteredResults.map(source => `
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
                    <h3>📚 Found ${filteredResults.length} relevant document(s) (${allSearchResults.length} total cached):</h3>
                    ${sourcesHtml}
                </div>
                <div id="ai-response-section"></div>
            `;

            // If we had an AI response before, regenerate it with the new filtered results
            if (filteredResults.length > 0 && filteredResults.length <= 5) {
                // Cancel any ongoing generation first
                invoke('cancel_generation').catch(error => {
                    console.warn('Failed to cancel previous generation:', error);
                });

                // Only regenerate if we have a reasonable number of results
                showGeneratingState();
                const documentIds = filteredResults.slice(0, 5).map(s => s.doc_id);

                // Use streaming if available
                if (listen) {
                    displayStreamingResponse();

                    // Setup streaming listeners
                    listen('llm-stream-chunk', (event) => {
                        appendStreamChunk(event.payload);
                    }).then(unlistenChunk => {
                        listen('llm-stream-complete', () => {
                            console.log('Stream completed for filtered results');
                            unlistenChunk();
                        }).then(unlistenComplete => {
                            // Start streaming
                            invoke('generate_response_stream', {
                                query: lastSearchQuery,
                                contextSources: documentIds
                            }).catch(error => {
                                console.error('Streaming failed, falling back:', error);
                                unlistenChunk();
                                unlistenComplete();
                                // Fallback to non-streaming
                                invoke('generate_response', {
                                    query: lastSearchQuery,
                                    contextSources: documentIds
                                }).then(aiResponse => {
                                    displayAIResponse(aiResponse);
                                });
                            });
                        });
                    });
                } else {
                    // Non-streaming fallback
                    invoke('generate_response', {
                        query: lastSearchQuery,
                        contextSources: documentIds
                    }).then(aiResponse => {
                        console.log('AI response for filtered results:', aiResponse);
                        displayAIResponse(aiResponse);
                    }).catch(error => {
                        console.error('Failed to generate AI response for filtered results:', error);
                        const aiSection = document.getElementById('ai-response-section');
                        if (aiSection && error.toString().includes('cancelled')) {
                            aiSection.innerHTML = `
                                <div class="answer-section">
                                    <h3>🤖 AI Response:</h3>
                                    <p><em>Generation was cancelled by filter change</em></p>
                                </div>
                            `;
                        }
                    });
                }
            }
        }
    }

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
            lastSearchQuery = ''; // Clear last search when no query
            return;
        }

        // Store the query for potential re-search when cutoff changes
        lastSearchQuery = query;

        // Get the current similarity cutoff value
        const cutoff = parseFloat(similarityCutoff.value);

        try {
            console.log('Searching for:', query, 'with cutoff:', cutoff);

            // Step 1: Get search hits immediately with cutoff 0.0 to get all results
            showLoadingState(query);
            // Always fetch with cutoff 0.0 to get top 100 results for caching
            const searchHits = await invoke('search_hits', { query, cutoff: 0.0 });
            console.log('Search hits (fetched with cutoff 0.0, got', searchHits.sources?.length || 0, 'results):', searchHits);

            // Cache ALL results for client-side filtering
            if (searchHits.sources) {
                allSearchResults = searchHits.sources;
                console.log('Cached', allSearchResults.length, 'results for client-side filtering');

                // Auto-set similarity threshold to show top 5-10 results
                const autoThreshold = calculateOptimalThreshold(allSearchResults);
                if (autoThreshold !== cutoff) {
                    console.log('Auto-setting similarity threshold from', cutoff, 'to', autoThreshold);
                    similarityCutoff.value = autoThreshold;
                    similarityValue.textContent = autoThreshold.toFixed(1);
                }

                // Now filter based on the auto-set threshold
                const finalCutoff = autoThreshold;
                const filteredSources = allSearchResults.filter(s => s.similarity >= finalCutoff);
                searchHits.sources = filteredSources;
                searchHits.has_results = filteredSources.length > 0;
                console.log('Filtered to', filteredSources.length, 'results with auto-threshold', finalCutoff);
            }

            // Display search hits immediately
            displaySearchHits(searchHits);

            if (searchHits.has_results) {
                // Step 2: Cancel any ongoing generation and start new one
                try {
                    await invoke('cancel_generation');
                    console.log('Cancelled previous generation requests');
                } catch (error) {
                    console.warn('Failed to cancel previous generation:', error);
                }

                // Generate AI response in background with streaming
                showGeneratingState();
                const documentIds = searchHits.sources.map(s => s.doc_id);

                // Setup streaming listeners if available
                if (listen) {
                    displayStreamingResponse();

                    // Listen for streaming chunks
                    const unlistenChunk = await listen('llm-stream-chunk', (event) => {
                        appendStreamChunk(event.payload);
                    });

                    // Listen for stream completion
                    const unlistenComplete = await listen('llm-stream-complete', () => {
                        console.log('Stream completed');
                        // Cleanup listeners
                        unlistenChunk();
                        unlistenComplete();
                    });

                    // Start streaming generation
                    try {
                        await invoke('generate_response_stream', {
                            query,
                            contextSources: documentIds
                        });
                    } catch (error) {
                        console.error('Failed to start streaming:', error);
                        // Cleanup listeners on error
                        unlistenChunk();
                        unlistenComplete();

                        // Fallback to non-streaming
                        const aiResponse = await invoke('generate_response', {
                            query,
                            contextSources: documentIds
                        });
                        displayAIResponse(aiResponse);
                    }
                } else {
                    // Fallback to non-streaming if listen is not available
                    try {
                        const aiResponse = await invoke('generate_response', {
                            query,
                            contextSources: documentIds
                        });
                        console.log('AI response:', aiResponse);
                        displayAIResponse(aiResponse);
                    } catch (error) {
                        console.error('Failed to generate AI response:', error);
                        const aiSection = document.getElementById('ai-response-section');
                        if (aiSection) {
                            if (error.toString().includes('cancelled')) {
                                aiSection.innerHTML = `
                                    <div class="answer-section">
                                        <h3>🤖 AI Response:</h3>
                                        <p><em>Generation was cancelled by new search</em></p>
                                    </div>
                                `;
                            } else {
                                aiSection.innerHTML = `
                                    <div class="answer-section">
                                        <h3>🤖 AI Response:</h3>
                                        <p><em>Failed to generate response: ${error}</em></p>
                                    </div>
                                `;
                            }
                        }
                    }
                }
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

    function displayStreamingResponse() {
        const aiSection = document.getElementById('ai-response-section');
        if (aiSection) {
            aiSection.innerHTML = `
                <div class="answer-section">
                    <h3>🤖 AI Response:</h3>
                    <p id="streaming-response"></p>
                </div>
            `;
        }
    }

    function appendStreamChunk(chunk) {
        const responseElement = document.getElementById('streaming-response');
        if (responseElement) {
            // Append the chunk and escape HTML
            const currentText = responseElement.textContent || '';
            responseElement.textContent = currentText + chunk;
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

    // Export calculateOptimalThreshold for testing
    window.calculateOptimalThreshold = calculateOptimalThreshold;

    // Unit tests for calculateOptimalThreshold
    function runThresholdTests() {
        const tests = [
            {
                name: 'Empty results returns 0.3',
                input: [],
                expected: 0.3
            },
            {
                name: 'Few results returns minimum of last result or 0.3',
                input: [0.8, 0.7, 0.6, 0.5],
                expectedMin: 0.3,
                expectedMax: 0.5
            },
            {
                name: 'Many results calculates decile threshold',
                input: [0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.35, 0.3, 0.25, 0.2, 0.15, 0.1],
                expectedMin: 0.3,
                expectedMax: 0.7
            },
            {
                name: 'All low scores enforces 0.3 minimum',
                input: [0.25, 0.2, 0.15, 0.1, 0.05],
                expected: 0.3
            },
            {
                name: 'Very similar high scores',
                input: [0.85, 0.84, 0.83, 0.82, 0.81, 0.80, 0.79, 0.78],
                expectedMin: 0.78,
                expectedMax: 0.82
            }
        ];

        let passed = 0;
        tests.forEach(test => {
            const mockResults = test.input.map((sim, i) => ({ similarity: sim }));
            const result = calculateOptimalThreshold(mockResults);

            let testPassed = false;
            if (test.expected !== undefined) {
                testPassed = result === test.expected;
            } else {
                testPassed = result >= test.expectedMin && result <= test.expectedMax;
            }

            if (testPassed) {
                passed++;
                console.log(`✅ ${test.name}: ${result}`);
            } else {
                console.log(`❌ ${test.name}: got ${result}, expected ${test.expected || `${test.expectedMin}-${test.expectedMax}`}`);
            }
        });

        console.log(`Tests: ${passed}/${tests.length} passed`);
        return passed === tests.length;
    }

    // Run tests on load if in development
    if (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1') {
        setTimeout(runThresholdTests, 1000);
    }

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