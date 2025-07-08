document.addEventListener('DOMContentLoaded', () => {
    const searchInput = document.getElementById('search-input');
    const searchButton = document.getElementById('search-button');
    const resultsContainer = document.getElementById('results-container');
    const resultsSection = document.getElementById('results-section');
    const noteTitle = document.getElementById('note-title');
    const noteContent = document.getElementById('note-content');
    searchInput.focus();
    const saveNoteButton = document.getElementById('save-note-button');
    const newNoteSection = document.getElementById('new-note-section');
    const fab = document.getElementById('fab');

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
            const result = await response.json();
            resultsSection.hidden = false;
            resultsContainer.innerHTML = `<pre>${JSON.stringify(result, null, 2)}</pre>`;
        } catch (error) {
            console.error('Error during search:', error);
            resultsContainer.innerHTML = `<p style="color: red;">Error: ${error.message}</p>`;
        }
    });

    fab.addEventListener('click', () => {
        newNoteSection.hidden = !newNoteSection.hidden;
    });

    saveNoteButton.addEventListener('click', async () => {
        const title = noteTitle.value;
        const content = noteContent.value;

        if (!title || !content) {
            alert('Please enter both title and content for the note.');
            return;
        }

        try {
            const response = await fetch('/documents', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ title, content }),
            });

            if (response.ok) {
                alert('Note saved successfully!');
                noteTitle.value = '';
                noteContent.value = '';
            } else {
                const errorData = await response.json();
                alert(`Failed to save note: ${errorData.message || response.statusText}`);
            }
        } catch (error) {
            console.error('Error saving note:', error);
            alert(`Error saving note: ${error.message}`);
        }
    });
});