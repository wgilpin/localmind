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

  describe('searchAndStream', () => {
    it('should correctly execute the RAG pipeline with diversity ranking and stream response', async () => {
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
      const completionChunks = ['generated', ' ', 'completion'];

      mockOllamaService.getEmbedding.mockResolvedValue(queryEmbedding);
      mockVectorStoreService.search.mockResolvedValue(searchResults);
      mockDatabaseService.getVectorMappingsByIds.mockReturnValue(vectorMappings);
      mockDatabaseService.getDocumentsByIds.mockReturnValue(documents);
      
      // Mock getCompletionStream to return an async iterator
      mockOllamaService.getCompletionStream.mockImplementation(async function*() {
        for (const chunk of completionChunks) {
          yield chunk;
        }
      });

      const onProgressMock = jest.fn();
      await ragService.searchAndStream(query, onProgressMock);

      expect(mockOllamaService.getEmbedding).toHaveBeenCalledWith(query);
      expect(mockVectorStoreService.search).toHaveBeenCalledWith(queryEmbedding, 100);
      expect(mockDatabaseService.getVectorMappingsByIds).toHaveBeenCalled();
      expect(mockDatabaseService.getDocumentsByIds).toHaveBeenCalled();
      expect(mockOllamaService.getCompletionStream).toHaveBeenCalled();

      // Verify progress callbacks
      expect(onProgressMock).toHaveBeenCalledWith('starting', 'Starting search...');
      expect(onProgressMock).toHaveBeenCalledWith('retrieving', 'Retrieving relevant documents...');
      expect(onProgressMock).toHaveBeenCalledWith('retrieving', 'Retrieved documents.', expect.objectContaining({ documents: expect.any(Array) }));
      expect(onProgressMock).toHaveBeenCalledWith('generating', 'Building response...');
      expect(onProgressMock).toHaveBeenCalledWith('generating', 'Streaming response...', { chunk: 'generated' });
      expect(onProgressMock).toHaveBeenCalledWith('generating', 'Streaming response...', { chunk: ' ' });
      expect(onProgressMock).toHaveBeenCalledWith('generating', 'Streaming response...', { chunk: 'completion' });
      expect(onProgressMock).toHaveBeenCalledWith('complete', 'Search complete.', { response: completionChunks.join('') });
    });

    it('should call onProgress with "complete" and "No relevant documents found" if no relevant documents are found', async () => {
      const query = 'test query';
      const queryEmbedding = [0.4, 0.5, 0.6];

      mockOllamaService.getEmbedding.mockResolvedValue(queryEmbedding);
      mockVectorStoreService.search.mockResolvedValue({
        I: [],
        D: []
      });

      const onProgressMock = jest.fn();
      await ragService.searchAndStream(query, onProgressMock);

      expect(mockOllamaService.getEmbedding).toHaveBeenCalledWith(query);
      expect(mockVectorStoreService.search).toHaveBeenCalledWith(queryEmbedding, 100);
      expect(mockDatabaseService.getVectorMappingsByIds).not.toHaveBeenCalled();
      expect(mockDatabaseService.getDocumentsByIds).not.toHaveBeenCalled();
      expect(mockOllamaService.getCompletionStream).not.toHaveBeenCalled();
      
      expect(onProgressMock).toHaveBeenCalledWith('starting', 'Starting search...');
      expect(onProgressMock).toHaveBeenCalledWith('retrieving', 'Retrieving relevant documents...');
      expect(onProgressMock).toHaveBeenCalledWith('complete', 'No relevant documents found');
    });

    it('should call onProgress with "error" if an error occurs during searchAndStream', async () => {
      const query = 'test query';
      const error = new Error('Test error');

      mockOllamaService.getEmbedding.mockRejectedValue(error);

      const onProgressMock = jest.fn();
      await ragService.searchAndStream(query, onProgressMock);

      expect(onProgressMock).toHaveBeenCalledWith('starting', 'Starting search...');
      expect(onProgressMock).toHaveBeenCalledWith('error', 'Search failed');
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