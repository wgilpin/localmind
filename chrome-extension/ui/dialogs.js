/**
 * ExtractionDialogs - Modal dialogs for extraction error handling
 * 
 * Provides user-friendly dialogs for clipboard permission issues,
 * empty content warnings, and general error messages.
 */
class ExtractionDialogs {
  constructor() {
    this.activeDialog = null;
  }

  /**
   * Show permission denied dialog with grant/fallback options
   * @param {Function} onGrant - Callback when user chooses to grant permissions
   * @param {Function} onFallback - Callback when user chooses fallback extraction
   */
  showPermissionDialog(onGrant, onFallback) {
    this.closeActiveDialog();
    
    const overlay = this.createOverlay();
    const modal = this.createModal('permission-dialog');
    
    modal.innerHTML = `
      <div class="dialog-header">
        <h3>Clipboard Permission Required</h3>
      </div>
      <div class="dialog-content">
        <p>This page uses canvas rendering and requires clipboard access to extract content.</p>
        <p>You can either:</p>
        <ul>
          <li><strong>Grant Permission</strong> - Allow clipboard access for better extraction</li>
          <li><strong>Use Fallback</strong> - Extract using standard method (may miss some content)</li>
        </ul>
      </div>
      <div class="dialog-actions">
        <button class="btn-primary" id="grant-permission-btn">Grant Permission</button>
        <button class="btn-secondary" id="use-fallback-btn">Use Fallback</button>
      </div>
    `;
    
    // Add event listeners
    modal.querySelector('#grant-permission-btn').addEventListener('click', () => {
      this.closeActiveDialog();
      onGrant();
    });
    
    modal.querySelector('#use-fallback-btn').addEventListener('click', () => {
      this.closeActiveDialog();
      onFallback();
    });
    
    overlay.appendChild(modal);
    document.body.appendChild(overlay);
    this.activeDialog = overlay;
  }

  /**
   * Show empty content warning dialog with retry/save options
   * @param {Function} onRetry - Callback when user chooses to retry
   * @param {Function} onSaveAnyway - Callback when user chooses to save anyway
   */
  showEmptyContentDialog(onRetry, onSaveAnyway) {
    this.closeActiveDialog();
    
    const overlay = this.createOverlay();
    const modal = this.createModal('empty-content-dialog');
    
    modal.innerHTML = `
      <div class="dialog-header warning">
        <h3>⚠️ Empty Content Detected</h3>
      </div>
      <div class="dialog-content">
        <p>The extracted content appears to be empty or very short.</p>
        <p>This might happen if:</p>
        <ul>
          <li>The page hasn't fully loaded yet</li>
          <li>The content is hidden or requires interaction</li>
          <li>The page is actually empty</li>
        </ul>
        <p>What would you like to do?</p>
      </div>
      <div class="dialog-actions">
        <button class="btn-primary" id="retry-btn">Retry Extraction</button>
        <button class="btn-secondary" id="save-anyway-btn">Save Anyway</button>
      </div>
    `;
    
    // Add event listeners
    modal.querySelector('#retry-btn').addEventListener('click', () => {
      this.closeActiveDialog();
      onRetry();
    });
    
    modal.querySelector('#save-anyway-btn').addEventListener('click', () => {
      this.closeActiveDialog();
      onSaveAnyway();
    });
    
    overlay.appendChild(modal);
    document.body.appendChild(overlay);
    this.activeDialog = overlay;
  }

  /**
   * Show progress indicator for extraction
   * @param {string} message - Progress message to display
   */
  showProgressIndicator(message) {
    this.closeActiveDialog();
    
    const overlay = this.createOverlay();
    const modal = this.createModal('progress-indicator');
    
    modal.innerHTML = `
      <div class="dialog-content progress">
        <div class="spinner"></div>
        <p>${message || 'Extracting content...'}</p>
      </div>
    `;
    
    overlay.appendChild(modal);
    document.body.appendChild(overlay);
    this.activeDialog = overlay;
  }

  /**
   * Show general error message
   * @param {string} message - Error message to display
   */
  showError(message) {
    this.closeActiveDialog();
    
    const overlay = this.createOverlay();
    const modal = this.createModal('error-dialog');
    
    modal.innerHTML = `
      <div class="dialog-header error">
        <h3>❌ Extraction Error</h3>
      </div>
      <div class="dialog-content">
        <p>${message || 'An error occurred during extraction.'}</p>
        <p>The system will attempt to use standard extraction instead.</p>
      </div>
      <div class="dialog-actions">
        <button class="btn-primary" id="close-error-btn">OK</button>
      </div>
    `;
    
    // Add event listener
    modal.querySelector('#close-error-btn').addEventListener('click', () => {
      this.closeActiveDialog();
    });
    
    // Auto-close after 5 seconds
    setTimeout(() => {
      this.closeActiveDialog();
    }, 5000);
    
    overlay.appendChild(modal);
    document.body.appendChild(overlay);
    this.activeDialog = overlay;
  }

  /**
   * Create overlay element
   * @returns {HTMLElement} Overlay element
   */
  createOverlay() {
    const overlay = document.createElement('div');
    overlay.className = 'extraction-modal-overlay';
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) {
        this.closeActiveDialog();
      }
    });
    return overlay;
  }

  /**
   * Create modal element
   * @param {string} className - Additional class name for the modal
   * @returns {HTMLElement} Modal element
   */
  createModal(className) {
    const modal = document.createElement('div');
    modal.className = `extraction-modal ${className || ''}`;
    return modal;
  }

  /**
   * Close currently active dialog
   */
  closeActiveDialog() {
    if (this.activeDialog && this.activeDialog.parentNode) {
      this.activeDialog.parentNode.removeChild(this.activeDialog);
      this.activeDialog = null;
    }
  }
}

// Make available globally
if (typeof window !== 'undefined') {
  window.ExtractionDialogs = ExtractionDialogs;
}


