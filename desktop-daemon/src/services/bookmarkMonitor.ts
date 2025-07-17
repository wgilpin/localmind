import fs from 'fs';
import path from 'path';
import os from 'os';
import { RagService } from './rag';
import { DatabaseService } from './database';

/**
 * Type definitions for bookmarks.
 */
interface BookmarkNode {
  type: 'url' | 'folder';
  url?: string;
  name: string;
  children?: BookmarkNode[];
}

/**
 * Indexes a new URL using the RAG service.
 */
async function indexUrl(url: string, ragService: RagService, statusCallback: (status: string, message: string, data?: any) => void): Promise<void> {
  console.log(`✅ Indexing new bookmark: ${url}`);
  statusCallback('info', `Indexing: ${url}`);
  try {
    // In a real scenario, you'd fetch content for the URL before adding.
    // For now, we'll just use the URL as content for demonstration.
    await ragService.addDocuments([{ title: url, content: url, url: url }]);
  } catch (error: unknown) {
    console.error(`Error indexing URL ${url}:`, error);
    statusCallback('error', `Failed to index: ${url}`, { error: (error as Error).message });
  }
}

/**
 * De-indexes a deleted URL from the RAG service.
 */
async function deindexUrl(url: string, ragService: RagService, databaseService: DatabaseService, statusCallback: (status: string, message: string, data?: any) => void): Promise<void> {
  console.log(`❌ De-indexing deleted bookmark: ${url}`);
  statusCallback('info', `De-indexing: ${url}`);
  try {
    const document = databaseService.getDocumentByUrl(url);
    if (document) {
      await ragService.deleteDocument(document.id);
    } else {
      console.warn(`Attempted to de-index URL not found in DB: ${url}`);
    }
  } catch (error: unknown) {
    console.error(`Error de-indexing URL ${url}:`, error);
    statusCallback('error', `Failed to de-index: ${url}`, { error: (error as Error).message });
  }
}

/**
 * Constructs the platform-agnostic path to the Chrome Bookmarks file.
 */
function getBookmarksPath(): string {
  // This logic is for Windows. Adapt for macOS/Linux if needed.
  // macOS: 'Library/Application Support/Google/Chrome/Default/Bookmarks'
  // Linux: '.config/google-chrome/Default/Bookmarks'
  return path.join(
    os.homedir(),
    'AppData',
    'Local',
    'Google',
    'Chrome',
    'User Data',
    'Default',
    'Bookmarks',
  );
}

/**
 * Recursively extracts URLs from a bookmark tree node.
 */
function extractUrlsFromNode(node: BookmarkNode, urls: Set<string>): void {
  if (node.type === 'url' && node.url) {
    urls.add(node.url);
  }
  if (node.children) {
    for (const child of node.children) {
      extractUrlsFromNode(child, urls);
    }
  }
}

/**
 * Parses the Bookmarks file and returns a set of all URLs.
 */
function getAllBookmarkUrls(filePath: string): Set<string> {
  try {
    const content = fs.readFileSync(filePath, 'utf-8');
    const bookmarksJson = JSON.parse(content);
    const allUrls = new Set<string>();

    const roots = bookmarksJson.roots;
    if (roots) {
      for (const rootKey in roots) {
        extractUrlsFromNode(roots[rootKey], allUrls);
      }
    }
    return allUrls;
  } catch (error) {
    console.error(`Could not read or parse bookmarks file: ${error}`);
    return new Set();
  }
}

/**
 * The main monitoring function.
 */
async function monitorBookmarks(ragService: RagService, databaseService: DatabaseService, statusCallback: (status: string, message: string, data?: any) => void): Promise<void> {
  const bookmarksPath = getBookmarksPath();
  if (!fs.existsSync(bookmarksPath)) {
    console.error('Error: Chrome bookmarks file not found.');
    return;
  }

  let knownUrls = getAllBookmarkUrls(bookmarksPath);
  console.log(`Monitoring initialized. Found ${knownUrls.size} bookmarks.`);

  // Initial scan and sync with the database
  statusCallback('info', 'Updating Bookmarks: Initial scan...');
  const existingDocuments = databaseService.getAllDocuments();
  const existingUrlsInDb = new Set(existingDocuments.map(doc => doc.url).filter(Boolean) as string[]);

  // Find bookmarks from file that are not in DB
  const urlsToAdd = new Set([...knownUrls].filter(url => !existingUrlsInDb.has(url)));
  for (const url of urlsToAdd) {
    await indexUrl(url, ragService, statusCallback);
  }

  // Find bookmarks in DB that are no longer in file
  const urlsToRemove = new Set([...existingUrlsInDb].filter(url => !knownUrls.has(url)));
  for (const url of urlsToRemove) {
    await deindexUrl(url, ragService, databaseService, statusCallback);
  }
  statusCallback('info', 'Updating Bookmarks: Initial scan complete.');


  // Watch the directory containing the file for more reliable events.
  const watchDirectory = path.dirname(bookmarksPath);
  const targetFilename = path.basename(bookmarksPath);
  let debounceTimer: NodeJS.Timeout;

  fs.watch(watchDirectory, async (eventType, filename) => { // Made callback async
    if (filename === targetFilename) {
      // Debounce to handle multiple rapid-fire events from a single save.
      clearTimeout(debounceTimer);
      debounceTimer = setTimeout(async () => {
        console.log('\nBookmarks file changed. Checking for updates...');
        statusCallback('info', 'Bookmarks file changed. Checking for updates...');
        
        const currentUrls = getAllBookmarkUrls(bookmarksPath);
        if (currentUrls.size === 0) {
          statusCallback('warn', 'Could not read current bookmarks or file is empty.');
          return; // Avoid processing on read error
        }

        // Find added bookmarks (in current but not in known)
        const newUrls = new Set([...currentUrls].filter(url => !knownUrls.has(url)));
        for (const url of newUrls) {
          await indexUrl(url, ragService, statusCallback);
        }

        // Find deleted bookmarks (in known but not in current)
        const deletedUrls = new Set([...knownUrls].filter(url => !currentUrls.has(url)));
        for (const url of deletedUrls) {
          await deindexUrl(url, ragService, databaseService, statusCallback);
        }
                
        // Update the state
        knownUrls = currentUrls;
        statusCallback('info', 'Bookmark sync complete.');
      }, 500); // 500ms debounce window
    }
  });

  console.log(`🚀 Starting bookmark monitor for: ${bookmarksPath}`);
}

/**
 * Initializes and starts the bookmark monitoring process.
 * @param ragService The RAG service instance.
 * @param databaseService The Database service instance.
 * @param statusCallback Callback to report status updates to the UI.
 */
export function startBookmarkMonitor(ragService: RagService, databaseService: DatabaseService, statusCallback: (status: string, message: string, data?: any) => void): void {
  monitorBookmarks(ragService, databaseService, statusCallback);
  // Keep the process alive. In a real app, this would be part of a larger server/service.
  process.on('SIGINT', () => {
      console.log("\n🛑 Monitor stopped by user.");
      process.exit();
  });
}