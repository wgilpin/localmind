# Implementation Plan: Canvas-Based Domain Content Extraction

**Feature Branch**: `001-canvas-domain-extraction`  
**Created**: December 11, 2025  
**Specification**: [spec.md](./spec.md)

## Architecture Overview

### Current System

The LocalMind system consists of:

1. **Chrome Extension** (`chrome-extension/`)
   - Manifest V3 extension with popup UI
   - `content.js`: Extracts page content via `document.body.innerText` (standard DOM)
   - `popup.js`: Handles user interactions
   - `background.js`: Service worker that POSTs data to backend
   - Currently supports: `activeTab`, `scripting` permissions
   - Connects to: `http://localhost:3000` (desktop-daemon server)

2. **Backend Server** (`desktop-daemon/`)
   - Node.js/Express server listening on port 3000
   - Receives documents via `POST /documents` endpoint
   - Processes content through RAG pipeline (Ollama + ChromaDB)
   - Handles YouTube transcript extraction
   - Auto-monitors Chrome bookmarks

3. **Data Flow** (Current)
   ```
   User clicks Save → content.js extracts DOM → popup.js → background.js 
   → POST /documents → RAG service → Vector store + Database
   ```

### New Architecture with Canvas Extraction

```
User clicks Save → Check special domains config (JSON) 
                 ├─ Standard domain → document.body.innerText → backend
                 └─ Special domain → Clipboard extraction flow:
                                   ├─ Check clipboard permissions
                                   │  ├─ Granted → Select all + Copy
                                   │  └─ Denied → Show dialog (grant/fallback)
                                   ├─ Capture clipboard content
                                   ├─ Validate content (not empty)
                                   │  ├─ Valid → Send to backend
                                   │  └─ Empty → Show warning (retry/save anyway)
                                   └─ Restore original clipboard
```

## Technical Design

### 1. Configuration System

**Storage**: `chrome.storage.local` (browser-managed)  
**Location**: Extension storage directory (managed by browser)  
**Format**: JSON with domain patterns

```json
{
  "version": "1.0",
  "enabled": true,
  "domains": [
    {
      "pattern": "docs.google.com",
      "description": "Google Docs (canvas-rendered)",
      "enabled": true,
      "match_type": "domain"
    },
    {
      "pattern": "*.figma.com",
      "description": "Figma design editor",
      "enabled": true,
      "match_type": "subdomain"
    }
  ]
}
```

**Note**: Configuration is stored in browser extension storage, not as a file. Users export/import JSON files for editing.

**Configuration Manager** (`config-manager.js`):
```javascript
class ConfigManager {
  constructor() {
    this.configKey = 'special_domains_config';
    this.defaultConfig = { /* Default JSON config */ };
  }
  
  async loadConfig() { /* Load from chrome.storage.local */ }
  async saveConfig(config) { /* Validate and save */ }
  async exportConfig() { /* Export as JSON string */ }
  async importConfig(jsonString) { /* Import from JSON */ }
  isSpecialDomain(url) { /* Match URL against patterns */ }
  validateConfig(config) { /* Validate JSON structure */ }
}
```

### 2. Chrome Extension Modifications

#### 2.1 Manifest.json Updates

**Add permissions**:
```json
{
  "permissions": [
    "activeTab",
    "scripting",
    "clipboardRead",
    "clipboardWrite",
    "storage"
  ]
}
```

#### 2.2 New Files Structure

```
chrome-extension/
├── manifest.json (updated)
├── background.js (updated)
├── popup.js (updated)
├── popup.html (updated)
├── content.js (updated)
├── content-clipboard.js (NEW)
├── config-manager.js (NEW)
├── config/
│   └── special-domains-default.json (NEW)
├── ui/
│   ├── dialogs.js (NEW)
│   └── dialogs.css (NEW)
└── images/ (existing)
```

#### 2.3 Content Script Updates

