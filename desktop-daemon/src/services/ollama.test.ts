import axios from 'axios';
import { OllamaService } from './ollama';

jest.mock('axios');

describe('OllamaService', () => {
  let ollamaService: OllamaService;
  const mockedAxios = axios as jest.Mocked<typeof axios>;

  beforeEach(() => {
    ollamaService = new OllamaService({
      ollamaApiUrl: 'http://localhost:11434',
      embeddingModel: 'test-embedding-model',
      embeddingDimension: 1536,
      completionModel: 'test-completion-model',
      vectorIndexFile: 'mock-vector-index-file.bin',
    });
    mockedAxios.post.mockClear();
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
});