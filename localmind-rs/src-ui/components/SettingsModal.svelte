<script>
import { getTauriAPI } from '../tauri.svelte.js';

let { show = false, onClose, onSave } = $props();

const { invoke } = getTauriAPI();

let excludedFolders = $state([]);
let excludedDomains = $state([]);
let originalFolders = $state([]);
let originalDomains = $state([]);
let folders = $state([]);
let patternInput = $state('');
let patternValidation = $state({ message: '', valid: true });
let loading = $state(false);

$effect(() => {
    if (show) {
        loadSettings();
    }
});

async function loadSettings() {
    loading = true;
    try {
        const rules = await invoke('get_exclusion_rules');
        excludedFolders = rules.excluded_folders || [];
        excludedDomains = rules.excluded_domains || [];
        originalFolders = [...excludedFolders];
        originalDomains = [...excludedDomains];

        await loadFolders();
    } catch (error) {
        console.error('Failed to load settings:', error);
    }
    loading = false;
}

async function loadFolders() {
    try {
        folders = await invoke('get_bookmark_folders');
    } catch (error) {
        console.error('Failed to load folders:', error);
        folders = [];
    }
}

function toggleFolder(folderId) {
    if (excludedFolders.includes(folderId)) {
        excludedFolders = excludedFolders.filter(id => id !== folderId);
    } else {
        excludedFolders = [...excludedFolders, folderId];
    }
}

function removePattern(pattern) {
    excludedDomains = excludedDomains.filter(p => p !== pattern);
}

async function addPattern() {
    const pattern = patternInput.trim();
    if (!pattern) return;

    try {
        const validation = await invoke('validate_domain_pattern', { pattern });

        if (!validation.valid) {
            patternValidation = { message: validation.error_message, valid: false };
            return;
        }

        if (excludedDomains.includes(pattern)) {
            patternValidation = { message: 'Pattern already exists', valid: false };
            return;
        }

        excludedDomains = [...excludedDomains, pattern];
        patternInput = '';
        patternValidation = { message: '', valid: true };
    } catch (error) {
        console.error('Failed to validate pattern:', error);
        patternValidation = { message: 'Validation failed', valid: false };
    }
}

async function validatePattern(pattern) {
    if (!pattern) {
        patternValidation = { message: '', valid: true };
        return;
    }

    try {
        const validation = await invoke('validate_domain_pattern', { pattern });
        if (validation.valid) {
            patternValidation = { message: 'Valid pattern', valid: true };
        } else {
            patternValidation = { message: validation.error_message, valid: false };
        }
    } catch (error) {
        console.error('Validation error:', error);
    }
}

let validationTimeout;
$effect(() => {
    clearTimeout(validationTimeout);
    if (patternInput) {
        validationTimeout = setTimeout(() => {
            validatePattern(patternInput);
        }, 300);
    } else {
        patternValidation = { message: '', valid: true };
    }
});

function hasChanges() {
    const foldersChanged = JSON.stringify([...excludedFolders].sort()) !== JSON.stringify([...originalFolders].sort());
    const domainsChanged = JSON.stringify([...excludedDomains].sort()) !== JSON.stringify([...originalDomains].sort());
    return foldersChanged || domainsChanged;
}

async function saveSettings() {
    try {
        const result = await invoke('set_exclusion_rules', {
            folders: excludedFolders,
            domains: excludedDomains
        });

        let message = 'Exclusion rules updated';
        if (result.bookmarks_removed > 0) {
            message += `. ${result.bookmarks_removed} bookmarks excluded`;
        }
        if (result.bookmarks_added > 0) {
            message += `. ${result.bookmarks_added} bookmarks added`;
        }

        onSave?.(message);
        onClose?.();
    } catch (error) {
        console.error('Failed to save settings:', error);
        throw error;
    }
}

function handleClose() {
    excludedFolders = [...originalFolders];
    excludedDomains = [...originalDomains];
    onClose?.();
}

