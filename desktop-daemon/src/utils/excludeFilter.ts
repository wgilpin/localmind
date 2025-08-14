import { OllamaConfig } from '../config';

/**
 * Checks if a URL should be excluded from indexing based on the exclude folders configuration.
 * @param url The URL to check
 * @returns true if the URL should be excluded, false otherwise
 */
export function shouldExcludeUrl(url: string): boolean {
  if (!url || !OllamaConfig.excludeFolders) {
    return false;
  }

  try {
    const urlObj = new URL(url);
    const pathname = urlObj.pathname.toLowerCase();
    
    // Check if any exclude folder pattern matches the URL path
    return OllamaConfig.excludeFolders.some(folder => {
      const folderPattern = folder.toLowerCase();
      
      // Check if the path contains the folder name
      // This handles both direct folder matches and nested paths
      return pathname.includes(`/${folderPattern}/`) || 
             pathname.includes(`/${folderPattern}`) ||
             pathname.endsWith(`/${folderPattern}`) ||
             pathname.includes(`${folderPattern}/`);
    });
  } catch (error) {
    // If URL parsing fails, don't exclude it
    console.warn(`Failed to parse URL for exclusion check: ${url}`, error);
    return false;
  }
}

/**
 * Checks if a bookmark title or URL contains any excluded folder names.
 * This is useful for bookmark titles that might indicate repository or project paths.
 * @param title The bookmark title to check
 * @param url The bookmark URL to check
 * @returns true if the bookmark should be excluded, false otherwise
 */
export function shouldExcludeBookmark(title: string, url: string): boolean {
  // First check the URL
  if (shouldExcludeUrl(url)) {
    return true;
  }

  // Then check if the title contains any exclude patterns
  if (!title || !OllamaConfig.excludeFolders) {
    return false;
  }

  const lowerTitle = title.toLowerCase();
  return OllamaConfig.excludeFolders.some(folder => {
    const folderPattern = folder.toLowerCase();
    return lowerTitle.includes(folderPattern);
  });
}