**content.js** (Updated - orchestrates extraction):
```javascript
// Main extraction orchestrator
(async () => {
  const configManager = new ConfigManager();
  await configManager.loadConfig();
  
  const currentUrl = window.location.href;
  const isSpecial = configManager.isSpecialDomain(currentUrl);
  
  let extractionData;
  if (isSpecial) {
    // Use clipboard-based extraction
    extractionData = await performClipboardExtraction(currentUrl);
  } else {
    // Standard DOM extraction
    extractionData = {
      title: document.title,
      url: currentUrl,
      content: document.body.innerText,
      extractionMethod: 'dom'
    };
  }
  
  // Send to popup
  chrome.runtime.sendMessage({
    action: 'pageDetails',
    data: extractionData
  });
})();
```

**content-clipboard.js** (NEW - handles clipboard operations):
```javascript
async function performClipboardExtraction(url) {
  try {
    // Check clipboard permissions
    const hasPermission = await checkClipboardPermission();
    if (!hasPermission) {
      return await handlePermissionDenied(url);
    }
    
    // Save current clipboard
    const originalClipboard = await readClipboard();
    
    // Select all and copy
    document.execCommand('selectAll');
    document.execCommand('copy');
    
    // Small delay for clipboard operation
    await sleep(100);
    
    // Read clipboard content
    const extractedContent = await readClipboard();
    
    // Restore original clipboard
    await writeClipboard(originalClipboard);
    
    // Validate content
    if (isContentEmpty(extractedContent)) {
      return await handleEmptyContent(url, document.title);
    }
    
    return {
      title: document.title,
      url: url,
      content: extractedContent,
      extractionMethod: 'clipboard',
      success: true
    };
    
  } catch (error) {
    console.error('Clipboard extraction failed:', error);
    return await handleExtractionError(error, url);
  }
}

async function checkClipboardPermission() {
  try {
    const result = await navigator.permissions.query({ name: 'clipboard-read' });
    return result.state === 'granted' || result.state === 'prompt';
  } catch {
    // Fallback: try to read clipboard
    try {
      await navigator.clipboard.readText();
      return true;
    } catch {
      return false;
    }
  }
}

async function readClipboard() {
  return await navigator.clipboard.readText();
}

async function writeClipboard(text) {
  await navigator.clipboard.writeText(text);
}

function isContentEmpty(content) {
  return !content || content.trim().length < 10; // Threshold: 10 chars
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}
```

#### 2.4 Error Handling UI

**dialogs.js** (NEW - modal dialogs for errors):
```javascript
class ExtractionDialogs {
  // Show permission denied dialog
  showPermissionDialog(onGrant, onFallback) {
    // Create modal overlay
    // Options: "Grant Permission" | "Use Fallback Extraction"
  }
  
  // Show empty content warning
  showEmptyContentDialog(onRetry, onSaveAnyway) {
    // Create modal
    // Warning message
    // Options: "Retry" | "Save Anyway"
  }
  
  // Show extraction progress
  showProgressIndicator(message) {
    // Non-blocking progress indicator
  }
  
  // Show error message
  showError(message) {
    // Error toast/modal
  }
}
```

**dialogs.css** (NEW - styling for modals):
```css
.extraction-modal {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 10000;
  background: white;
  border-radius: 8px;
  box-shadow: 0 4px 20px rgba(0,0,0,0.3);
  padding: 24px;
  min-width: 400px;
}

.extraction-modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0,0,0,0.5);
  z-index: 9999;
}

/* Additional styles for buttons, warnings, etc. */
```

#### 2.5 Config Manager Implementation

