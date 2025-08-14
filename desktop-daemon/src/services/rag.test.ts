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
  ChromaStoreService
} from './chromaStore';
import {
  DatabaseService,
  Document
} from './database';

// Mock the low-level services
jest.mock('./ollama');
jest.mock('./chromaStore');
jest.mock('./database'); // Keep this global mock

describe('RagService (Integration Tests)', () => {
  let ragService: RagService;
  let mockOllamaService: jest.Mocked < OllamaService > ;
  let mockVectorStoreService: jest.Mocked < ChromaStoreService > ;
  let mockDatabaseService: jest.Mocked < DatabaseService > ;

  beforeEach(async () => {
    // Clear all mocks before each test
    jest.clearAllMocks();

    // Initialize RagService with mocked dependencies
    mockOllamaService = new OllamaService() as jest.Mocked<OllamaService>;
    mockVectorStoreService = new ChromaStoreService(
      'test-chromadb',
      {} as any, // Dummy DatabaseService, as it's mocked globally
      mockOllamaService
    ) as jest.Mocked<ChromaStoreService>;

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
    
    // Mock the ntotal method for ChromaStoreService to return increasing values
    let ntotalCount = 0;
    mockVectorStoreService.ntotal.mockImplementation(() => {
        const currentTotal = ntotalCount;
        return currentTotal;
    });
    // Mock the ChromaDB-specific methods
    mockVectorStoreService.saveWithMappings = jest.fn().mockImplementation(async (embeddings: number[][], mappings: any[]) => {
        ntotalCount += embeddings.length;
    });
    mockVectorStoreService.updateVectorCount = jest.fn().mockResolvedValue(undefined);
    mockVectorStoreService.processPendingDeletions = jest.fn().mockResolvedValue(undefined);
    mockVectorStoreService.save = jest.fn().mockResolvedValue(undefined);
    mockVectorStoreService.getFilePath = jest.fn().mockReturnValue('test-chromadb');

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

      expect(mockVectorStoreService.saveWithMappings).toHaveBeenCalledWith(
        [...embeddingsDoc1, ...embeddingsDoc2],
        expect.arrayContaining([
          expect.objectContaining({ vectorId: expect.any(Number), documentId: expect.any(String) })
        ])
      );
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
  describe('chunkDocument (via addDocuments)', () => {
    it('should handle empty text', async () => {
      const docsToAdd = [{
        content: '',
        url: 'http://example.com/empty',
        title: 'Empty Document',
      }];

      mockOllamaService.getEmbeddings.mockResolvedValue([]);
      
      await ragService.addDocuments(docsToAdd);

      expect(mockOllamaService.getEmbeddings).not.toHaveBeenCalled();
      expect(mockVectorStoreService.add).not.toHaveBeenCalled();
    });

    it('should handle single sentence within chunk size', async () => {
      const docsToAdd = [{
        content: 'This is a single sentence.',
        url: 'http://example.com/single',
        title: 'Single Sentence Document',
      }];

      const embeddings = [[0.1, 0.2, 0.3]];
      mockOllamaService.getEmbeddings.mockResolvedValue(embeddings);
      
      await ragService.addDocuments(docsToAdd);

      expect(mockOllamaService.getEmbeddings).toHaveBeenCalledWith(['This is a single sentence.']);
      expect(mockVectorStoreService.saveWithMappings).toHaveBeenCalledWith(
        embeddings,
        expect.arrayContaining([
          expect.objectContaining({ vectorId: expect.any(Number), documentId: expect.any(String) })
        ])
      );
    });

    it('should create sliding window chunks for multiple sentences', async () => {
      const docsToAdd = [{
        content: 'First sentence. Second sentence. Third sentence. Fourth sentence.',
        url: 'http://example.com/multi',
        title: 'Multi Sentence Document',
      }];

      const embeddings = [
        [0.1, 0.2, 0.3],
        [0.2, 0.3, 0.4],
        [0.3, 0.4, 0.5],
        [0.4, 0.5, 0.6]
      ];
      mockOllamaService.getEmbeddings.mockResolvedValue(embeddings);
      
      await ragService.addDocuments(docsToAdd);

      expect(mockOllamaService.getEmbeddings).toHaveBeenCalledWith([
        'First sentence. Second sentence. Third sentence. Fourth sentence.',
        'First sentence. Second sentence. Third sentence. Fourth sentence.',
        'First sentence. Second sentence. Third sentence. Fourth sentence.',
        'First sentence. Second sentence. Third sentence. Fourth sentence.'
      ]);
    });

    it('should handle text with various sentence endings', async () => {
      const docsToAdd = [{
        content: 'Question? Exclamation! Regular period. Another sentence.',
        url: 'http://example.com/punctuation',
        title: 'Punctuation Document',
      }];

      const embeddings = [
        [0.1, 0.2, 0.3],
        [0.2, 0.3, 0.4],
        [0.3, 0.4, 0.5],
        [0.4, 0.5, 0.6]
      ];
      mockOllamaService.getEmbeddings.mockResolvedValue(embeddings);
      
      await ragService.addDocuments(docsToAdd);

      expect(mockOllamaService.getEmbeddings).toHaveBeenCalledTimes(1);
      expect(mockOllamaService.getEmbeddings).toHaveBeenCalledWith(expect.arrayContaining([
        expect.stringContaining('Question?'),
        expect.stringContaining('Exclamation!'),
        expect.stringContaining('Regular period.'),
        expect.stringContaining('Another sentence.')
      ]));
    });

    it('should respect chunk size limits', async () => {
      const longSentence = 'This is a very long sentence that should exceed the default chunk size of 512 characters when repeated multiple times to test the chunking behavior and ensure that the sliding window algorithm respects the maximum chunk size parameter and does not create chunks that are too large for the embedding model to process efficiently.';
      const docsToAdd = [{
        content: `${longSentence} ${longSentence} ${longSentence}`,
        url: 'http://example.com/long',
        title: 'Long Document',
      }];

      const embeddings = [[0.1, 0.2, 0.3]];
      mockOllamaService.getEmbeddings.mockResolvedValue(embeddings);
      
      await ragService.addDocuments(docsToAdd);

      expect(mockOllamaService.getEmbeddings).toHaveBeenCalledTimes(1);
      const calledChunks = mockOllamaService.getEmbeddings.mock.calls[0][0];
      
      calledChunks.forEach((chunk: string) => {
        expect(chunk.length).toBeLessThanOrEqual(512);
      });
    });

    it('should handle text with only whitespace', async () => {
      const docsToAdd = [{
        content: '   \n\t   ',
        url: 'http://example.com/whitespace',
        title: 'Whitespace Document',
      }];

      mockOllamaService.getEmbeddings.mockResolvedValue([]);
      
      await ragService.addDocuments(docsToAdd);

      expect(mockOllamaService.getEmbeddings).not.toHaveBeenCalled();
      expect(mockVectorStoreService.add).not.toHaveBeenCalled();
    });

    it('should handle malformed sentences gracefully', async () => {
      const docsToAdd = [{
        content: 'Sentence without ending Another sentence. Yet another',
        url: 'http://example.com/malformed',
        title: 'Malformed Document',
      }];

      const embeddings = [
        [0.1, 0.2, 0.3],
        [0.2, 0.3, 0.4],
        [0.3, 0.4, 0.5]
      ];
      mockOllamaService.getEmbeddings.mockResolvedValue(embeddings);
      
      await ragService.addDocuments(docsToAdd);

      expect(mockOllamaService.getEmbeddings).toHaveBeenCalledTimes(1);
      expect(mockVectorStoreService.saveWithMappings).toHaveBeenCalledWith(
        embeddings,
        expect.arrayContaining([
          expect.objectContaining({ vectorId: expect.any(Number), documentId: expect.any(String) })
        ])
      );
    });

    it('should create overlapping chunks with sliding window', async () => {
      const docsToAdd = [{
        content: 'A. B. C. D. E.',
        url: 'http://example.com/overlap',
        title: 'Overlap Test Document',
      }];

      const embeddings = [
        [0.1, 0.2, 0.3],
        [0.2, 0.3, 0.4],
        [0.3, 0.4, 0.5],
        [0.4, 0.5, 0.6],
        [0.5, 0.6, 0.7]
      ];
      mockOllamaService.getEmbeddings.mockResolvedValue(embeddings);
      
      await ragService.addDocuments(docsToAdd);

      const calledChunks = mockOllamaService.getEmbeddings.mock.calls[0][0];
      
      expect(calledChunks).toHaveLength(5);
      
      expect(calledChunks[0]).toContain('A.');
      expect(calledChunks[1]).toContain('B.');
      expect(calledChunks[2]).toContain('C.');
      expect(calledChunks[3]).toContain('D.');
      expect(calledChunks[4]).toContain('E.');
    });
  });

  });

});