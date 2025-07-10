import axios from 'axios';
import { OllamaService } from './ollama';
import { OllamaConfig, saveConfig } from '../config';

jest.mock('axios');
jest.mock('../config', () => {
  const actualConfig = jest.requireActual('../config');
  return {
    ...actualConfig,
    OllamaConfig: { ...actualConfig.OllamaConfig }, // Ensure OllamaConfig is a mutable copy
    saveConfig: jest.fn(),
  };
});

describe('OllamaService', () => {
  let ollamaService: OllamaService;
  const mockedAxios = axios as jest.Mocked<typeof axios>;

  let originalOllamaConfig: typeof OllamaConfig;
  let originalSaveConfig: typeof saveConfig;

  beforeEach(async () => {
    // Save original config and saveConfig function
    originalOllamaConfig = { ...OllamaConfig };
    originalSaveConfig = saveConfig;

    // Mock for the initial listModels call during OllamaService construction
    mockedAxios.get.mockResolvedValue({
      data: { models: [{ name: 'test-model-1' }, { name: 'test-model-2' }] },
    });
    mockedAxios.post.mockResolvedValue({}); // Mock for initial pullModel calls

    ollamaService = new OllamaService({
      ollamaApiUrl: 'http://localhost:11434',
      embeddingModel: 'test-embedding-model',
      embeddingDimension: 1536,
      completionModel: 'test-completion-model',
      vectorIndexFile: 'mock-vector-index-file.bin',
    });
    // Ensure mocks are cleared after service initialization for subsequent tests
    mockedAxios.post.mockClear();
    mockedAxios.get.mockClear();
  });

  afterEach(() => {
    // Restore original config and saveConfig function
    Object.assign(OllamaConfig, originalOllamaConfig);
    Object.assign(saveConfig, originalSaveConfig);
    jest.clearAllMocks();
  });

  describe('getEmbedding', () => {
    it('should call the correct Ollama endpoint and return the embedding', async () => {
      const mockEmbedding = [0.1, 0.2, 0.3];
      mockedAxios.post.mockResolvedValueOnce({
        data: { embedding: mockEmbedding },
      });

      const text = 'test text';
      const result = await ollamaService.getEmbedding(text);

      expect(mockedAxios.post).toHaveBeenCalledWith(
        'http://localhost:11434/api/embeddings',
        {
          prompt: text,
          model: ollamaService['embeddingModel'],
        },
      );
      expect(result).toEqual(mockEmbedding);
    });

    it('should handle API errors for getEmbedding', async () => {
      const errorMessage = 'API error occurred';
      mockedAxios.post.mockRejectedValueOnce(new Error(errorMessage));

      const text = 'test text';

      await expect(ollamaService.getEmbedding(text)).rejects.toThrow(errorMessage);
    });
  });

  describe('getCompletion', () => {
    it('should call the correct Ollama endpoint and return the response string', async () => {
      const mockResponse = 'This is a test response.';
      mockedAxios.post.mockResolvedValueOnce({
        data: { response: mockResponse },
      });

      const prompt = 'test prompt';
      const result = await ollamaService.getCompletion(prompt);

      expect(mockedAxios.post).toHaveBeenCalledWith(
        'http://localhost:11434/api/generate',
        {
          prompt: prompt,
          model: 'test-completion-model',
          stream: false,
        },
      );
      expect(result).toEqual(mockResponse);
    });

    it('should handle API errors for getCompletion', async () => {
      const errorMessage = 'API error occurred';
      mockedAxios.post.mockRejectedValueOnce(new Error(errorMessage));

      const prompt = 'test prompt';

      await expect(ollamaService.getCompletion(prompt)).rejects.toThrow(errorMessage);
    });
  });

  describe('listModels', () => {
    it('should call the correct Ollama endpoint and return a list of models', async () => {
      const mockModels = [{ name: 'model1' }, { name: 'model2' }];
      mockedAxios.get.mockResolvedValueOnce({
        data: { models: mockModels },
      });

      const result = await ollamaService.listModels();

      expect(mockedAxios.get).toHaveBeenCalledWith(
        'http://localhost:11434/api/tags',
      );
      expect(result).toEqual(['model1', 'model2']);
      expect(ollamaService['availableModels']).toEqual(['model1', 'model2']);
    });

    it('should handle API errors for listModels', async () => {
      const errorMessage = 'API error occurred';
      mockedAxios.get.mockRejectedValueOnce(new Error(errorMessage));

      await expect(ollamaService.listModels()).rejects.toThrow(errorMessage);
    });
  });

  describe('getCompletionModel', () => {
    it('should return the current completion model', () => {
      expect(ollamaService.getCompletionModel()).toEqual('test-completion-model');
    });
  });

  describe('setCompletionModel', () => {
    it('should set the completion model and pull it', async () => {
      mockedAxios.post.mockResolvedValueOnce({}); // Mock pullModel success
      const newModel = 'new-test-model';
      await ollamaService.setCompletionModel(newModel);

      expect(ollamaService['completionModel']).toEqual(newModel);
      expect(mockedAxios.post).toHaveBeenCalledWith(
        'http://localhost:11434/api/pull',
        { name: newModel },
      );
    });

    it('should handle errors when pulling model in setCompletionModel', async () => {
      const errorMessage = 'Pull error';
      mockedAxios.post.mockRejectedValueOnce(new Error(errorMessage));
      const newModel = 'new-test-model-fail';

      await expect(ollamaService.setCompletionModel(newModel)).rejects.toThrow(errorMessage);
      expect(ollamaService['completionModel']).toEqual(newModel); // Model should still be set
    });
  });
});