function handleKeydown(e) {
    if (e.key === 'Escape') {
        handleClose();
    }
}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if show}
    <div class="modal show" onclick={(e) => e.target.classList.contains('modal') && handleClose()}>
        <div class="modal-content">
            <div class="modal-header">
                <h2>Settings</h2>
                <button class="close-btn" onclick={handleClose}>×</button>
            </div>

            <div class="modal-body">
                <section class="settings-section">
                    <h3>Excluded Bookmark Folders</h3>
                    <div class="folder-tree">
                        {#if loading}
                            <div class="loading">Loading folders...</div>
                        {:else if folders.length === 0}
                            <div class="empty-state">No bookmark folders found.</div>
                        {:else}
                            {#each folders as folder}
                                <div class="folder-item">
                                    <input
                                        type="checkbox"
                                        id="folder-{folder.id}"
                                        checked={excludedFolders.includes(folder.id)}
                                        onchange={() => toggleFolder(folder.id)}
                                    />
                                    <label for="folder-{folder.id}">
                                        {folder.path.join(' > ') || folder.name}
                                    </label>
                                    <span class="folder-count">({folder.bookmark_count})</span>
                                </div>
                            {/each}
                        {/if}
                    </div>
                </section>

                <section class="settings-section">
                    <h3>Excluded Domain Patterns</h3>
                    <div class="domain-patterns-list">
                        {#if excludedDomains.length === 0}
                            <div class="empty-state">No domain patterns excluded.</div>
                        {:else}
                            {#each excludedDomains as pattern}
                                <div class="domain-pattern-item">
                                    <span class="pattern-text">{pattern}</span>
                                    <div class="pattern-actions">
                                        <button onclick={() => removePattern(pattern)}>×</button>
                                    </div>
                                </div>
                            {/each}
                        {/if}
                    </div>
                    <div class="add-pattern-container">
                        <input
                            type="text"
                            bind:value={patternInput}
                            placeholder="Enter domain pattern (e.g., *.internal.com)"
                            onkeypress={(e) => e.key === 'Enter' && addPattern()}
                        />
                        <button onclick={addPattern}>+ Add Pattern</button>
                    </div>
                    <div class="validation-message" class:error={!patternValidation.valid} class:success={patternValidation.valid && patternValidation.message}>
                        {patternValidation.message}
                    </div>
                </section>

                <section class="settings-section">
                    <div class="preview-info" class:has-changes={hasChanges()}>
                        <span>{hasChanges() ? 'You have unsaved changes' : 'No changes'}</span>
                    </div>
                </section>
            </div>

            <div class="modal-footer">
                <button class="btn-secondary" onclick={handleClose}>Cancel</button>
                <button class="btn-primary" onclick={saveSettings} disabled={!hasChanges()}>Save Changes</button>
            </div>
        </div>
    </div>
{/if}

<style>
.modal {
    display: none;
    position: fixed;
    z-index: 1000;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    background-color: rgba(0, 0, 0, 0.5);
}

.modal.show {
    display: flex;
    align-items: center;
    justify-content: center;
}

.modal-content {
    background-color: white;
    border-radius: 8px;
    width: 90%;
    max-width: 700px;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
}

.modal-header {
    padding: 20px;
    border-bottom: 1px solid #e5e7eb;
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.modal-header h2 {
    margin: 0;
    color: #2563eb;
}

.modal-header .close-btn {
    background: none;
    border: none;
    font-size: 2rem;
    cursor: pointer;
    color: #6b7280;
    line-height: 1;
    padding: 0;
    width: 32px;
    height: 32px;
}

.modal-header .close-btn:hover {
    color: #374151;
}

.modal-body {
    padding: 20px;
    overflow-y: auto;
    flex: 1;
}

.modal-footer {
    padding: 20px;
    border-top: 1px solid #e5e7eb;
    display: flex;
    justify-content: flex-end;
    gap: 10px;
}

.settings-section {
    margin-bottom: 30px;
}

.settings-section h3 {
    margin-bottom: 15px;
    color: #374151;
    font-size: 1.1rem;
}

.folder-tree {
    border: 1px solid #e5e7eb;
    border-radius: 4px;
    padding: 15px;
    max-height: 200px;
    overflow-y: auto;
    background: #f9fafb;
}

.folder-item {
    padding: 5px 0;
    display: flex;
    align-items: center;
    gap: 8px;
}

.folder-item input[type="checkbox"] {
    cursor: pointer;
}

.folder-item label {
    cursor: pointer;
    flex: 1;
}

.folder-count {
    color: #6b7280;
    font-size: 0.9rem;
}

.domain-patterns-list {
    border: 1px solid #e5e7eb;
    border-radius: 4px;
    padding: 15px;
    min-height: 100px;
    background: #f9fafb;
    margin-bottom: 15px;
}

.domain-pattern-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 4px;
    margin-bottom: 8px;
}

.domain-pattern-item:last-child {
    margin-bottom: 0;
}

.pattern-text {
    flex: 1;
    font-family: 'Courier New', monospace;
}

.pattern-actions button {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1.2rem;
    padding: 4px;
    border-radius: 4px;
    transition: background-color 0.2s;
}

.pattern-actions button:hover {
    background-color: #f3f4f6;
}

.empty-state {
    color: #9ca3af;
    text-align: center;
    padding: 20px;
    font-style: italic;
}

.add-pattern-container {
    display: flex;
    gap: 10px;
    margin-bottom: 10px;
}

.add-pattern-container input {
    flex: 1;
    padding: 10px;
    border: 1px solid #d1d5db;
    border-radius: 4px;
    font-size: 1rem;
}

.add-pattern-container input:focus {
    outline: none;
    border-color: #2563eb;
}

.add-pattern-container button {
    padding: 10px 20px;
    background-color: #2563eb;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    transition: background-color 0.2s;
}

.add-pattern-container button:hover {
    background-color: #1d4ed8;
}

.validation-message {
    min-height: 24px;
    padding: 5px;
    font-size: 0.9rem;
}

.validation-message.error {
    color: #dc2626;
}

.validation-message.success {
    color: #16a34a;
}

.preview-info {
    background: #eff6ff;
    border: 1px solid #bfdbfe;
    border-radius: 4px;
    padding: 15px;
    color: #1e40af;
}

.preview-info.has-changes {
    background: #fef3c7;
    border-color: #fcd34d;
    color: #92400e;
}

.btn-primary {
    padding: 10px 24px;
    background-color: #2563eb;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    transition: background-color 0.2s;
}

.btn-primary:hover {
    background-color: #1d4ed8;
}

.btn-primary:disabled {
    background-color: #9ca3af;
    cursor: not-allowed;
}

.btn-secondary {
    padding: 10px 24px;
    background-color: #f3f4f6;
    color: #374151;
    border: 1px solid #d1d5db;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    transition: background-color 0.2s;
}

.btn-secondary:hover {
    background-color: #e5e7eb;
}

.loading {
    text-align: center;
    color: #6b7280;
    padding: 20px;
}
</style>