**config-manager.js** (NEW):
```javascript
// No external dependencies needed - native JSON support

class ConfigManager {
  constructor() {
    this.configKey = 'special_domains_config';
    this.config = null;
  }
  
  async loadConfig() {
    try {
      const stored = await chrome.storage.local.get(this.configKey);
      
      if (stored[this.configKey]) {
        this.config = stored[this.configKey];
      } else {
        // Load default config
        this.config = await this.loadDefaultConfig();
        await this.saveConfig(this.config);
      }
      
      return this.config;
    } catch (error) {
      console.error('Failed to load config:', error);
      return this.getHardcodedDefaults();
    }
  }
  
  async loadDefaultConfig() {
    // Fetch default JSON file
    const response = await fetch(chrome.runtime.getURL('config/special-domains-default.json'));
    const jsonText = await response.text();
    return JSON.parse(jsonText);
  }
  
  async saveConfig(config) {
    const validated = this.validateConfig(config);
    if (!validated.valid) {
      throw new Error(`Invalid config: ${validated.errors.join(', ')}`);
    }
    
    await chrome.storage.local.set({ [this.configKey]: config });
    this.config = config;
  }
  
  isSpecialDomain(url) {
    if (!this.config || !this.config.enabled) {
      return false;
    }
    
    const urlObj = new URL(url);
    const hostname = urlObj.hostname;
    
    for (const domain of this.config.domains) {
      if (!domain.enabled) continue;
      
      if (this.matchesDomain(hostname, domain.pattern, domain.match_type)) {
        return true;
      }
    }
    
    return false;
  }
  
  matchesDomain(hostname, pattern, matchType) {
    switch (matchType) {
      case 'domain':
        return hostname === pattern;
      
      case 'subdomain':
        if (pattern.startsWith('*.')) {
          const baseDomain = pattern.substring(2);
          return hostname === baseDomain || hostname.endsWith('.' + baseDomain);
        }
        return hostname === pattern;
      
      case 'pattern':
        // Convert glob pattern to regex
        const regex = this.globToRegex(pattern);
        return regex.test(hostname);
      
      default:
        return hostname === pattern;
    }
  }
  
  globToRegex(pattern) {
    const escaped = pattern
      .replace(/\./g, '\\.')
      .replace(/\*/g, '.*')
      .replace(/\?/g, '.');
    return new RegExp('^' + escaped + '$');
  }
  
  validateConfig(config) {
    const errors = [];
    
    if (!config.version) {
      errors.push('Missing version field');
    }
    
    if (typeof config.enabled !== 'boolean') {
      errors.push('enabled must be a boolean');
    }
    
    if (!Array.isArray(config.domains)) {
      errors.push('domains must be an array');
    } else {
      config.domains.forEach((domain, index) => {
        if (!domain.pattern) {
          errors.push(`Domain ${index}: missing pattern`);
        }
        if (!['domain', 'subdomain', 'pattern'].includes(domain.match_type)) {
          errors.push(`Domain ${index}: invalid match_type`);
        }
      });
    }
    
    return {
      valid: errors.length === 0,
      errors
    };
  }
  
  getHardcodedDefaults() {
    return {
      version: '1.0',
      enabled: true,
      domains: [
        {
          pattern: 'docs.google.com',
          description: 'Google Docs',
          enabled: true,
          match_type: 'domain'
        }
      ]
    };
  }
  
  async exportConfig() {
    // Export current config as formatted JSON string
    return JSON.stringify(this.config, null, 2);
  }
  
  async importConfig(jsonString) {
    try {
      const parsed = JSON.parse(jsonString);
      await this.saveConfig(parsed);
      return { success: true };
    } catch (error) {
      return { success: false, error: error.message };
    }
  }
  
  getConfigLocation() {
    // Return helpful message about where config is stored
    return "Configuration is stored in the browser's extension storage. " +
           "Use the extension popup to export/import JSON configuration.";
  }
}

// Export for use in other scripts
if (typeof module !== 'undefined' && module.exports) {
  module.exports = ConfigManager;
}
```

#### 2.6 Popup UI Updates

**popup.html** (Add config UI):
```html
<!DOCTYPE html>
<html>
<head>
  <link rel="stylesheet" href="popup.css">
</head>
<body>
  <div class="container">
    <h1>LocalMind</h1>
    
    <!-- Existing buttons -->
    <button id="save-button">Save Page</button>
    <button id="show-note-input-button">Add Note</button>
    
    <!-- NEW: Config button -->
    <button id="config-button">⚙️ Config</button>
    
    <!-- Existing note input -->
    <div id="note-input-container" class="hidden">
      <textarea id="note-content" placeholder="Enter note..."></textarea>
      <button id="add-note-button">Save Note</button>
    </div>
    
    <!-- NEW: Config panel -->
    <div id="config-panel" class="hidden">
      <h3>Special Domains Configuration</h3>
      <div class="config-info">
        <p>Domains using canvas rendering require clipboard extraction.</p>
      </div>
      <div class="config-actions">
        <button id="export-config">Export JSON</button>
        <button id="import-config">Import JSON</button>
        <button id="view-location">View Storage Location</button>
      </div>
      <div id="config-list">
        <!-- Dynamically populated domain list -->
      </div>
    </div>
    
    <!-- Status messages -->
    <div id="status-message"></div>
  </div>
  
  <script src="config-manager.js"></script>
  <script src="popup.js"></script>
</body>
</html>
```

