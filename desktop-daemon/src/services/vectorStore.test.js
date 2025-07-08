"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
/**
 * @module VectorStoreService
 * @description Unit tests for the VectorStoreService.
 */
const vectorStore_1 = require("./vectorStore");
const fs = __importStar(require("fs"));
// Mock the faiss-node library
jest.mock('faiss-node', () => {
    const mockIndexInstance = {
        add: jest.fn(),
        search: jest.fn(),
        write: jest.fn(),
    };
    // Define a mock constructor function for Index
    const MockIndex = jest.fn().mockImplementation(() => mockIndexInstance);
    // Attach the static 'read' method directly to the MockIndex constructor, casting to any to satisfy TypeScript
    MockIndex.read = jest.fn().mockReturnValue(mockIndexInstance);
    return {
        Index: MockIndex,
    };
});
// Mock fs.existsSync and fs.mkdirSync
jest.mock('fs', () => (Object.assign(Object.assign({}, jest.requireActual('fs')), { existsSync: jest.fn(), mkdirSync: jest.fn() })));
describe('VectorStoreService', () => {
    const mockModelService = {
        embed: jest.fn((text) => __awaiter(void 0, void 0, void 0, function* () { return [parseFloat(text), parseFloat(text)]; })),
        getEmbeddingVectorSize: jest.fn(() => 2),
    };
    const testFilePath = 'test-index.faiss';
    let vectorStoreService;
    let mockFaissIndex;
    beforeEach(() => {
        jest.clearAllMocks();
        // Ensure that the mock Index is returned consistently
        const { Index } = jest.requireMock('faiss-node'); // Only destructure Index
        mockFaissIndex = {
            add: jest.fn(),
            search: jest.fn(),
            write: jest.fn(),
        };
        Index.mockImplementation(() => mockFaissIndex); // Cast Index to any for mockImplementation
        Index.read.mockImplementation(() => mockFaissIndex); // Access read as a static property of Index
        fs.existsSync.mockReturnValue(false);
        vectorStoreService = new vectorStore_1.VectorStoreService(testFilePath);
    });
    /**
     * @method add
     * @description Test that the service's `add` method calls the `add` method on the mocked FAISS index.
     */
    test('add should call faiss index add method', () => __awaiter(void 0, void 0, void 0, function* () {
        const documents = [{ id: '1', text: 'hello world' }];
        const embeddings = yield Promise.all(documents.map(doc => mockModelService.embed(doc.text)));
        yield vectorStoreService.add(embeddings);
        expect(mockModelService.embed).toHaveBeenCalledWith('hello world');
        expect(mockFaissIndex.add).toHaveBeenCalledTimes(1);
        expect(mockFaissIndex.add).toHaveBeenCalledWith(embeddings[0]); // Ensure it's called with the actual embedding
    }));
    /**
     * @method search
     * @description Test that the service's `search` method calls the `search` method on the mocked FAISS index with the correct `k` value.
     */
    test('search should call faiss index search method with correct k', () => __awaiter(void 0, void 0, void 0, function* () {
        const query = 'test query';
        const k = 5;
        const queryEmbedding = yield mockModelService.embed(query);
        mockFaissIndex.search.mockReturnValue({ distances: [0.1, 0.2], labels: [0, 1] });
        yield vectorStoreService.search(queryEmbedding, k);
        expect(mockModelService.embed).toHaveBeenCalledWith(query);
        expect(mockFaissIndex.search).toHaveBeenCalledWith(queryEmbedding, k);
    }));
    /**
     * @method save
     * @description Test that the `save` method calls the `write` method on the mocked index.
     */
    test('save should call faiss index write method', () => __awaiter(void 0, void 0, void 0, function* () {
        yield vectorStoreService.save();
        expect(mockFaissIndex.write).toHaveBeenCalledWith(testFilePath);
    }));
    /**
     * @method load
     * @description Test that the `load` method calls the static `read` method on the mocked FAISS `Index` class.
     */
    test('load should call faiss Index.read method if file exists', () => __awaiter(void 0, void 0, void 0, function* () {
        fs.existsSync.mockReturnValue(true);
        const { Index } = jest.requireMock('faiss-node'); // Get Index mock
        yield vectorStoreService.load();
        expect(Index.read).toHaveBeenCalledWith(testFilePath); // Access read as static property
    }));
    /**
     * @method load
     * @description Test that the `load` method does not call `read` if the file does not exist.
     */
    test('load should not call faiss Index.read method if file does not exist', () => __awaiter(void 0, void 0, void 0, function* () {
        fs.existsSync.mockReturnValue(false);
        const { Index } = jest.requireMock('faiss-node'); // Get Index mock
        yield vectorStoreService.load();
        expect(Index.read).not.toHaveBeenCalled(); // Access read as static property
    }));
});
