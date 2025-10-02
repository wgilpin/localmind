<script>
let { onSearch, similarityCutoff = 0.2, onSimilarityChange, onSettingsClick } = $props();

let query = $state('');
let cutoff = $state(similarityCutoff);

$effect(() => {
    cutoff = similarityCutoff;
});

function handleSearch() {
    if (query.trim()) {
        onSearch?.(query.trim(), cutoff);
    }
}

function handleKeypress(e) {
    if (e.key === 'Enter') {
        handleSearch();
    }
}

function handleCutoffChange(e) {
    const value = parseFloat(e.target.value);
    cutoff = value;
    onSimilarityChange?.(value);
}
</script>

<header>
    <h1>LocalMind</h1>
    <button class="settings-btn" title="Settings" onclick={onSettingsClick}>⚙️</button>
</header>

<div class="search-container">
    <input
        type="text"
        bind:value={query}
        onkeypress={handleKeypress}
        placeholder="Search your knowledge base..."
    />
    <button onclick={handleSearch}>Search</button>
</div>

<div class="settings-container">
    <div class="similarity-setting">
        <label for="similarity-cutoff">Similarity Threshold:</label>
        <input
            type="range"
            id="similarity-cutoff"
            min="0.1"
            max="0.9"
            step="0.1"
            value={cutoff}
            oninput={handleCutoffChange}
        />
        <span id="similarity-value">{cutoff.toFixed(1)}</span>
        <small>(Lower = more results, Higher = more precise)</small>
    </div>
</div>

<style>
header {
    text-align: center;
    margin-bottom: 40px;
    position: relative;
}

header h1 {
    font-size: 2.5rem;
    color: #2563eb;
}

.settings-btn {
    position: absolute;
    top: 0;
    right: 0;
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    padding: 8px;
    border-radius: 4px;
    transition: background-color 0.2s;
}

.settings-btn:hover {
    background-color: rgba(0, 0, 0, 0.05);
}

.search-container {
    display: flex;
    gap: 10px;
    margin-bottom: 20px;
}

.search-container input {
    flex: 1;
    padding: 12px 16px;
    border: 1px solid #ddd;
    border-radius: 8px;
    font-size: 16px;
}

.search-container input:focus {
    outline: none;
    border-color: #2563eb;
    box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.1);
}

.search-container button {
    padding: 12px 24px;
    background-color: #2563eb;
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 16px;
    cursor: pointer;
}

.search-container button:hover {
    background-color: #1d4ed8;
}

.settings-container {
    background: white;
    padding: 15px;
    border-radius: 8px;
    margin-bottom: 20px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.similarity-setting {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
}

.similarity-setting label {
    font-weight: 500;
    color: #374151;
    min-width: 140px;
}

#similarity-cutoff {
    flex: 1;
    min-width: 200px;
    height: 6px;
    background: #e5e7eb;
    border-radius: 3px;
    outline: none;
    appearance: none;
    cursor: pointer;
}

#similarity-cutoff::-webkit-slider-thumb {
    appearance: none;
    width: 20px;
    height: 20px;
    background: #2563eb;
    border-radius: 50%;
    cursor: pointer;
}

#similarity-cutoff::-moz-range-thumb {
    width: 20px;
    height: 20px;
    background: #2563eb;
    border-radius: 50%;
    cursor: pointer;
    border: none;
}

#similarity-value {
    font-weight: 600;
    color: #2563eb;
    min-width: 30px;
    text-align: center;
}

.similarity-setting small {
    color: #6b7280;
    font-size: 0.85rem;
    margin-left: 10px;
}
</style>
