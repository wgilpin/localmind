<script>
let { onSearch, onSettingsClick } = $props();

let query = $state('');

function handleSearch() {
    if (query.trim()) {
        onSearch?.(query.trim());
    }
}

function handleKeypress(e) {
    if (e.key === 'Enter') {
        handleSearch();
    }
}
</script>

<header class="app-header">
    <div class="header-content">
        <div class="brand">
            <svg class="logo" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"></path>
            </svg>
            <h1>LocalMind <span class="version">v2</span></h1>
        </div>
        <button class="settings-btn" title="Settings" onclick={onSettingsClick}>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="3"></circle>
                <path d="M12 1v6m0 6v6M5.64 5.64l4.24 4.24m4.24 4.24l4.24 4.24M1 12h6m6 0h6M5.64 18.36l4.24-4.24m4.24-4.24l4.24-4.24"></path>
            </svg>
        </button>
    </div>

    <div class="search-container">
        <svg class="search-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8"></circle>
            <path d="m21 21-4.35-4.35"></path>
        </svg>
        <input
            type="text"
            bind:value={query}
            onkeypress={handleKeypress}
            placeholder="Search your knowledge base..."
            class="search-input"
        />
        <button class="search-btn" onclick={handleSearch}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="11" cy="11" r="8"></circle>
                <path d="m21 21-4.35-4.35"></path>
            </svg>
            Search
        </button>
    </div>
</header>

<style>
.app-header {
    background: linear-gradient(135deg, #1a1f2e 0%, #0f1419 100%);
    border-radius: 12px;
    border: 1px solid #2d3548;
    padding: 20px 24px;
    margin-bottom: 20px;
}

.header-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
}

.brand {
    display: flex;
    align-items: center;
    gap: 12px;
}

.logo {
    color: #8b5cf6;
}

.brand h1 {
    font-size: 1.75rem;
    font-weight: 700;
    color: #e5e7eb;
    margin: 0;
    letter-spacing: -0.5px;
}

.version {
    font-size: 0.875rem;
    font-weight: 500;
    color: #6b7280;
    margin-left: 8px;
}

.settings-btn {
    background: #374151;
    border: 1px solid #4b5563;
    border-radius: 8px;
    padding: 8px;
    color: #9ca3af;
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
}

.settings-btn:hover {
    background: #4b5563;
    color: #e5e7eb;
    transform: rotate(90deg);
}

.search-container {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
}

.search-icon {
    position: absolute;
    left: 16px;
    color: #6b7280;
    pointer-events: none;
}

.search-input {
    flex: 1;
    padding: 14px 16px 14px 48px;
    background: #0f1419;
    border: 2px solid #2d3548;
    border-radius: 10px;
    font-size: 1rem;
    color: #e5e7eb;
    transition: all 0.2s ease;
}

.search-input:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
}

.search-input::placeholder {
    color: #6b7280;
}

.search-btn {
    padding: 14px 24px;
    background: linear-gradient(135deg, #3b82f6, #2563eb);
    color: white;
    border: none;
    border-radius: 10px;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: all 0.2s ease;
}

.search-btn:hover {
    background: linear-gradient(135deg, #2563eb, #1d4ed8);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(59, 130, 246, 0.3);
}

.search-btn:active {
    transform: translateY(0);
}

@media (max-width: 768px) {
    .search-container {
        flex-direction: column;
    }

    .search-btn {
        width: 100%;
        justify-content: center;
    }
}
</style>
