/**
 * @module VectorStoreService
 * @description Unit tests for the VectorStoreService.
 */
import { VectorStoreService } from './vectorStore';
import { DatabaseService } from './database';
import { OllamaService } from './ollama';
import * as fs from 'fs';

// Mock the faiss-node library
jest.mock('faiss-node', () => {
    const mockIndexInstance = {
        add: jest.fn(),
        search: jest.fn(),
        write: jest.fn(),
        ntotal: jest.fn(),
        deleteVector: jest.fn(), // Add mock for deleteVector
    };

    const MockIndex = jest.fn().mockImplementation(() => mockIndexInstance);

    (MockIndex as any).read = jest.fn().mockReturnValue(mockIndexInstance);

    return {
        Index: MockIndex,
    };
});

// Mock fs.existsSync and fs.mkdirSync
jest.mock('fs', () => ({
    ...jest.requireActual('fs'),
    existsSync: jest.fn(),
    mkdirSync: jest.fn(),
}));

// Mock the DatabaseService module
jest.mock('./database', () => {
    const actualModule = jest.requireActual('./database');
    return {
        ...actualModule,
        DatabaseService: jest.fn().mockImplementation(() => {
            const mockDb = {
                prepare: jest.fn().mockReturnThis(),
                run: jest.fn(),
                get: jest.fn(),
                all: jest.fn(),
                transaction: jest.fn((cb) => {
                    return jest.fn(() => cb());
                }),
                close: jest.fn(),
            };
            // Return a mock instance that behaves like DatabaseService but uses the mockDb
            return {
                insertDocument: jest.fn(),
                insertVectorMappings: jest.fn(),
                getDocumentById: jest.fn(),
                getDocumentsByIds: jest.fn(),
                getAllDocuments: jest.fn(),
                getDocumentIdByVectorId: jest.fn(),
                getDocumentIdsByVectorIds: jest.fn(),
                getVectorIdsByDocumentId: jest.fn(),
                getVectorMappingsByIds: jest.fn(),
                deleteDocument: jest.fn(),
                transaction: jest.fn((cb) => {
                    return jest.fn(() => cb());
                }),
                close: jest.fn(),
            };
        }),
    };
});


describe('VectorStoreService', () => {
    const testFilePath = 'test-index.faiss';
    let vectorStoreService: VectorStoreService;
    let mockFaissIndex: any;
    let mockDatabaseService: jest.Mocked<DatabaseService>;
    let mockOllamaService: jest.Mocked<OllamaService>;

    beforeEach(() => {
        jest.clearAllMocks();
        const { Index } = jest.requireMock('faiss-node');
        mockFaissIndex = {
            add: jest.fn(),
            search: jest.fn(),
            write: jest.fn(),
            ntotal: jest.fn(),
            deleteVector: jest.fn(),
        };
        (Index as any).mockImplementation(() => mockFaissIndex);
        (Index as any).read.mockImplementation(() => mockFaissIndex);

        (fs.existsSync as jest.Mock).mockReturnValue(false);
        
        // Get the mocked DatabaseService instance created by the jest.mock above
        mockDatabaseService = new DatabaseService('dummy-path') as jest.Mocked<DatabaseService>;
        mockOllamaService = new OllamaService() as jest.Mocked<OllamaService>;
        vectorStoreService = new VectorStoreService(testFilePath, mockDatabaseService, mockOllamaService);
    });

    /**
     * @method add
     * @description Test that the service's `add` method calls the `add` method on the mocked FAISS index.
     */
    test('add should call faiss index add method', () => {
        const embeddings = [
            [1, 2, 3]
        ];
        vectorStoreService.add(embeddings);
        expect(mockFaissIndex.add).toHaveBeenCalledTimes(1);
        expect(mockFaissIndex.add).toHaveBeenCalledWith(embeddings[0]);
    });

    /**
     * @method search
     * @description Test that the service's `search` method calls the `search` method on the mocked FAISS index with the correct `k` value.
     */
    test('search should call faiss index search method with correct k', async () => {
        const k = 5;
        const queryEmbedding = [0.1, 0.2];
        mockFaissIndex.ntotal.mockReturnValue(10);
        mockFaissIndex.search.mockReturnValue({
            distances: [0.1, 0.2],
            labels: [0, 1]
        });
        await vectorStoreService.search(queryEmbedding, k);
        expect(mockFaissIndex.search).toHaveBeenCalledWith(queryEmbedding, k);
    });

    /**
     * @method save
     * @description Test that the `save` method calls the `write` method on the mocked index.
     */
    test('save should call faiss index write method', async () => {
        await vectorStoreService.save(testFilePath);
        expect(mockFaissIndex.write).toHaveBeenCalledWith(testFilePath);
    });

    /**
     * @method load
     * @description Test that the `load` method calls the static `read` method on the mocked FAISS `Index` class.
     */
    test('load should call faiss Index.read method if file exists', async () => {
        (fs.existsSync as jest.Mock).mockReturnValue(true);
        const {
            Index
        } = jest.requireMock('faiss-node');
        await vectorStoreService.load(testFilePath);
        expect((Index as any).read).toHaveBeenCalledWith(testFilePath);
    });

    /**
     * @method load
     * @description Test that the `load` method does not call `read` if the file does not exist.
     */
    test('load should not call faiss Index.read method if file does not exist', async () => {
        (fs.existsSync as jest.Mock).mockReturnValue(false);
        const {
            Index
        } = jest.requireMock('faiss-node');
        await vectorStoreService.load(testFilePath);
        expect((Index as any).read).not.toHaveBeenCalled();
    });
});