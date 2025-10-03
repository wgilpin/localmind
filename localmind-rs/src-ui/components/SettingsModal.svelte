<script>
import { getTauriAPI } from '../tauri.svelte.js';
import FolderTreeNode from './FolderTreeNode.svelte';

let { show = false, onClose, onSave } = $props();

const tauri = getTauriAPI();

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
        const rules = await tauri.invoke('get_exclusion_rules');
        console.log('loadSettings: received rules from backend =', rules);
        excludedFolders = rules.excluded_folders || [];
        excludedDomains = rules.excluded_domains || [];
        console.log('loadSettings: excludedFolders =', $state.snapshot(excludedFolders));
        console.log('loadSettings: excludedDomains =', $state.snapshot(excludedDomains));
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
        folders = await tauri.invoke('get_bookmark_folders');
    } catch (error) {
        console.error('Failed to load folders:', error);
        folders = [];
    }
}

function buildFolderTree(folders) {
    const tree = {};

    for (const folder of folders) {
        let current = tree;

        for (let i = 0; i < folder.path.length; i++) {
            const pathPart = folder.path[i];

            if (!current[pathPart]) {
                current[pathPart] = {
                    name: pathPart,
                    children: {},
                    folder: null,
                    expanded: true
                };
            }

            if (i === folder.path.length - 1) {
                current[pathPart].folder = folder;
            }

            current = current[pathPart].children;
        }
    }

    return tree;
}

let folderTree = $derived(buildFolderTree(folders));

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
        const validation = await tauri.invoke('validate_domain_pattern', { pattern });

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
        const validation = await tauri.invoke('validate_domain_pattern', { pattern });
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

function getPreviewText() {
    if (!hasChanges()) {
        return 'No changes - configure exclusions above';
    }

    const addedFolders = excludedFolders.filter(f => !originalFolders.includes(f)).length;
    const removedFolders = originalFolders.filter(f => !excludedFolders.includes(f)).length;
    const addedDomains = excludedDomains.filter(d => !originalDomains.includes(d)).length;
    const removedDomains = originalDomains.filter(d => !excludedDomains.includes(d)).length;

    let changes = [];
    if (addedFolders > 0) changes.push(`${addedFolders} folder(s) will be excluded`);
    if (removedFolders > 0) changes.push(`${removedFolders} folder(s) will be included`);
    if (addedDomains > 0) changes.push(`${addedDomains} domain pattern(s) added`);
    if (removedDomains > 0) changes.push(`${removedDomains} domain pattern(s) removed`);

    return changes.join(', ');
}

async function saveSettings() {
    try {
        console.log('saveSettings: excludedFolders =', $state.snapshot(excludedFolders));
        console.log('saveSettings: excludedDomains =', $state.snapshot(excludedDomains));

        const result = await tauri.invoke('set_exclusion_rules', {
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
    <div
        class="modal show"
        role="button"
        tabindex="-1"
        onclick={(e) => e.target.classList.contains('modal') && handleClose()}
        onkeydown={(e) => e.key === 'Enter' && e.target.classList.contains('modal') && handleClose()}
    >
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
                            {#each Object.values(folderTree) as rootNode}
                                <FolderTreeNode
                                    node={rootNode}
                                    depth={0}
                                    {excludedFolders}
                                    onToggle={toggleFolder}
                                />
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
                        <span>{getPreviewText()}</span>
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
    background-color: rgba(0, 0, 0, 0.7);
}

.modal.show {
    display: flex;
    align-items: center;
    justify-content: center;
}

.modal-content {
    background-color: #1a1f2e;
    border: 1px solid #2d3548;
    border-radius: 8px;
    width: 90%;
    max-width: 700px;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
}

.modal-header {
    padding: 20px;
    border-bottom: 1px solid #2d3548;
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.modal-header h2 {
    margin: 0;
    color: #60a5fa;
    font-size: 1.5rem;
}

.modal-header .close-btn {
    background: none;
    border: none;
    font-size: 2rem;
    cursor: pointer;
    color: #9ca3af;
    line-height: 1;
    padding: 0;
    width: 32px;
    height: 32px;
    transition: color 0.2s;
}

.modal-header .close-btn:hover {
    color: #e5e7eb;
}

.modal-body {
    padding: 20px;
    overflow-y: auto;
    flex: 1;
}

.modal-footer {
    padding: 20px;
    border-top: 1px solid #2d3548;
    display: flex;
    justify-content: flex-end;
    gap: 10px;
}

.settings-section {
    margin-bottom: 30px;
}

.settings-section h3 {
    margin-bottom: 15px;
    color: #e5e7eb;
    font-size: 1.1rem;
}

.folder-tree {
    border: 1px solid #2d3548;
    border-radius: 4px;
    padding: 15px;
    max-height: 200px;
    overflow-y: auto;
    background: #0f1419;
}

.domain-patterns-list {
    border: 1px solid #2d3548;
    border-radius: 4px;
    padding: 15px;
    min-height: 100px;
    background: #0f1419;
    margin-bottom: 15px;
}

.domain-pattern-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: #1a1f2e;
    border: 1px solid #2d3548;
    border-radius: 4px;
    margin-bottom: 8px;
}

.domain-pattern-item:last-child {
    margin-bottom: 0;
}

.pattern-text {
    flex: 1;
    font-family: 'Courier New', monospace;
    color: #d1d5db;
}

.pattern-actions button {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1.2rem;
    padding: 4px;
    border-radius: 4px;
    color: #9ca3af;
    transition: all 0.2s;
}

.pattern-actions button:hover {
    background-color: #2d3548;
    color: #e5e7eb;
}

.empty-state {
    color: #6b7280;
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
    border: 1px solid #2d3548;
    border-radius: 4px;
    font-size: 1rem;
    background: #0f1419;
    color: #e5e7eb;
}

.add-pattern-container input:focus {
    outline: none;
    border-color: #60a5fa;
}

.add-pattern-container button {
    padding: 10px 20px;
    background-color: #60a5fa;
    color: #0f1419;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: 500;
    transition: background-color 0.2s;
}

.add-pattern-container button:hover {
    background-color: #3b82f6;
}

.validation-message {
    min-height: 24px;
    padding: 5px;
    font-size: 0.9rem;
}

.validation-message.error {
    color: #f87171;
}

.validation-message.success {
    color: #4ade80;
}

.preview-info {
    background: #1e3a5f;
    border: 1px solid #2d5a8c;
    border-radius: 4px;
    padding: 15px;
    color: #93c5fd;
}

.preview-info.has-changes {
    background: #422006;
    border-color: #78350f;
    color: #fbbf24;
}

.btn-primary {
    padding: 10px 24px;
    background-color: #60a5fa;
    color: #0f1419;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: 500;
    transition: background-color 0.2s;
}

.btn-primary:hover {
    background-color: #3b82f6;
}

.btn-primary:disabled {
    background-color: #4b5563;
    color: #6b7280;
    cursor: not-allowed;
}

.btn-secondary {
    padding: 10px 24px;
    background-color: #2d3548;
    color: #e5e7eb;
    border: 1px solid #4b5563;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    transition: background-color 0.2s;
}

.btn-secondary:hover {
    background-color: #374151;
}

.loading {
    text-align: center;
    color: #9ca3af;
    padding: 20px;
}
</style>
