"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const axios_1 = __importDefault(require("axios"));
const ollama_1 = require("./ollama");
jest.mock('axios');
describe('OllamaService', () => {
    let ollamaService;
    const mockedAxios = axios_1.default;
    beforeEach(() => {
        ollamaService = new ollama_1.OllamaService({
            ollamaApiUrl: 'http://localhost:11434',
            embeddingModel: 'test-embedding-model',
            embeddingDimension: 1536,
            completionModel: 'test-completion-model',
            vectorIndexFile: 'mock-vector-index-file.bin',
        });
        mockedAxios.post.mockClear();
    });
    describe('getEmbedding', () => {
        it('should call the correct Ollama endpoint and return the embedding', () => __awaiter(void 0, void 0, void 0, function* () {
            const mockEmbedding = [0.1, 0.2, 0.3];
            mockedAxios.post.mockResolvedValueOnce({
                data: { embedding: mockEmbedding },
            });
            const text = 'test text';
            const result = yield ollamaService.getEmbedding(text);
            expect(mockedAxios.post).toHaveBeenCalledWith('http://localhost:11434/api/embeddings', {
                prompt: text,
                model: ollamaService['embeddingModel'],
            });
            expect(result).toEqual(mockEmbedding);
        }));
        it('should handle API errors for getEmbedding', () => __awaiter(void 0, void 0, void 0, function* () {
            const errorMessage = 'API error occurred';
            mockedAxios.post.mockRejectedValueOnce(new Error(errorMessage));
            const text = 'test text';
            yield expect(ollamaService.getEmbedding(text)).rejects.toThrow(errorMessage);
        }));
    });
    describe('getCompletion', () => {
        it('should call the correct Ollama endpoint and return the response string', () => __awaiter(void 0, void 0, void 0, function* () {
            const mockResponse = 'This is a test response.';
            mockedAxios.post.mockResolvedValueOnce({
                data: { response: mockResponse },
            });
            const prompt = 'test prompt';
            const result = yield ollamaService.getCompletion(prompt);
            expect(mockedAxios.post).toHaveBeenCalledWith('http://localhost:11434/api/generate', {
                prompt: prompt,
                model: 'test-completion-model',
                stream: false,
            });
            expect(result).toEqual(mockResponse);
        }));
        it('should handle API errors for getCompletion', () => __awaiter(void 0, void 0, void 0, function* () {
            const errorMessage = 'API error occurred';
            mockedAxios.post.mockRejectedValueOnce(new Error(errorMessage));
            const prompt = 'test prompt';
            yield expect(ollamaService.getCompletion(prompt)).rejects.toThrow(errorMessage);
        }));
    });
});
