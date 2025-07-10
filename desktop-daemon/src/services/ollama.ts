import axios from "axios";
import { OllamaConfig, saveConfig } from "../config";

/**
 * Service for interacting with the Ollama API to get embeddings and completions.
 */
export class OllamaService {
  private ollamaApiUrl: string;
  private embeddingModel: string;
  private completionModel: string;
  private availableModels: string[] = [];

  /**
   * Constructs an OllamaService instance.
   * @param config Optional configuration for Ollama API URL and models.
   */
  constructor(config = OllamaConfig) {
    this.ollamaApiUrl = config.ollamaApiUrl;
    this.embeddingModel = config.embeddingModel;
    this.completionModel = config.completionModel;
    this.initializeModels();
  }

  private async initializeModels(): Promise<void> {
    await this.pullModel(this.embeddingModel);
    await this.pullModel(this.completionModel);
    await this.listModels();
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
   * Lists available models from the Ollama API.
   * @returns A promise that resolves to an array of model names.
   * @throws Error if the API request fails.
   */
  public async listModels(): Promise<string[]> {
    try {
      const response = await axios.get(`${this.ollamaApiUrl}/api/tags`);
      if (response.data && Array.isArray(response.data.models)) {
        this.availableModels = response.data.models.map((model: any) => model.name);
        return this.availableModels;
      }
      throw new Error("Invalid response from Ollama API for listing models");
    } catch (error) {
      console.error("Error listing models:", error);
      throw error;
    }
  }

  /**
   * Gets the currently configured completion model.
   * @returns The name of the completion model.
   */
  public getCompletionModel(): string {
    return this.completionModel;
  }

  /**
   * Sets the completion model and pulls it if not available locally.
   * @param modelName The name of the model to set as the completion model.
   * @returns A promise that resolves when the model is set and pulled.
   */
  public async setCompletionModel(modelName: string): Promise<void> {
    this.completionModel = modelName;
    OllamaConfig.completionModel = modelName; // Update the config object
    saveConfig(); // Save the updated config
    await this.pullModel(modelName);
  }

  /**
   * Gets an embedding for the given text from the configured embedding model.
   * @param text The text to get an embedding for.
   * @returns A promise that resolves to an array of numbers representing the embedding.
   * @throws Error if the API request fails or returns an invalid response.
   */
  public async getEmbedding(text: string): Promise<number[]> {
    try {
      const requestPayload = {
        model: this.embeddingModel,
        prompt: text,
      };

      const response = await axios.post(
        `${this.ollamaApiUrl}/api/embeddings`,
        requestPayload
      );
      if (response.data && response.data.embedding) {
        return response.data.embedding;
      }
      throw new Error("Invalid embedding response from Ollama API");
    } catch (error) {
      console.error("Error getting embedding:", error);
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
      throw new Error("Invalid completion response from Ollama API");
    } catch (error) {
      console.error("Error getting completion:", error);
      throw error;
    }
  }

  /**
   * Gets a streaming completion for the given prompt from the configured completion model.
   * @param prompt The prompt for which to get a completion.
   * @returns An async generator that yields response chunks.
   */
  public async *getCompletionStream(prompt: string): AsyncGenerator<string> {
    console.time("ollamaStreamTime");
    const response = await axios.post(
      `${this.ollamaApiUrl}/api/generate`,
      {
        model: this.completionModel,
        prompt: prompt,
        stream: true,
      },
      { responseType: "stream" }
    );
    console.timeLog("ollamaStreamTime", `ollama stream started`);
    let buffer = "";
    try {
      for await (const chunk of response.data) {
        buffer += chunk.toString();
        const lines = buffer.split("\n");
        buffer = lines.pop() || "";

        for (const line of lines) {
          if (line.trim() === "") continue;
          const parsed = JSON.parse(line);
          if (parsed.response) {
            console.timeLog("ollamaStreamTime", `ollama stream: ${parsed.response}`);
            yield parsed.response;
          }
        }
      }
      if (buffer.trim() !== "") {
        const parsed = JSON.parse(buffer);
        if (parsed.response) {
          console.timeLog("ollamaStreamTime", `ollama stream: ${parsed.response}`);
          yield parsed.response;
        }
      }
    } catch (error) {
      console.timeLog("ollamaStreamTime", `Ollama error: ${error}`);
    }
  }

  /**
   * Gets embeddings for an array of texts from the configured embedding model.
   * @param texts An array of texts to get embeddings for.
   * @returns A promise that resolves to a 2D array of numbers representing the embeddings.
   * @throws Error if any API request fails or returns an invalid response.
   */
  public async getEmbeddings(texts: string[]): Promise<number[][]> {
    const embeddings: number[][] = [];
    for (const text of texts) {
      embeddings.push(await this.getEmbedding(text));
    }
    return embeddings;
  }
}
