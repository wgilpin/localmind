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
    console.log('=== OllamaService Constructor Debug ===');
    console.log('Raw config object:', JSON.stringify(config, null, 2));
    console.log('Config embeddingModel:', config.embeddingModel);
    console.log('Config completionModel:', config.completionModel);
    
    this.ollamaApiUrl = config.ollamaApiUrl;
    this.embeddingModel = config.embeddingModel;
    this.completionModel = config.completionModel;
    
    console.log('Assigned embeddingModel:', this.embeddingModel);
    console.log('Assigned completionModel:', this.completionModel);
    console.log('=== End Constructor Debug ===');
    
    this.pullModel(this.embeddingModel);
    this.pullModel(this.completionModel);
  }

  private async pullModel(modelName: string): Promise<void> {
    try {
      await axios.post(`${this.ollamaApiUrl}/api/pull`, {
        name: modelName,
      });
    } catch (error) {
      console.error(`Error pulling model ${modelName}:`, error);
      throw error;
    }
  }

  /**
   * Gets an embedding for the given text from the configured embedding model.
   * @param text The text to get an embedding for.
   * @returns A promise that resolves to an array of numbers representing the embedding.
   * @throws Error if the API request fails or returns an invalid response.
   */
  public async getEmbedding(text: string): Promise<number[]> {
    try {
      console.log('=== getEmbedding Debug ===');
      console.log(`this.embeddingModel: "${this.embeddingModel}"`);
      console.log(`typeof this.embeddingModel: ${typeof this.embeddingModel}`);
      console.log(`Request payload model: "${this.embeddingModel}"`);
      console.log('=== End getEmbedding Debug ===');
      
      const requestPayload = {
        model: this.embeddingModel,
        prompt: text,
      };
      console.log('Full request payload:', JSON.stringify(requestPayload, null, 2));
      
      const response = await axios.post(`${this.ollamaApiUrl}/api/embeddings`, requestPayload);
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