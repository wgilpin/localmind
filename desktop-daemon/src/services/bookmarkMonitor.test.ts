import { jest } from '@jest/globals';
import fs from 'fs';
import path from 'path';
import os from 'os';
import { startBookmarkMonitor } from './bookmarkMonitor';
import { RagService } from './rag';
import { DatabaseService, Document } from './database'; // Corrected import path for Document

// Mock fs module
jest.mock('fs', () => ({
  watch: jest.fn(),
  readFileSync: jest.fn(),
  existsSync: jest.fn(),
}));

describe('bookmarkMonitor', () => {
  let mockRagService: jest.Mocked<RagService>;
  let mockDatabaseService: jest.Mocked<DatabaseService>;
  let mockStatusCallback: jest.Mock;
  const mockBookmarksPath = path.join(os.homedir(), 'AppData', 'Local', 'Google', 'Chrome', 'User Data', 'Default', 'Bookmarks');

  beforeEach(() => {
    mockRagService = {
      addDocuments: jest.fn(),
      deleteDocument: jest.fn(),
    } as any;
    mockDatabaseService = {
      getAllDocuments: jest.fn(),
      getDocumentByUrl: jest.fn(),
    } as any;
    mockStatusCallback = jest.fn();

    // Reset mocks before each test
    (fs.watch as jest.Mock).mockClear();
    (fs.readFileSync as jest.Mock).mockClear();
    (fs.existsSync as jest.Mock).mockClear();
    mockRagService.addDocuments.mockClear();
    mockRagService.deleteDocument.mockClear();
    mockDatabaseService.getAllDocuments.mockClear();
    mockDatabaseService.getDocumentByUrl.mockClear();
    mockStatusCallback.mockClear();

    // Default mock for existsSync
    (fs.existsSync as jest.Mock).mockReturnValue(true);
  });

  it('should index new bookmarks on initial scan with correct titles', async () => {
    const mockBookmarkJson = {
      roots: {
        bookmark_bar: {
          children: [
            { type: 'url', name: 'Test Bookmark 1', url: 'http://example.com/1' },
            { type: 'url', name: 'Test Bookmark 2', url: 'http://example.com/2' },
          ],
        },
      },
    };

    (fs.readFileSync as jest.Mock).mockReturnValue(JSON.stringify(mockBookmarkJson));
    mockDatabaseService.getAllDocuments.mockReturnValue([]); // No existing documents in DB

    // Start the monitor
    startBookmarkMonitor(mockRagService, mockDatabaseService, mockStatusCallback);

    // Give some time for the initial scan to complete (it's async)
    await new Promise(process.nextTick); // Simulates a microtask queue flush

    expect(mockRagService.addDocuments).toHaveBeenCalledTimes(2);
    expect(mockRagService.addDocuments).toHaveBeenCalledWith([
      { title: 'Test Bookmark 1', content: 'http://example.com/1', url: 'http://example.com/1' },
    ]);
    expect(mockRagService.addDocuments).toHaveBeenCalledWith([
      { title: 'Test Bookmark 2', content: 'http://example.com/2', url: 'http://example.com/2' },
    ]);
    expect(mockStatusCallback).toHaveBeenCalledWith('info', 'Updating Bookmarks: Initial scan...');
    expect(mockStatusCallback).toHaveBeenCalledWith('info', 'Updating Bookmarks: Initial scan complete.');
  });

  it('should de-index deleted bookmarks', async () => {
    const initialBookmarkJson = {
      roots: {
        bookmark_bar: {
          children: [
            { type: 'url', name: 'Test Bookmark 1', url: 'http://example.com/1' },
            { type: 'url', name: 'Test Bookmark 2', url: 'http://example.com/2' },
          ],
        },
      },
    };

    const updatedBookmarkJson = {
      roots: {
        bookmark_bar: {
          children: [
            { type: 'url', name: 'Test Bookmark 1', url: 'http://example.com/1' },
          ],
        },
      },
    };

    // Mock initial state
    (fs.readFileSync as jest.Mock)
      .mockReturnValueOnce(JSON.stringify(initialBookmarkJson))
      .mockReturnValueOnce(JSON.stringify(updatedBookmarkJson)); // For the watch callback

    mockDatabaseService.getAllDocuments.mockReturnValue([
      { id: 'doc1', url: 'http://example.com/1', content: '', title: 'Test Bookmark 1', timestamp: Date.now() },
      { id: 'doc2', url: 'http://example.com/2', content: '', title: 'Test Bookmark 2', timestamp: Date.now() },
    ]);
    mockDatabaseService.getDocumentByUrl.mockImplementation((url) => {
      if (url === 'http://example.com/2') {
        return { id: 'doc2', url: 'http://example.com/2', content: '', title: 'Test Bookmark 2', timestamp: Date.now() };
      }
      return undefined;
    });

    startBookmarkMonitor(mockRagService, mockDatabaseService, mockStatusCallback);
    await new Promise(process.nextTick); // Initial scan

    // Simulate file change
    const watchCallback = (fs.watch as jest.Mock).mock.calls[0][1] as (eventType: string, filename: string) => void;
    watchCallback('change', path.basename(mockBookmarksPath));

    // Wait for the debounce and processing
    await new Promise((resolve) => setTimeout(resolve, 550)); // Debounce + a bit

    expect(mockRagService.deleteDocument).toHaveBeenCalledTimes(1);
    expect(mockRagService.deleteDocument).toHaveBeenCalledWith('doc2');
    expect(mockStatusCallback).toHaveBeenCalledWith('info', 'Bookmarks file changed. Checking for updates...');
    expect(mockStatusCallback).toHaveBeenCalledWith('info', 'Bookmark sync complete.');
  });

  it('should not re-index existing bookmarks', async () => {
    const mockBookmarkJson = {
      roots: {
        bookmark_bar: {
          children: [
            { type: 'url', name: 'Test Bookmark 1', url: 'http://example.com/1' },
          ],
        },
      },
    };

    (fs.readFileSync as jest.Mock).mockReturnValue(JSON.stringify(mockBookmarkJson));
    mockDatabaseService.getAllDocuments.mockReturnValue([
      { id: 'doc1', url: 'http://example.com/1', content: '', title: 'Test Bookmark 1', timestamp: Date.now() },
    ]);

    startBookmarkMonitor(mockRagService, mockDatabaseService, mockStatusCallback);
    await new Promise(process.nextTick);

    expect(mockRagService.addDocuments).not.toHaveBeenCalled();
  });

  it('should handle adding a new bookmark after initial scan', async () => {
    const initialBookmarkJson = {
      roots: {
        bookmark_bar: {
          children: [
            { type: 'url', name: 'Test Bookmark 1', url: 'http://example.com/1' },
          ],
        },
      },
    };

    const newBookmarkJson = {
      roots: {
        bookmark_bar: {
          children: [
            { type: 'url', name: 'Test Bookmark 1', url: 'http://example.com/1' },
            { type: 'url', name: 'New Bookmark', url: 'http://example.com/new' },
          ],
        },
      },
    };

    (fs.readFileSync as jest.Mock)
      .mockReturnValueOnce(JSON.stringify(initialBookmarkJson))
      .mockReturnValueOnce(JSON.stringify(newBookmarkJson));

    mockDatabaseService.getAllDocuments.mockReturnValue([
      { id: 'doc1', url: 'http://example.com/1', content: '', title: 'Test Bookmark 1', timestamp: Date.now() },
    ]);

    startBookmarkMonitor(mockRagService, mockDatabaseService, mockStatusCallback);
    await new Promise(process.nextTick); // Initial scan

    mockRagService.addDocuments.mockClear(); // Clear calls from initial scan

    const watchCallback = (fs.watch as jest.Mock).mock.calls[0][1] as (eventType: string, filename: string) => void;
    watchCallback('change', path.basename(mockBookmarksPath));

    await new Promise((resolve) => setTimeout(resolve, 550));

    expect(mockRagService.addDocuments).toHaveBeenCalledTimes(1);
    expect(mockRagService.addDocuments).toHaveBeenCalledWith([
      { title: 'New Bookmark', content: 'http://example.com/new', url: 'http://example.com/new' },
    ]);
  });
});