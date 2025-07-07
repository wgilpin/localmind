import { DocumentStoreService } from './documentStore';
import * as fsPromises from 'fs/promises';
import * as path from 'path';

jest.mock('fs/promises', () => ({
  readFile: jest.fn(),
  writeFile: jest.fn(),
  mkdir: jest.fn(() => Promise.resolve()),
}));

describe('DocumentStoreService', () => {
  let documentStore: DocumentStoreService;
  const filePath = 'test-documents.json';

  beforeEach(() => {
    documentStore = new DocumentStoreService(filePath);
    jest.clearAllMocks();
  });

  describe('add and get', () => {
    it('should add a document and retrieve it by ID', async () => {
      const newDocument = {
        content: 'This is the content of document 1.',
        url: 'http://example.com/doc1',
        title: 'Document 1',
        timestamp: Date.now(),
      };
      const addedDocument = await documentStore.add(newDocument);
      expect(await documentStore.get(addedDocument.id)).toEqual(expect.objectContaining(newDocument));
    });

    it('should return undefined for a non-existent document', async () => {
      expect(await documentStore.get('nonExistentDoc')).toBeUndefined();
    });
  });

  describe('save', () => {
    it('should attempt to write to the file system with the correct data', async () => {
      const docContent1 = 'Content 1.';
      const docContent2 = 'Content 2.';

      const newDoc1 = { content: docContent1, url: 'url1', title: 'title1', timestamp: Date.now() };
      const newDoc2 = { content: docContent2, url: 'url2', title: 'title2', timestamp: Date.now() };

      const addedDoc1 = await documentStore.add(newDoc1);
      const addedDoc2 = await documentStore.add(newDoc2);

      await documentStore.save();

      const expectedData = JSON.stringify([
        { id: addedDoc1.id, ...newDoc1 },
        { id: addedDoc2.id, ...newDoc2 },
      ], null, 2);

      expect(fsPromises.writeFile).toHaveBeenCalledWith(filePath, expectedData, 'utf8');
    });
  });

  describe('load', () => {
    it('should correctly parse data from the mocked file system', async () => {
      const mockData = [
        { id: 'docA', content: 'Mocked content A.', url: 'urlA', title: 'titleA', timestamp: 123 },
        { id: 'docB', content: 'Mocked content B.', url: 'urlB', title: 'titleB', timestamp: 456 },
      ];
      (fsPromises.readFile as jest.Mock).mockResolvedValueOnce(JSON.stringify(mockData));

      await documentStore.load();

      expect(await documentStore.get('docA')).toEqual(expect.objectContaining({ content: 'Mocked content A.' }));
      expect(await documentStore.get('docB')).toEqual(expect.objectContaining({ content: 'Mocked content B.' }));
    });

    it('should handle file not found gracefully', async () => {
      (fsPromises.readFile as jest.Mock).mockRejectedValueOnce({ code: 'ENOENT' });
      await expect(documentStore.load()).resolves.not.toThrow();
      expect(await documentStore.get('anyDoc')).toBeUndefined();
      expect(fsPromises.writeFile).toHaveBeenCalledWith(filePath, '[]', 'utf8');
    });

    it('should handle invalid JSON gracefully', async () => {
      (fsPromises.readFile as jest.Mock).mockResolvedValueOnce('invalid json');
      // Expect the promise to reject when parsing invalid JSON
      await expect(documentStore.load()).rejects.toThrow(SyntaxError);
      expect(await documentStore.get('anyDoc')).toBeUndefined();
    });
  });
});