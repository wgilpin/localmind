document.addEventListener('DOMContentLoaded', () => {
    const searchInput = document.getElementById('search-input') as HTMLInputElement;
    const searchButton = document.getElementById('search-button');
    const resultsContainer = document.getElementById('results-container');
    const resultsSection = document.getElementById('results-section');

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
                const results = data.result;

                if (resultsContainer) {
                    resultsContainer.innerHTML = '';
                    if (results && results.length > 0) {
                        results.forEach((result: { content: string }) => {
                            const resultElement = document.createElement('div');
                            resultElement.textContent = result.content;
                            resultsContainer.appendChild(resultElement);
                        });
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