**popup.js** (Add config handling):
```javascript
// Existing DOMContentLoaded event handler
document.addEventListener('DOMContentLoaded', () => {
  const saveButton = document.getElementById('save-button');
  const configButton = document.getElementById('config-button');
  const configPanel = document.getElementById('config-panel');
  const exportConfigBtn = document.getElementById('export-config');
  const importConfigBtn = document.getElementById('import-config');
  const statusMessage = document.getElementById('status-message');
  
  const configManager = new ConfigManager();
  
  // Load config on popup open
  configManager.loadConfig().then(config => {
    renderConfigList(config);
  });
  
  // Config button toggle
  configButton.addEventListener('click', () => {
    configPanel.classList.toggle('hidden');
  });
  
  // Export config
  exportConfigBtn.addEventListener('click', async () => {
    const json = await configManager.exportConfig();
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'special-domains.json';
    a.click();
    URL.revokeObjectURL(url);
    
    statusMessage.textContent = 'Config exported!';
    statusMessage.style.color = 'green';
  });
  
  // Import config
  importConfigBtn.addEventListener('click', () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = async (e) => {
      const file = e.target.files[0];
      const text = await file.text();
      const result = await configManager.importConfig(text);
      
      if (result.success) {
        statusMessage.textContent = 'Config imported successfully!';
        statusMessage.style.color = 'green';
        const config = await configManager.loadConfig();
        renderConfigList(config);
      } else {
        statusMessage.textContent = `Import failed: ${result.error}`;
        statusMessage.style.color = 'red';
      }
    };
    input.click();
  });
  
  // View storage location
  document.getElementById('view-location').addEventListener('click', () => {
    const location = configManager.getConfigLocation();
    alert(location);
  });
  
  // Existing save button logic
  saveButton.addEventListener('click', () => {
    statusMessage.textContent = 'Extracting content...';
    statusMessage.style.color = 'blue';
    
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      const activeTab = tabs[0];
      if (activeTab) {
        chrome.scripting.executeScript({
          target: { tabId: activeTab.id },
          files: ['config-manager.js', 'content-clipboard.js', 'content.js']
        }, () => {
          console.log('Content scripts executed.');
        });
      }
    });
  });
  
  // Render config list
  function renderConfigList(config) {
    const listContainer = document.getElementById('config-list');
    listContainer.innerHTML = '<h4>Configured Domains:</h4>';
    
    if (config && config.domains) {
      const list = document.createElement('ul');
      config.domains.forEach(domain => {
        const item = document.createElement('li');
        item.textContent = `${domain.pattern} - ${domain.description} ${domain.enabled ? '✓' : '✗'}`;
        item.className = domain.enabled ? 'enabled' : 'disabled';
        list.appendChild(item);
      });
      listContainer.appendChild(list);
    }
  }
  
  // Listen for extraction results (existing + updated)
  chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message.action === 'pageDetails') {
      const statusMessage = document.getElementById('status-message');
      
      // Check extraction method
      if (message.data.extractionMethod === 'clipboard') {
        statusMessage.textContent = 'Using clipboard extraction for canvas content...';
      }
      
      chrome.runtime.sendMessage({
        action: 'sendPageData',
        data: message.data
      }, (response) => {
        if (response && response.success) {
          statusMessage.textContent = 'Page saved successfully!';
          statusMessage.style.color = 'green';
          setTimeout(() => {
            window.close();
          }, 1000);
        } else {
          statusMessage.textContent = `Error: ${response ? response.error : 'Unknown error'}`;
          statusMessage.style.color = 'red';
          console.error('Save operation failed:', response ? response.error : 'No response');
        }
      });
    }
  });
});
```

