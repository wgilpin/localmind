/**
 * ConfigManager - Manages special domain configuration for canvas-based content extraction
 * 
 * Stores configuration in chrome.storage.local and provides domain matching logic
 * to determine which domains require clipboard-based extraction.
 */
class ConfigManager {
  constructor() {
    this.configKey = 'special_domains_config';
    this.config = null;
  }

  /**
   * Load configuration from chrome.storage.local
   * Falls back to default config if not found
   * @returns {Promise<Object>} The loaded configuration
   */
  async loadConfig() {
    try {
      const stored = await chrome.storage.local.get(this.configKey);
      
      if (stored[this.configKey]) {
        this.config = stored[this.configKey];
      } else {
        // Load default config from JSON file
        this.config = await this.loadDefaultConfig();
        await this.saveConfig(this.config);
      }
      
      return this.config;
    } catch (error) {
      console.error('Failed to load config:', error);
      return this.getHardcodedDefaults();
    }
  }

  /**
   * Load default configuration from JSON file
   * @returns {Promise<Object>} The default configuration
   */
  async loadDefaultConfig() {
    try {
      const response = await fetch(chrome.runtime.getURL('config/special-domains-default.json'));
      const jsonText = await response.text();
      return JSON.parse(jsonText);
    } catch (error) {
      console.error('Failed to load default config file:', error);
      return this.getHardcodedDefaults();
    }
  }

  /**
   * Save configuration to chrome.storage.local after validation
   * @param {Object} config - Configuration object to save
   * @throws {Error} If configuration is invalid
   */
  async saveConfig(config) {
    const validated = this.validateConfig(config);
    if (!validated.valid) {
      throw new Error(`Invalid config: ${validated.errors.join(', ')}`);
    }
    
    await chrome.storage.local.set({ [this.configKey]: config });
    this.config = config;
  }

  /**
   * Check if a URL belongs to a special domain requiring clipboard extraction
   * @param {string} url - The URL to check
   * @returns {boolean} True if domain requires special handling
   */
  isSpecialDomain(url) {
    if (!this.config || !this.config.enabled) {
      return false;
    }
    
    try {
      const urlObj = new URL(url);
      const hostname = urlObj.hostname;
      
      for (const domain of this.config.domains) {
        if (!domain.enabled) continue;
        
        if (this.matchesDomain(hostname, domain.pattern, domain.match_type)) {
          return true;
        }
      }
      
      return false;
    } catch (error) {
      console.error('Error checking special domain:', error);
      return false;
    }
  }

  /**
   * Match a hostname against a domain pattern
   * @param {string} hostname - The hostname to match
   * @param {string} pattern - The pattern to match against
   * @param {string} matchType - Type of matching: 'domain', 'subdomain', or 'pattern'
   * @returns {boolean} True if hostname matches pattern
   */
  matchesDomain(hostname, pattern, matchType) {
    switch (matchType) {
      case 'domain':
        // Exact domain match
        return hostname === pattern;
      
      case 'subdomain':
        // Wildcard subdomain match (*.example.com)
        if (pattern.startsWith('*.')) {
          const baseDomain = pattern.substring(2);
          return hostname === baseDomain || hostname.endsWith('.' + baseDomain);
        }
        return hostname === pattern;
      
      case 'pattern':
        // Glob pattern to regex match
        const regex = this.globToRegex(pattern);
        return regex.test(hostname);
      
      default:
        return hostname === pattern;
    }
  }

  /**
   * Convert glob pattern to regular expression
   * @param {string} pattern - Glob pattern
   * @returns {RegExp} Regular expression
   */
  globToRegex(pattern) {
    const escaped = pattern
      .replace(/\./g, '\\.') // Escape dots
      .replace(/\*/g, '.*')  // Convert * to .*
      .replace(/\?/g, '.');  // Convert ? to .
    return new RegExp('^' + escaped + '$');
  }

  /**
   * Validate configuration object structure
   * @param {Object} config - Configuration to validate
   * @returns {Object} Validation result with {valid: boolean, errors: string[]}
   */
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
          errors.push(`Domain ${index}: invalid match_type (must be 'domain', 'subdomain', or 'pattern')`);
        }
        if (typeof domain.enabled !== 'boolean') {
          errors.push(`Domain ${index}: enabled must be a boolean`);
        }
      });
    }
    
    return {
      valid: errors.length === 0,
      errors
    };
  }

  /**
   * Get hardcoded default configuration as fallback
   * @returns {Object} Default configuration
   */
  getHardcodedDefaults() {
    return {
      version: '1.0',
      enabled: true,
      domains: [
        {
          pattern: 'docs.google.com',
          description: 'Google Docs (canvas-rendered)',
          enabled: true,
          match_type: 'domain'
        }
      ]
    };
  }

  /**
   * Export current configuration as formatted JSON string
   * @returns {Promise<string>} JSON string
   */
  async exportConfig() {
    if (!this.config) {
      await this.loadConfig();
    }
    return JSON.stringify(this.config, null, 2);
  }

  /**
   * Import configuration from JSON string
   * @param {string} jsonString - JSON configuration string
   * @returns {Promise<Object>} Result with {success: boolean, error?: string}
   */
  async importConfig(jsonString) {
    try {
      const parsed = JSON.parse(jsonString);
      await this.saveConfig(parsed);
      return { success: true };
    } catch (error) {
      return { success: false, error: error.message };
    }
  }

  /**
   * Get helpful message about configuration storage location
   * @returns {string} Location information
   */
  getConfigLocation() {
    return "Configuration is stored in the browser's extension storage. " +
           "Use the extension popup to export/import JSON configuration.";
  }
}

// Export for use in other scripts
if (typeof module !== 'undefined' && module.exports) {
  module.exports = ConfigManager;
}


