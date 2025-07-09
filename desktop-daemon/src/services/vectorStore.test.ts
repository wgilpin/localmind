/**
 * @module VectorStoreService
 * @description Unit tests for the VectorStoreService.
 */
import { VectorStoreService } from './vectorStore';
import * as fs from 'fs';

// Mock the faiss-node library
jest.mock('faiss-node', () => {
    const mockIndexInstance = {
        add: jest.fn(),
        search: jest.fn(),
        write: jest.fn(),
        ntotal: jest.fn(),
    };

    // Define a mock constructor function for Index
    const MockIndex = jest.fn().mockImplementation(() => mockIndexInstance);

    // Attach the static 'read' method directly to the MockIndex constructor, casting to any to satisfy TypeScript
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

describe('VectorStoreService', () => {
    const mockModelService: any = {
        embed: jest.fn(async (text: string) => [parseFloat(text), parseFloat(text)]),
        getEmbeddingVectorSize: jest.fn(() => 2),
    };
    const testFilePath = 'test-index.faiss';
    let vectorStoreService: VectorStoreService;
    let mockFaissIndex: any;

    beforeEach(() => {
        jest.clearAllMocks();
        // Ensure that the mock Index is returned consistently
        const { Index } = jest.requireMock('faiss-node'); // Only destructure Index
        mockFaissIndex = {
            add: jest.fn(),
            search: jest.fn(),
            write: jest.fn(),
            ntotal: jest.fn(),
        };
        (Index as any).mockImplementation(() => mockFaissIndex); // Cast Index to any for mockImplementation
        (Index as any).read.mockImplementation(() => mockFaissIndex); // Access read as a static property of Index

        (fs.existsSync as jest.Mock).mockReturnValue(false);
        vectorStoreService = new VectorStoreService(testFilePath);
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
        } = jest.requireMock('faiss-node'); // Get Index mock
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
        } = jest.requireMock('faiss-node'); // Get Index mock
        await vectorStoreService.load(testFilePath);
        expect((Index as any).read).not.toHaveBeenCalled();
    });
});