### 3. Backend Updates

#### 3.1 Document Endpoint Enhancement

**desktop-daemon/src/index.ts** (Update `/documents` endpoint):

```typescript
app.post("/documents", async (req: any, res: any) => {
  try {
    let { title, content, url, extractionMethod } = req.body;

    if (!title || !content) {
      return res
        .status(400)
        .json({ message: "Title and content are required." });
    }

    // Log extraction method for analytics
    console.log(`Document from ${extractionMethod || 'unknown'} extraction: ${title}`);

    // YouTube transcript handling (existing)
    if (url && url.includes("youtube.com/watch")) {
      try {
        const transcript = await YoutubeTranscript.fetchTranscript(url);
        if (transcript.length > 0) {
          content = transcript.map((t: { text: any }) => t.text).join(" ");
        }
        title = title.replace(/^\([^)]*\)\s*/, "");
      } catch (youtubeError) {
        console.warn(
          `Could not fetch YouTube transcript for ${url}:`,
          youtubeError
        );
      }
    }

    // Add document with extraction method metadata
    await ragService.addDocuments([{ 
      title, 
      content, 
      url,
      metadata: { extractionMethod: extractionMethod || 'dom' }
    }]);
    
    res.status(200).json({ 
      message: "Document added successfully.",
      extractionMethod: extractionMethod || 'dom'
    });
  } catch (error) {
    console.error("Error adding document:", error);
    res.status(500).json({ message: "Failed to add document." });
  }
});
```

#### 3.2 Database Schema Update

**desktop-daemon/src/services/database.ts** (Add extraction method tracking):

```typescript
// Add field to documents table
async function initializeDatabase() {
  // Existing initialization...
  
  // Check if extraction_method column exists
  const columnCheck = db.prepare(`
    SELECT COUNT(*) as count 
    FROM pragma_table_info('documents') 
    WHERE name='extraction_method'
  `).get();
  
  if (columnCheck.count === 0) {
    console.log('Adding extraction_method column to documents table...');
    db.prepare(`
      ALTER TABLE documents 
      ADD COLUMN extraction_method TEXT DEFAULT 'dom'
    `).run();
  }
}
```

### 4. Build & Bundling

#### 4.1 Package Dependencies

**chrome-extension/package.json** (NEW):
```json
{
  "name": "localmind-extension",
  "version": "1.0.0",
  "scripts": {
    "build": "webpack --config webpack.config.js",
    "watch": "webpack --watch"
  },
  "dependencies": {},
  "devDependencies": {
    "webpack": "^5.89.0",
    "webpack-cli": "^5.1.4",
    "copy-webpack-plugin": "^11.0.0"
  }
}
```

**Note**: No runtime dependencies needed! JSON parsing is native to JavaScript.

#### 4.2 Webpack Configuration

**chrome-extension/webpack.config.js** (NEW):
```javascript
const path = require('path');
const CopyPlugin = require('copy-webpack-plugin');

module.exports = {
  mode: 'production',
  entry: {
    'background': './background.js',
    'popup': './popup.js',
    'content': './content.js',
    'content-clipboard': './content-clipboard.js',
    'config-manager': './config-manager.js',
    'dialogs': './ui/dialogs.js'
  },
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: '[name].js'
  },
  plugins: [
    new CopyPlugin({
      patterns: [
        { from: 'manifest.json', to: 'manifest.json' },
        { from: 'popup.html', to: 'popup.html' },
        { from: 'popup.css', to: 'popup.css' },
        { from: 'ui/dialogs.css', to: 'ui/dialogs.css' },
        { from: 'config/special-domains-default.json', to: 'config/special-domains-default.json' },
        { from: 'images', to: 'images' }
      ]
    })
  ]
};
```

## Implementation Phases

### Phase 1: Configuration System (Priority: P1)

