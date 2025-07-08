"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ServerConfig = exports.DocumentStoreConfig = exports.OllamaConfig = void 0;
/**
 * Configuration for the Ollama service.
 */
exports.OllamaConfig = {
    ollamaApiUrl: process.env.OLLAMA_API_URL || 'http://localhost:11434',
    embeddingModel: 'mahonzhan/all-MiniLM-L6-v2',
    embeddingDimension: 384,
    completionModel: 'llama3.2:3b',
    vectorIndexFile: './data/localmind.index',
};
/**
 * Configuration for the Document Store service.
 */
exports.DocumentStoreConfig = {
    documentStoreFile: './data/documents.json',
};
/**
 * Configuration for the Server.
 */
exports.ServerConfig = {
    port: process.env.PORT || 3000,
};
