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
exports.OllamaService = void 0;
const axios_1 = __importDefault(require("axios"));
const config_1 = require("../config");
/**
 * Service for interacting with the Ollama API to get embeddings and completions.
 */
class OllamaService {
    /**
     * Constructs an OllamaService instance.
     * @param config Optional configuration for Ollama API URL and models.
     */
    constructor(config = config_1.OllamaConfig) {
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
    getEmbedding(text) {
        return __awaiter(this, void 0, void 0, function* () {
            try {
                const response = yield axios_1.default.post(`${this.ollamaApiUrl}/api/embeddings`, {
                    model: this.embeddingModel,
                    prompt: text,
                });
                if (response.data && response.data.embedding) {
                    return response.data.embedding;
                }
                throw new Error('Invalid embedding response from Ollama API');
            }
            catch (error) {
                console.error('Error getting embedding:', error);
                throw error;
            }
        });
    }
    /**
     * Gets a completion for the given prompt from the configured completion model.
     * @param prompt The prompt for which to get a completion.
     * @returns A promise that resolves to a string representing the completion.
     * @throws Error if the API request fails or returns an invalid response.
     */
    getCompletion(prompt) {
        return __awaiter(this, void 0, void 0, function* () {
            try {
                const response = yield axios_1.default.post(`${this.ollamaApiUrl}/api/generate`, {
                    model: this.completionModel,
                    prompt: prompt,
                    stream: false, // Ensure we get the full response at once
                });
                if (response.data && response.data.response) {
                    return response.data.response;
                }
                throw new Error('Invalid completion response from Ollama API');
            }
            catch (error) {
                console.error('Error getting completion:', error);
                throw error;
            }
        });
    }
}
exports.OllamaService = OllamaService;
