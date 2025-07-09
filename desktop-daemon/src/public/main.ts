document.addEventListener('DOMContentLoaded', () => {
    const searchInput = document.getElementById('search-input') as HTMLInputElement;
    const searchButton = document.getElementById('search-button');
    const resultsContainer = document.getElementById('results-container');
    const resultsSection = document.getElementById('results-section');
    const fab = document.getElementById('fab');
    const newNoteSection = document.getElementById('new-note-section');
    const saveNoteButton = document.getElementById('save-note-button');
    const noteTitleInput = document.getElementById('note-title') as HTMLInputElement;
    const noteContentInput = document.getElementById('note-content') as HTMLTextAreaElement;

    if (fab) {
        fab.addEventListener('click', () => {
            if (newNoteSection) {
                newNoteSection.hidden = !newNoteSection.hidden;
            }
        });
    }

    if (saveNoteButton) {
        saveNoteButton.addEventListener('click', async () => {
            const title = noteTitleInput.value;
            const content = noteContentInput.value;

            if (!title || !content) {
                alert('Please enter a title and content for the note.');
                return;
            }

            try {
                const response = await fetch('/documents', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({
                        title: title,
                        content: content,
                        url: `note://${Date.now()}`
                    }),
                });

                if (!response.ok) {
                    throw new Error('Failed to save note');
                }

                noteTitleInput.value = '';
                noteContentInput.value = '';
                if (newNoteSection) {
                    newNoteSection.hidden = true;
                }
                alert('Note saved successfully!');

            } catch (error) {
                console.error('Error saving note:', error);
                alert('Failed to save note. See console for details.');
            }
        });
    }

    if (searchButton) {
        searchButton.addEventListener('click', async () => {
            const query = searchInput.value;
            if (!query) return;

            try {
                const response = await fetch('/search', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({ query }),
                });

                if (!response.ok) {
                    throw new Error('Search request failed');
                }

                const data = await response.json();
                const result = data.result;

                console.log('=== Frontend Search Debug ===');
                console.log('Response data:', data);
                console.log('Result type:', typeof result);
                console.log('Result content:', result);
                console.log('=== End Frontend Search Debug ===');

                if (resultsContainer) {
                    resultsContainer.innerHTML = '';
                    if (result && typeof result === 'string' && result.trim().length > 0) {
                        const resultElement = document.createElement('div');
                        resultElement.className = 'search-result';
                        resultElement.textContent = result;
                        resultsContainer.appendChild(resultElement);
                        if (resultsSection) {
                            resultsSection.hidden = false;
                        }
                    } else {
                        const noResultsElement = document.createElement('div');
                        noResultsElement.className = 'no-results';
                        noResultsElement.textContent = 'No results found.';
                        resultsContainer.appendChild(noResultsElement);
                        if (resultsSection) {
                            resultsSection.hidden = false;
                        }
                    }
                }
            } catch (error) {
                console.error('Error during search:', error);
            }
        });
    }
});