**Tasks**:
1. Create JSON configuration file structure
2. Implement ConfigManager class
3. Add config storage/retrieval logic (chrome.storage.local)
4. Implement domain matching algorithms
5. Add config validation
6. Unit tests for ConfigManager

**Deliverables**:
- `config-manager.js`
- `config/special-domains-default.json`
- Tests for domain matching

**Acceptance Criteria**:
- Can load/save config from chrome.storage
- Domain matching works for all types (domain, subdomain, pattern)
- Invalid configs are rejected with clear errors

**Complexity**: Medium (domain matching logic, YAML parsing)

---

### Phase 2: Clipboard Extraction Core (Priority: P1)

**Tasks**:
1. Update manifest.json with clipboard permissions
2. Implement clipboard read/write functions
3. Implement select-all + copy operations
4. Add clipboard save/restore logic
5. Implement content validation
6. Add error handling for clipboard failures

**Deliverables**:
- `content-clipboard.js`
- Updated `manifest.json`
- Updated `content.js` (orchestration)

**Acceptance Criteria**:
- Successfully extracts content from docs.google.com
- Original clipboard is restored
- Handles permission errors gracefully
- Detects empty content

**Complexity**: High (browser APIs, async operations, error handling)

---

### Phase 3: User Interface & Error Handling (Priority: P2)

**Tasks**:
1. Create modal dialog system
2. Implement permission denied dialog
3. Implement empty content warning dialog
4. Add progress indicators
5. Style dialogs with CSS
6. Integrate dialogs with extraction flow

**Deliverables**:
- `ui/dialogs.js`
- `ui/dialogs.css`
- Updated `content-clipboard.js` (with dialog triggers)

**Acceptance Criteria**:
- Permission denied shows dialog with grant/fallback options
- Empty content shows retry/save anyway options
- Dialogs are visually consistent with extension style
- User choices are respected

**Complexity**: Medium (UI/UX, CSS styling, modal management)

---

### Phase 4: Config UI in Popup (Priority: P2)

**Tasks**:
1. Update popup.html with config panel
2. Add export/import JSON buttons
3. Implement domain list display
4. Add config location helper
5. Update popup.css

**Deliverables**:
- Updated `popup.html`
- Updated `popup.js`
- Updated `popup.css`

**Acceptance Criteria**:
- Config panel toggles visibility
- Can export config as JSON file
- Can import JSON config file
- Shows list of configured domains with status

**Complexity**: Low (UI updates, file handling)

---

### Phase 5: Backend Integration (Priority: P3)

**Tasks**:
1. Update `/documents` endpoint to accept extractionMethod
2. Add database schema migration for extraction_method field
3. Update document metadata handling
4. Add logging for extraction method analytics

**Deliverables**:
- Updated `desktop-daemon/src/index.ts`
- Updated `desktop-daemon/src/services/database.ts`

**Acceptance Criteria**:
- Backend accepts extractionMethod parameter
- Database stores extraction method
- Existing documents continue to work (backward compatible)

**Complexity**: Low (simple parameter addition, optional field)

---

### Phase 6: Build System & Packaging (Priority: P3)

**Tasks**:
1. Create package.json for extension
2. Set up webpack configuration
3. Configure js-yaml bundling
4. Add build scripts
5. Test bundled extension

**Deliverables**:
- `package.json`
- `webpack.config.js`
- `dist/` directory with bundled extension

**Acceptance Criteria**:
- `npm run build` produces working extension in `dist/`
- All dependencies bundled correctly
- Extension loads in Chrome without errors

**Complexity**: Low (standard webpack setup)

---

### Phase 7: Testing & Refinement (Priority: P3)

**Tasks**:
1. End-to-end testing on docs.google.com
2. Test permission denial scenarios
3. Test empty content scenarios
4. Test config import/export
5. Test domain matching edge cases
6. Cross-browser testing (Chrome, Edge)
7. Performance testing (large documents)

**Deliverables**:
- Test results documentation
- Bug fixes
- Performance optimizations

**Acceptance Criteria**:
- All user stories from spec pass acceptance scenarios
- Success criteria SC-001 through SC-006 are met
- No critical bugs

**Complexity**: Medium (comprehensive testing, edge cases)

