/**
 * Configuration for the Ollama service.
 */
export const OllamaConfig = {
  ollamaApiUrl: process.env.OLLAMA_API_URL || 'http://localhost:11434',
  embeddingModel: 'all-MiniLM-L6-v2',
  completionModel: 'llama3',
};
