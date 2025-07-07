/**
 * Configuration for the Ollama service.
 */
export const OllamaConfig = {
  ollamaApiUrl: process.env.OLLAMA_API_URL || 'http://localhost:11434',
  embeddingModel: 'all-MiniLM-L6-v2',
  embeddingDimension: 384,
  completionModel: 'llama3',
  vectorIndexFile: './data/localmind.index',
};

/**
 * Configuration for the Document Store service.
 */
export const DocumentStoreConfig = {
  documentStoreFile: './data/documents.json',
};

/**
 * Configuration for the Server.
 */
export const ServerConfig = {
  port: process.env.PORT || 3000,
};
