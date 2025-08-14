import { shouldExcludeUrl, shouldExcludeBookmark } from './excludeFilter';
import { OllamaConfig } from '../config';

describe('excludeFilter', () => {
  beforeEach(() => {
    // Set up test exclude folders
    OllamaConfig.excludeFolders = ['node_modules', '.git', 'build', 'dist'];
  });

  describe('shouldExcludeUrl', () => {
    it('should exclude URLs containing excluded folder names', () => {
      expect(shouldExcludeUrl('https://github.com/user/repo/tree/main/node_modules/package')).toBe(true);
      expect(shouldExcludeUrl('https://example.com/project/.git/config')).toBe(true);
      expect(shouldExcludeUrl('https://site.com/app/build/index.html')).toBe(true);
      expect(shouldExcludeUrl('https://cdn.com/dist/bundle.js')).toBe(true);
    });

    it('should not exclude URLs not containing excluded folder names', () => {
      expect(shouldExcludeUrl('https://github.com/user/repo/src/index.js')).toBe(false);
      expect(shouldExcludeUrl('https://docs.example.com/api/reference')).toBe(false);
      expect(shouldExcludeUrl('https://blog.site.com/article')).toBe(false);
    });

    it('should handle invalid URLs gracefully', () => {
      expect(shouldExcludeUrl('not-a-valid-url')).toBe(false);
      expect(shouldExcludeUrl('')).toBe(false);
    });

    it('should handle partial matches correctly', () => {
      // Should not match partial folder names
      expect(shouldExcludeUrl('https://example.com/nodejs/tutorial')).toBe(false);
      expect(shouldExcludeUrl('https://example.com/git-tutorial/basics')).toBe(false);
    });
  });

  describe('shouldExcludeBookmark', () => {
    it('should exclude bookmarks based on URL', () => {
      expect(shouldExcludeBookmark('Package Documentation', 'https://github.com/user/repo/node_modules/pkg')).toBe(true);
    });

    it('should exclude bookmarks based on title', () => {
      expect(shouldExcludeBookmark('node_modules package info', 'https://example.com/page')).toBe(true);
      expect(shouldExcludeBookmark('Build Configuration', 'https://example.com/page')).toBe(true);
    });

    it('should not exclude normal bookmarks', () => {
      expect(shouldExcludeBookmark('React Documentation', 'https://reactjs.org/docs')).toBe(false);
      expect(shouldExcludeBookmark('API Reference', 'https://api.example.com/docs')).toBe(false);
    });

    it('should handle empty inputs gracefully', () => {
      expect(shouldExcludeBookmark('', '')).toBe(false);
      expect(shouldExcludeBookmark('Valid Title', '')).toBe(false);
    });
  });

  describe('configuration edge cases', () => {
    it('should handle empty exclude folders list', () => {
      OllamaConfig.excludeFolders = [];
      expect(shouldExcludeUrl('https://github.com/user/repo/node_modules/package')).toBe(false);
      expect(shouldExcludeBookmark('node_modules info', 'https://example.com')).toBe(false);
    });

    it('should handle undefined exclude folders', () => {
      (OllamaConfig as any).excludeFolders = undefined;
      expect(shouldExcludeUrl('https://github.com/user/repo/node_modules/package')).toBe(false);
      expect(shouldExcludeBookmark('node_modules info', 'https://example.com')).toBe(false);
    });
  });
});