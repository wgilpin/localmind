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
 * Defines the structure for a bookmark entry with its URL and title.
 */
interface BookmarkEntry {
  url: string;
  title: string;
}

/**
 * Indexes a new URL using the RAG service.
 */
async function indexUrl(url: string, ragService: RagService, statusCallback: (status: string, message: string, data?: any) => void, title?: string): Promise<void> {
  console.log(`✅ Indexing new bookmark: ${url}`);
  statusCallback('info', `Indexing: ${url}`);
  try {
    // In a real scenario, you'd fetch content for the URL before adding.
    // For now, we'll just use the URL as content for demonstration.
    await ragService.addDocuments([{ title: title??url, content: url, url: url }]);
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
 * Recursively extracts URLs and their titles from a bookmark tree node.
 */
function extractBookmarksFromNode(node: BookmarkNode, bookmarks: BookmarkEntry[]): void {
  if (node.type === 'url' && node.url) {
    bookmarks.push({ url: node.url, title: node.name });
  }
  if (node.children) {
    for (const child of node.children) {
      extractBookmarksFromNode(child, bookmarks);
    }
  }
}

/**
 * Parses the Bookmarks file and returns an array of BookmarkEntry objects.
 */
function getAllBookmarks(filePath: string): BookmarkEntry[] {
  try {
    const content = fs.readFileSync(filePath, 'utf-8');
    const bookmarksJson = JSON.parse(content);
    const allBookmarks: BookmarkEntry[] = [];

    const roots = bookmarksJson.roots;
    if (roots) {
      for (const rootKey in roots) {
        extractBookmarksFromNode(roots[rootKey], allBookmarks);
      }
    }
    return allBookmarks;
  } catch (error) {
    console.error(`Could not read or parse bookmarks file: ${error}`);
    return [];
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

  let knownBookmarks = getAllBookmarks(bookmarksPath);
  console.log(`Monitoring initialized. Found ${knownBookmarks.length} bookmarks.`);

  // Initial scan and sync with the database
  statusCallback('info', 'Updating Bookmarks: Initial scan...');
  const existingDocuments = databaseService.getAllDocuments();
  const existingUrlsInDb = new Set(existingDocuments.map(doc => doc.url).filter(Boolean) as string[]);

  // Find bookmarks from file that are not in DB
  const bookmarksToAdd = knownBookmarks.filter(bookmark => !existingUrlsInDb.has(bookmark.url));
  for (const bookmark of bookmarksToAdd) {
    await indexUrl(bookmark.url, ragService, statusCallback, bookmark.title);
  }

  // Find bookmarks in DB that are no longer in file
  const currentUrls = new Set(knownBookmarks.map(b => b.url));
  const urlsToRemove = new Set([...existingUrlsInDb].filter(url => !currentUrls.has(url)));
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
        
        const currentBookmarks = getAllBookmarks(bookmarksPath);
        if (currentBookmarks.length === 0) {
          statusCallback('warn', 'Could not read current bookmarks or file is empty.');
          return; // Avoid processing on read error
        }

        const currentUrlsSet = new Set(currentBookmarks.map(b => b.url));
        const knownUrlsSet = new Set(knownBookmarks.map(b => b.url));

        // Find added bookmarks (in current but not in known)
        const newBookmarks = currentBookmarks.filter(bookmark => !knownUrlsSet.has(bookmark.url));
        for (const bookmark of newBookmarks) {
          await indexUrl(bookmark.url, ragService, statusCallback, bookmark.title);
        }

        // Find deleted bookmarks (in known but not in current)
        const deletedUrls = [...knownUrlsSet].filter(url => !currentUrlsSet.has(url));
        for (const url of deletedUrls) {
          await deindexUrl(url, ragService, databaseService, statusCallback);
        }
                
        // Update the state
        knownBookmarks = currentBookmarks;
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