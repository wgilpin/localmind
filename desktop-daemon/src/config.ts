import path from 'path';
import os from 'os';

const appDataDir = path.join(os.homedir(), '.localmind');

/**
 * Configuration for the Ollama service.
 */
export const OllamaConfig = {
  ollamaApiUrl: process.env.OLLAMA_API_URL || 'http://localhost:11434',
  embeddingModel: 'mahonzhan/all-MiniLM-L6-v2',
  embeddingDimension: 384,
  completionModel: 'llama3.2:3b',
  vectorIndexFile: path.join(appDataDir, 'localmind.index'),
};

/**
 * Configuration for the Document Store service.
 */
export const DocumentStoreConfig = {
  documentStoreFile: path.join(appDataDir, 'documents.json'),
};

/**
 * Configuration for the Server.
 */
export const ServerConfig = {
  port: process.env.PORT || 3000,
};
