import axios from 'axios';
import { OllamaConfig } from '../config';

/**
 * Service for interacting with the Ollama API to get embeddings and completions.
 */
export class OllamaService {
  private ollamaApiUrl: string;
  private embeddingModel: string;
  private completionModel: string;

  /**
   * Constructs an OllamaService instance.
   * @param config Optional configuration for Ollama API URL and models.
   */
  constructor(config = OllamaConfig) {
    this.ollamaApiUrl = config.ollamaApiUrl;
    this.embeddingModel = config.embeddingModel;
    this.completionModel = config.completionModel;
  }

  /**
   * Gets an embedding for the given text from the configured embedding model.
   * @param text The text to get an embedding for.
   * @returns A promise that resolves to an array of numbers representing the embedding.
   * @throws Error if the API request fails or returns an invalid response.
   */
  public async getEmbedding(text: string): Promise<number[]> {
    try {
      const response = await axios.post(`${this.ollamaApiUrl}/api/embeddings`, {
        model: this.embeddingModel,
        prompt: text,
      });
      if (response.data && response.data.embedding) {
        return response.data.embedding;
      }
      throw new Error('Invalid embedding response from Ollama API');
    } catch (error) {
      console.error('Error getting embedding:', error);
      throw error;
    }
  }

  /**
   * Gets a completion for the given prompt from the configured completion model.
   * @param prompt The prompt for which to get a completion.
   * @returns A promise that resolves to a string representing the completion.
   * @throws Error if the API request fails or returns an invalid response.
   */
  public async getCompletion(prompt: string): Promise<string> {
    try {
      const response = await axios.post(`${this.ollamaApiUrl}/api/generate`, {
        model: this.completionModel,
        prompt: prompt,
        stream: false, // Ensure we get the full response at once
      });
      if (response.data && response.data.response) {
        return response.data.response;
      }
      throw new Error('Invalid completion response from Ollama API');
    } catch (error) {
      console.error('Error getting completion:', error);
      throw error;
    }
  }
}