## Technical Decisions

### 1. Why JSON for Configuration?

- **Native support**: No external dependencies, uses built-in JSON.parse/stringify
- **Browser storage compatibility**: chrome.storage.local works natively with JSON objects
- **Performance**: Native parsing is faster than YAML libraries
- **Simplicity**: Reduces bundle size (~20KB saved by not including js-yaml)
- **Human-readable**: Still readable for export/import by advanced users
- **Clarification**: Updated from YAML based on storage location decision

### 2. Why Chrome Extension Storage vs Local File?

- **Security**: Browser-managed storage is secure
- **Permissions**: Doesn't require file system permissions
- **Portability**: Works consistently across OS
- **Sync**: Can leverage chrome.storage.sync if needed later
- **Clarification**: "Extension storage directory" selected by user

### 3. Clipboard Restoration Strategy

- **Why save/restore**: Prevents data loss for users
- **Timing**: Save before operation, restore after capture
- **Failure handling**: If restore fails, log warning but don't block
- **Alternative considered**: Not restoring (rejected - poor UX)

### 4. Content Validation Threshold

- **10 characters minimum**: Reasonable signal of content
- **Why not more**: Some legitimate pages are short
- **User override**: Warning allows "Save Anyway"

## Dependencies

### Extension
- **No runtime dependencies**: Native JSON support only
- **webpack** (^5.89.0): Module bundling (dev dependency)
- **copy-webpack-plugin** (^11.0.0): Copy static assets (dev dependency)

### Backend
- No new dependencies (uses existing Express/TypeScript stack)

### Browser Requirements
- Chrome 88+ (Manifest V3 support)
- Clipboard API support (Chrome 66+)
- Chrome Extension Storage API

## Security Considerations

### 1. Clipboard Access
- **Risk**: Extension can read clipboard content
- **Mitigation**: Only read during explicit user action (Save button click)
- **Disclosure**: Declare clipboardRead in manifest, user must approve

### 2. Configuration Injection
- **Risk**: Malicious JSON could contain unexpected structures
- **Mitigation**: Validate all config after parsing, strict schema validation
- **Validation**: Whitelist pattern structure, validate all field types

### 3. Content Injection
- **Risk**: Extracted content could contain malicious scripts
- **Mitigation**: Backend sanitizes content before storage/display
- **Existing**: Backend already handles untrusted content

### 4. Permission Escalation
- **Risk**: Users might grant excessive permissions
- **Mitigation**: Request minimal permissions, clear documentation
- **Principle**: Least privilege approach

## Performance Considerations

### 1. Clipboard Operations
- **Impact**: Select-all + copy can be slow on large documents
- **Mitigation**: Show progress indicator, timeout after 5 seconds
- **Measurement**: Log operation time, alert if > 3 seconds

### 2. Config Loading
- **Impact**: Loading config on every page save
- **Mitigation**: Cache config in memory, only reload on changes
- **Measurement**: Config load should be < 50ms

### 3. Domain Matching
- **Impact**: Regex matching can be expensive
- **Mitigation**: Cache compiled regexes, limit pattern complexity
- **Measurement**: Domain check should be < 10ms

### 4. JSON Parsing
- **Impact**: Native JSON.parse is very fast
- **Mitigation**: Parse once on load, cache in memory
- **Measurement**: Parse should be < 10ms for reasonable configs (native performance)

## Monitoring & Analytics

### Metrics to Track

1. **Extraction Success Rate**
   - DOM extractions: success/failure counts
   - Clipboard extractions: success/failure counts
   - Target: ≥95% success rate (SC-002)

2. **Performance Metrics**
   - Extraction time: p50, p95, p99
   - Target: <3 seconds (SC-001)

3. **Permission Metrics**
   - Permission denial rate
   - Fallback usage rate
   - Permission grant after dialog

4. **Config Metrics**
   - Number of special domains configured
   - Most common special domains
   - Config validation errors

5. **Error Metrics**
   - Empty content warnings
   - Clipboard operation failures
   - Backend ingestion failures

### Logging Strategy

