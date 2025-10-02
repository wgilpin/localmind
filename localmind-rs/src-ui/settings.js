// Settings modal functionality
(function() {
    const settingsBtn = document.getElementById('settings-btn');
    const settingsModal = document.getElementById('settings-modal');
    const closeSettingsBtn = document.getElementById('close-settings-btn');
    const cancelSettingsBtn = document.getElementById('cancel-settings-btn');
    const saveSettingsBtn = document.getElementById('save-settings-btn');
    const folderTreeEl = document.getElementById('folder-tree');
    const domainPatternsListEl = document.getElementById('domain-patterns-list');
    const patternInput = document.getElementById('pattern-input');
    const addPatternBtn = document.getElementById('add-pattern-btn');
    const patternValidation = document.getElementById('pattern-validation');
    const previewText = document.getElementById('preview-text');
    const previewInfo = document.getElementById('preview-info');

    let currentSettings = {
        excludedFolders: [],
        excludedDomains: []
    };

    let originalSettings = {
        excludedFolders: [],
        excludedDomains: []
    };

    // Open settings modal
    settingsBtn.addEventListener('click', async function() {
        settingsModal.classList.add('show');
        await loadSettings();
    });

    // Close modal handlers
    closeSettingsBtn.addEventListener('click', closeModal);
    cancelSettingsBtn.addEventListener('click', closeModal);

    settingsModal.addEventListener('click', function(e) {
        if (e.target === settingsModal) {
            closeModal();
        }
    });

    function closeModal() {
        settingsModal.classList.remove('show');
        // Reset to original settings
        currentSettings = JSON.parse(JSON.stringify(originalSettings));
        updatePreview();
    }

    // Load current settings
    async function loadSettings() {
        try {
            const rules = await invoke('get_exclusion_rules');
            currentSettings = {
                excludedFolders: rules.excluded_folders || [],
                excludedDomains: rules.excluded_domains || []
            };
            originalSettings = JSON.parse(JSON.stringify(currentSettings));

            renderDomainPatterns();
            await loadFolders();
            updatePreview();
        } catch (error) {
            console.error('Failed to load settings:', error);
            showToast('Failed to load settings: ' + error, 'error');
        }
    }

    // Load bookmark folders
    async function loadFolders() {
        folderTreeEl.innerHTML = '<div class="loading">Loading folders...</div>';

        try {
            const folders = await invoke('get_bookmark_folders');

            if (folders.length === 0) {
                folderTreeEl.innerHTML = '<div class="empty-state">No bookmark folders found.</div>';
                return;
            }

            // Render folder tree
            folderTreeEl.innerHTML = '';
            folders.forEach(folder => {
                const folderItem = document.createElement('div');
                folderItem.className = 'folder-item';

                const checkbox = document.createElement('input');
                checkbox.type = 'checkbox';
                checkbox.id = `folder-${folder.id}`;
                checkbox.value = folder.id;
                checkbox.checked = currentSettings.excludedFolders.includes(folder.id);
                checkbox.addEventListener('change', function() {
                    if (this.checked) {
                        if (!currentSettings.excludedFolders.includes(folder.id)) {
                            currentSettings.excludedFolders.push(folder.id);
                        }
                    } else {
                        currentSettings.excludedFolders = currentSettings.excludedFolders.filter(id => id !== folder.id);
                    }
                    updatePreview();
                });

                const label = document.createElement('label');
                label.htmlFor = `folder-${folder.id}`;
                label.textContent = folder.path.join(' > ') || folder.name;

                const count = document.createElement('span');
                count.className = 'folder-count';
                count.textContent = `(${folder.bookmark_count})`;

                folderItem.appendChild(checkbox);
                folderItem.appendChild(label);
                folderItem.appendChild(count);
                folderTreeEl.appendChild(folderItem);
            });
        } catch (error) {
            console.error('Failed to load folders:', error);
            folderTreeEl.innerHTML = '<div class="error">Failed to load folders</div>';
        }
    }

    // Render domain patterns
    function renderDomainPatterns() {
        if (currentSettings.excludedDomains.length === 0) {
            domainPatternsListEl.innerHTML = '<div class="empty-state">No domain patterns excluded.</div>';
            return;
        }

        domainPatternsListEl.innerHTML = '';
        currentSettings.excludedDomains.forEach(pattern => {
            const item = document.createElement('div');
            item.className = 'domain-pattern-item';

            const text = document.createElement('span');
            text.className = 'pattern-text';
            text.textContent = pattern;

            const actions = document.createElement('div');
            actions.className = 'pattern-actions';

            const deleteBtn = document.createElement('button');
            deleteBtn.textContent = '×';
            deleteBtn.title = 'Delete pattern';
            deleteBtn.addEventListener('click', function() {
                currentSettings.excludedDomains = currentSettings.excludedDomains.filter(p => p !== pattern);
                renderDomainPatterns();
                updatePreview();
            });

            actions.appendChild(deleteBtn);
            item.appendChild(text);
            item.appendChild(actions);
            domainPatternsListEl.appendChild(item);
        });
    }

    // Add pattern
    addPatternBtn.addEventListener('click', addPattern);
    patternInput.addEventListener('keypress', function(e) {
        if (e.key === 'Enter') {
            addPattern();
        }
    });

    async function addPattern() {
        const pattern = patternInput.value.trim();
        if (!pattern) {
            return;
        }

        try {
            const validation = await invoke('validate_domain_pattern', { pattern });

            if (!validation.valid) {
                patternValidation.textContent = validation.error_message;
                patternValidation.className = 'validation-message error';
                return;
            }

            // Check for duplicates
            if (currentSettings.excludedDomains.includes(pattern)) {
                patternValidation.textContent = 'Pattern already exists';
                patternValidation.className = 'validation-message error';
                return;
            }

            // Add pattern
            currentSettings.excludedDomains.push(pattern);
            patternInput.value = '';
            patternValidation.textContent = '';
            patternValidation.className = 'validation-message';
            renderDomainPatterns();
            updatePreview();
        } catch (error) {
            console.error('Failed to validate pattern:', error);
            patternValidation.textContent = 'Validation failed';
            patternValidation.className = 'validation-message error';
        }
    }

    // Real-time validation
    let validationTimeout;
    patternInput.addEventListener('input', function() {
        clearTimeout(validationTimeout);
        const pattern = this.value.trim();

        if (!pattern) {
            patternValidation.textContent = '';
            patternValidation.className = 'validation-message';
            return;
        }

        validationTimeout = setTimeout(async () => {
            try {
                const validation = await invoke('validate_domain_pattern', { pattern });
                if (validation.valid) {
                    patternValidation.textContent = 'Valid pattern';
                    patternValidation.className = 'validation-message success';
                } else {
                    patternValidation.textContent = validation.error_message;
                    patternValidation.className = 'validation-message error';
                }
            } catch (error) {
                console.error('Validation error:', error);
            }
        }, 300);
    });

    // Update preview
    function updatePreview() {
        const foldersChanged = JSON.stringify(currentSettings.excludedFolders.sort()) !== JSON.stringify(originalSettings.excludedFolders.sort());
        const domainsChanged = JSON.stringify(currentSettings.excludedDomains.sort()) !== JSON.stringify(originalSettings.excludedDomains.sort());
        const hasChanges = foldersChanged || domainsChanged;

        if (hasChanges) {
            previewInfo.classList.add('has-changes');
            previewText.textContent = 'You have unsaved changes';
            saveSettingsBtn.disabled = false;
        } else {
            previewInfo.classList.remove('has-changes');
            previewText.textContent = 'No changes';
            saveSettingsBtn.disabled = true;
        }
    }

    // Save settings
    saveSettingsBtn.addEventListener('click', async function() {
        try {
            saveSettingsBtn.disabled = true;
            saveSettingsBtn.textContent = 'Saving...';

            const result = await invoke('set_exclusion_rules', {
                folders: currentSettings.excludedFolders,
                domains: currentSettings.excludedDomains
            });

            originalSettings = JSON.parse(JSON.stringify(currentSettings));
            updatePreview();

            let message = 'Exclusion rules updated';
            if (result.bookmarks_removed > 0) {
                message += `. ${result.bookmarks_removed} bookmarks excluded`;
            }
            if (result.bookmarks_added > 0) {
                message += `. ${result.bookmarks_added} bookmarks added`;
            }

            showToast(message, 'success');

            saveSettingsBtn.textContent = 'Save Changes';
            settingsModal.classList.remove('show');

        } catch (error) {
            console.error('Failed to save settings:', error);
            showToast('Failed to save settings: ' + error, 'error');
            saveSettingsBtn.disabled = false;
            saveSettingsBtn.textContent = 'Save Changes';
        }
    });

    // ESC key to close modal
    document.addEventListener('keydown', function(e) {
        if (e.key === 'Escape' && settingsModal.classList.contains('show')) {
            closeModal();
        }
    });
})();
