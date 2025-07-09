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
jest.mock('./database');

describe('RagService (Integration Tests)', () => {
  let ragService: RagService;
  let mockOllamaService: jest.Mocked < OllamaService > ;
  let mockVectorStoreService: jest.Mocked < VectorStoreService > ;
  let mockDatabaseService: jest.Mocked < DatabaseService > ;

  beforeEach(async () => {
    // Clear all mocks before each test
    jest.clearAllMocks();

    // Initialize RagService with mocked dependencies
    mockOllamaService = new OllamaService() as jest.Mocked < OllamaService > ;
    mockVectorStoreService = new VectorStoreService('test-vector-store.faiss') as jest.Mocked < VectorStoreService > ;
    
    // Mock the ntotal method for VectorStoreService
    mockVectorStoreService.ntotal.mockReturnValue(0);

    // First, create the ragService, which will instantiate the mocked DatabaseService
    ragService = await RagService.create(
      mockOllamaService,
      mockVectorStoreService
    );
    
    // Now, get the instance of DatabaseService that was created by RagService.create
    mockDatabaseService = (DatabaseService as jest.Mock).mock.instances[0] as jest.Mocked<DatabaseService>;

    // Configure the transaction mock
    mockDatabaseService.transaction.mockImplementation((cb) => {
      const runTransaction = jest.fn(cb);
      const mockTransactionWrapper = (() => runTransaction()) as any;
      mockTransactionWrapper.default = runTransaction;
      mockTransactionWrapper.deferred = runTransaction;
      mockTransactionWrapper.immediate = runTransaction;
      mockTransactionWrapper.exclusive = runTransaction;
      return mockTransactionWrapper;
    });
  });

  describe('addDocuments', () => {
    it('should correctly call add methods on database and vector stores and getEmbeddings on Ollama service', async () => {
      // Define the documents to be added
      const docsToAdd = [{
        content: 'test content 1',
        url: 'http://example.com/doc1',
        title: 'Test Document 1',
      }, {
        content: 'test content 2',
        url: 'http://example.com/doc2',
        title: 'Test Document 2',
      }, ];
      
      const embeddings = [
        [0.1, 0.2, 0.3],
        [0.4, 0.5, 0.6]
      ];

      mockOllamaService.getEmbeddings.mockResolvedValueOnce(embeddings.slice(0, 1)).mockResolvedValueOnce(embeddings.slice(1, 2));
      mockVectorStoreService.add.mockImplementation(() => {});

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

      expect(mockVectorStoreService.add).toHaveBeenCalledWith(embeddings);
      expect(mockDatabaseService.insertVectorMappings).toHaveBeenCalledWith([{
        vectorId: 0,
        documentId: expect.any(String)
      }, {
        vectorId: 1,
        documentId: expect.any(String)
      }]);
    });
  });

  describe('search', () => {
    it('should correctly execute the RAG pipeline for a given query', async () => {
      const query = 'test query';
      const queryEmbedding = [0.4, 0.5, 0.6];
      const searchResults = {
        I: [0, 1],
        D: [0.9, 0.8]
      };
      const documents: Document[] = [{
        id: 'doc0-id',
        content: 'content of doc1',
        url: 'http://example.com/doc0',
        title: 'Doc 0',
        timestamp: Date.now()
      }, {
        id: 'doc1-id',
        content: 'content of doc2',
        url: 'http://example.com/doc1',
        title: 'Doc 1',
        timestamp: Date.now()
      }, ];
      const completion = 'generated completion';

      mockOllamaService.getEmbedding.mockResolvedValue(queryEmbedding);
      mockVectorStoreService.search.mockResolvedValue(searchResults);
      mockDatabaseService.getDocumentIdsByVectorIds.mockReturnValue(['doc0-id', 'doc1-id']);
      mockDatabaseService.getDocumentsByIds.mockReturnValue(documents);
      mockOllamaService.getCompletion.mockResolvedValue(completion);

      const result = await ragService.search(query);

      expect(mockOllamaService.getEmbedding).toHaveBeenCalledWith(query);
      expect(mockVectorStoreService.search).toHaveBeenCalledWith(queryEmbedding, 5);
      expect(mockDatabaseService.getDocumentIdsByVectorIds).toHaveBeenCalledWith([0, 1]);
      expect(mockDatabaseService.getDocumentsByIds).toHaveBeenCalledWith(['doc0-id', 'doc1-id']);

      const expectedPrompt = `
        You are a helpful AI assistant.
        Answer the following question based on the provided context:

        Question: ${query}

        Context:
        content of doc1

content of doc2

        Instructions:
        Be concise.
        Do not refer to the context or the provided information .
        Constrain your answers very strongly to the provided material and if you do need to refer to you your built-in knowledge tell the user where you have done so.
        `.trim().replace(/ {2,}/g, ' ');

      expect(mockOllamaService.getCompletion).toHaveBeenCalledWith(expectedPrompt);
      expect(result).toBe(completion);
    });

    it('should return a default message if no relevant documents are found', async () => {
      const query = 'test query';
      const queryEmbedding = [0.4, 0.5, 0.6];

      mockOllamaService.getEmbedding.mockResolvedValue(queryEmbedding);
      mockVectorStoreService.search.mockResolvedValue({
        I: [],
        D: []
      });
      mockDatabaseService.getDocumentsByIds.mockReturnValue([]);

      const result = await ragService.search(query);

      expect(mockOllamaService.getEmbedding).toHaveBeenCalledWith(query);
      expect(mockVectorStoreService.search).toHaveBeenCalledWith(queryEmbedding, 5);
      expect(mockDatabaseService.getDocumentIdsByVectorIds).not.toHaveBeenCalled();
      expect(mockDatabaseService.getDocumentsByIds).not.toHaveBeenCalled();
      expect(mockOllamaService.getCompletion).not.toHaveBeenCalled();
      expect(result).toBe('No documents available in the knowledge base. Please add some documents first.');
    });
  });
});