```javascript
// Log format
{
  timestamp: '2025-12-11T10:30:00Z',
  action: 'clipboard_extraction',
  domain: 'docs.google.com',
  success: true,
  duration_ms: 1250,
  content_length: 5432,
  error: null
}
```

## Rollback Plan

### If Issues Arise

1. **Clipboard extraction fails**:
   - User can choose fallback to DOM extraction
   - Config can disable clipboard feature globally

2. **Performance issues**:
   - Increase timeouts
   - Add configuration option to disable for specific domains

3. **Permission issues**:
   - Fall back to DOM extraction automatically
   - Clear messaging to user

4. **Backend incompatibility**:
   - extractionMethod is optional parameter
   - Backend defaults to 'dom' if not specified
   - Fully backward compatible

### Rollback Steps

1. Set `config.enabled = false` in default config
2. Users can still use standard DOM extraction
3. Deploy patch to remove clipboard code if necessary
4. No database rollback needed (field addition is non-breaking)

## Documentation Requirements

### User Documentation

1. **README.md** (chrome-extension/):
   - Installation instructions
   - How to use clipboard extraction
   - Configuring special domains
   - Troubleshooting permission issues

2. **CONFIG.md** (chrome-extension/):
   - JSON configuration format
   - Match type explanations
   - Example configurations
   - Export/import workflow for advanced users

3. **CHANGELOG.md**:
   - Feature addition
   - Breaking changes (none)
   - Migration guide (none needed)

### Developer Documentation

1. **ARCHITECTURE.md**:
   - System design
   - Component interactions
   - Data flow diagrams

2. **API.md**:
   - ConfigManager API
   - ExtractionDialogs API
   - Content script messaging protocol

3. **TESTING.md**:
   - Test scenarios
   - Manual testing checklist
   - Automated test coverage

## Success Criteria Validation

### SC-001: 3-Second Extraction Time
**Test**: Time extraction on 10 different Google Docs  
**Pass**: Average ≤ 3 seconds, p95 ≤ 5 seconds  
**Method**: Performance.now() timing in content script

### SC-002: 95% Success Rate
**Test**: 100 extractions across various special domains  
**Pass**: ≥ 95 successful extractions  
**Method**: Automated test suite + manual testing

### SC-003: Search Accuracy Parity
**Test**: Search for same queries on DOM vs clipboard extracted content  
**Pass**: Similarity scores within 5% margin  
**Method**: Backend search quality tests

### SC-004: 2-Minute Configuration
**Test**: Time to add new domain and verify  
**Pass**: Advanced user can complete in ≤ 2 minutes  
**Method**: User testing with instructions

### SC-005: Seamless Mixed Content
**Test**: Bookmark 10 pages (5 special, 5 standard)  
**Pass**: All indexed without user intervention  
**Method**: Verify no error messages, all searchable

### SC-006: Zero OAuth
**Test**: Review code for OAuth implementation  
**Pass**: No OAuth libraries or flows present  
**Method**: Code review + dependency audit

## Implementation Dependencies

**Critical Path**: Phases 1 → 2 → 3 (core functionality must be sequential)  
**Parallel Work Possible**: Phase 4 can start after Phase 1, Phase 5 is independent

**Phase Dependencies**:
- Phase 2 requires Phase 1 (needs ConfigManager to check if domain is special)
- Phase 3 requires Phase 2 (dialogs integrate with clipboard extraction)
- Phase 4 requires Phase 1 (config UI needs ConfigManager)
- Phase 5 is independent (backend changes don't block frontend)
- Phase 6 requires all code complete (bundles everything)
- Phase 7 requires all phases (comprehensive testing)

**Suggested Order**: 1 → 2 → 3 → 4 → 6 → 5 → 7
(Backend integration can come later since extractionMethod parameter is optional)

## Next Steps

1. ✅ Review and approve this plan
2. ⏭️ Begin Phase 1: Configuration System implementation
3. Set up development environment (install dependencies)
4. Create feature branch tracking document
5. Implement phases sequentially following dependency order

---

**Plan Status**: Draft  
**Reviewer**: Pending  
**Approved**: Pending

