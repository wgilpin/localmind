/**
 * @module RagServiceIntegrationTests
 * @description Integration tests for the RagService,
 * mocking its dependencies to ensure end-to-end functionality.
 */

import {
  RagService
} from './rag';
import {
  OllamaService
} from './ollama';
import {
  VectorStoreService
} from './vectorStore';
import {
  DatabaseService,
  Document
} from './database';

// Mock the low-level services
jest.mock('./ollama');
jest.mock('./vectorStore');
jest.mock('./database'); // Keep this global mock

describe('RagService (Integration Tests)', () => {
  let ragService: RagService;
  let mockOllamaService: jest.Mocked < OllamaService > ;
  let mockVectorStoreService: jest.Mocked < VectorStoreService > ;
  let mockDatabaseService: jest.Mocked < DatabaseService > ;

  beforeEach(async () => {
    // Clear all mocks before each test
    jest.clearAllMocks();

    // Initialize RagService with mocked dependencies
    mockOllamaService = new OllamaService() as jest.Mocked<OllamaService>;
    mockVectorStoreService = new VectorStoreService(
      'test-vector-store.faiss',
      {} as any, // Dummy DatabaseService, as it's mocked globally
      mockOllamaService
    ) as jest.Mocked<VectorStoreService>;

    // Initialize mockDatabaseService directly
    mockDatabaseService = new DatabaseService('test.db') as jest.Mocked<DatabaseService>;

    // Configure the transaction mock on the retrieved instance
    mockDatabaseService.transaction.mockImplementation((cb) => {
      const runTransaction = jest.fn(cb);
      return jest.fn(() => runTransaction());
    });

    // Mock other DatabaseService methods on the retrieved instance
    mockDatabaseService.getVectorMappingsByIds.mockReturnValue([]); // Default empty array for search test
    mockDatabaseService.getDocumentsByIds.mockReturnValue([]); // Default empty array for search test
    mockDatabaseService.deleteDocument.mockReturnValue(true); // Default for delete test
    mockDatabaseService.getVectorIdsByDocumentId.mockReturnValue([]); // Default for delete test
    
    // Mock the ntotal method for VectorStoreService to return increasing values
    let ntotalCount = 0;
    mockVectorStoreService.ntotal.mockImplementation(() => {
        const currentTotal = ntotalCount;
        return currentTotal;
    });
    mockVectorStoreService.add.mockImplementation((embeddings: number[][]) => {
        ntotalCount += embeddings.length;
    });

    ragService = new RagService(
      mockOllamaService,
      mockVectorStoreService,
      mockDatabaseService
    );
  });

  describe('addDocuments', () => {
    it('should correctly call add methods on database and vector stores and getEmbeddings on Ollama service', async () => {
      const docsToAdd = [{
        content: 'test content 1',
        url: 'http://example.com/doc1',
        title: 'Test Document 1',
      }, {
        content: 'test content 2',
        url: 'http://example.com/doc2',
        title: 'Test Document 2',
      }, ];
      
      const embeddingsDoc1 = [
        [0.1, 0.2, 0.3], [0.11, 0.21, 0.31]
      ];
      const embeddingsDoc2 = [
        [0.4, 0.5, 0.6], [0.41, 0.51, 0.61]
      ];

      mockOllamaService.getEmbeddings
        .mockResolvedValueOnce(embeddingsDoc1)
        .mockResolvedValueOnce(embeddingsDoc2);
      
      await ragService.addDocuments(docsToAdd);

      expect(mockOllamaService.getEmbeddings).toHaveBeenCalledTimes(2);
      expect(mockOllamaService.getEmbeddings).toHaveBeenCalledWith(['test content 1']);
      expect(mockOllamaService.getEmbeddings).toHaveBeenCalledWith(['test content 2']);

      expect(mockDatabaseService.insertDocument).toHaveBeenCalledTimes(2);
      expect(mockDatabaseService.insertDocument).toHaveBeenCalledWith(expect.objectContaining({
        content: 'test content 1'
      }));
      expect(mockDatabaseService.insertDocument).toHaveBeenCalledWith(expect.objectContaining({
        content: 'test content 2'
      }));

      expect(mockVectorStoreService.add).toHaveBeenCalledWith([...embeddingsDoc1, ...embeddingsDoc2]);
      expect(mockDatabaseService.insertVectorMappings).toHaveBeenCalledWith([
        { vectorId: 0, documentId: expect.any(String) },
        { vectorId: 1, documentId: expect.any(String) },
        { vectorId: 2, documentId: expect.any(String) },
        { vectorId: 3, documentId: expect.any(String) }
      ]);
    });
  });

  describe('search', () => {
    it('should correctly execute the RAG pipeline with diversity ranking', async () => {
      const query = 'test query';
      const queryEmbedding = [0.1, 0.2, 0.3];
      const searchResults = {
        I: [0, 1, 2, 3, 4],
        D: [0.1, 0.2, 0.15, 0.3, 0.25]
      };
      const vectorMappings = [{
        vectorId: 0,
        documentId: 'doc-A'
      }, {
        vectorId: 1,
        documentId: 'doc-A'
      }, {
        vectorId: 2,
        documentId: 'doc-B'
      }, {
        vectorId: 3,
        documentId: 'doc-B'
      }, {
        vectorId: 4,
        documentId: 'doc-C'
      }, ];
      const documents = [{
        id: 'doc-A',
        content: 'content A',
        title: 'Doc A',
        url: '',
        timestamp: 0
      }, {
        id: 'doc-B',
        content: 'content B',
        title: 'Doc B',
        url: '',
        timestamp: 0
      }, {
        id: 'doc-C',
        content: 'content C',
        title: 'Doc C',
        url: '',
        timestamp: 0
      }, ];
      const completion = 'generated completion';

      mockOllamaService.getEmbedding.mockResolvedValue(queryEmbedding);
      mockVectorStoreService.search.mockResolvedValue(searchResults);
      mockDatabaseService.getVectorMappingsByIds.mockReturnValue(vectorMappings);
      mockDatabaseService.getDocumentsByIds.mockReturnValue(documents);
      mockOllamaService.getCompletion.mockResolvedValue(completion);

      const result = await ragService.search(query);

      expect(mockOllamaService.getEmbedding).toHaveBeenCalledWith(query);
      expect(mockVectorStoreService.search).toHaveBeenCalledWith(queryEmbedding, 100);
      expect(mockDatabaseService.getVectorMappingsByIds).toHaveBeenCalled();
      expect(mockDatabaseService.getDocumentsByIds).toHaveBeenCalled();

      const receivedPrompt = mockOllamaService.getCompletion.mock.calls[0][0];
      expect(receivedPrompt).toContain(query);
      expect(receivedPrompt).toContain('content A');
      expect(receivedPrompt).toContain('content B');
      expect(receivedPrompt).toContain('content C');
      expect(result).toEqual({ response: completion, documents: expect.any(Array) });
    });

    it('should return a default message if no relevant documents are found', async () => {
      const query = 'test query';
      const queryEmbedding = [0.4, 0.5, 0.6];

      mockOllamaService.getEmbedding.mockResolvedValue(queryEmbedding);
      mockVectorStoreService.search.mockResolvedValue({
        I: [],
        D: []
      });

      const result = await ragService.search(query);

      expect(mockOllamaService.getEmbedding).toHaveBeenCalledWith(query);
      expect(mockVectorStoreService.search).toHaveBeenCalledWith(queryEmbedding, 100);
      expect(mockDatabaseService.getVectorMappingsByIds).not.toHaveBeenCalled();
      expect(mockDatabaseService.getDocumentsByIds).not.toHaveBeenCalled();
      expect(mockOllamaService.getCompletion).not.toHaveBeenCalled();
      expect(result).toEqual({ response: 'No relevant documents found.', documents: [] });
    });
  });

  describe('deleteDocument', () => {
    it('should delete a document and its associated vector entries', async () => {
      const documentIdToDelete = 'test-doc-id-123';
      const mockVectorIds = [100, 101];

      mockDatabaseService.getVectorIdsByDocumentId.mockReturnValue(mockVectorIds);
      mockDatabaseService.deleteDocument.mockReturnValue(true);
      mockVectorStoreService.deleteVector.mockImplementation(() => {});
      mockVectorStoreService.save.mockResolvedValue(undefined);

      const result = await ragService.deleteDocument(documentIdToDelete);

      expect(mockDatabaseService.deleteDocument).toHaveBeenCalledWith(documentIdToDelete);
      expect(mockDatabaseService.getVectorIdsByDocumentId).toHaveBeenCalledWith(documentIdToDelete);
      expect(mockVectorStoreService.deleteVector).toHaveBeenCalledWith(mockVectorIds);
      expect(mockVectorStoreService.save).toHaveBeenCalled();
      expect(result).toBe(true);
    });

    it('should return false if the document is not found in the database', async () => {
      const documentIdToDelete = 'non-existent-doc-id';

      mockDatabaseService.deleteDocument.mockReturnValue(false);
      mockDatabaseService.getVectorIdsByDocumentId.mockReturnValue([]);

      const result = await ragService.deleteDocument(documentIdToDelete);

      expect(mockDatabaseService.deleteDocument).toHaveBeenCalledWith(documentIdToDelete);
      expect(mockVectorStoreService.deleteVector).not.toHaveBeenCalled();
      expect(mockVectorStoreService.save).not.toHaveBeenCalled();
      expect(result).toBe(false);
    });
  });

});