import path from 'path';
import os from 'os';
import fs from 'fs';

export const appDataDir = path.join(os.homedir(), '.localmind'); // Exported appDataDir
const configFilePath = path.join(appDataDir, 'config.json');

/**
 * Interface for Ollama service configuration.
 */
interface IOllamaConfig {
  ollamaApiUrl: string;
  embeddingModel: string;
  embeddingDimension: number;
  completionModel: string;
  vectorIndexFile: string;
  chromaDbPath: string;
  excludeFolders: string[];
}

/**
 * Interface for Server configuration.
 */
interface IServerConfig {
  port: number;
}

/**
 * Default configuration values.
 */
const defaultConfig = {
  ollama: {
    ollamaApiUrl: process.env.OLLAMA_API_URL || 'http://localhost:11434',
    embeddingModel: 'mahonzhan/all-MiniLM-L6-v2',
    embeddingDimension: 384,
    completionModel: 'qwen3:0.6b',
    vectorIndexFile: path.join(appDataDir, 'localmind.index'),
    chromaDbPath: path.join(appDataDir, 'chromadb'),
    excludeFolders: [
      'node_modules',
      '.git',
      '.svn',
      '.hg',
      'target',
      'build',
      'dist',
      '.next',
      '.nuxt',
      'coverage',
      '.nyc_output',
      '.cache',
      'tmp',
      'temp',
      'logs',
      '.DS_Store',
      'Thumbs.db'
    ],
  },
  server: {
    port: parseInt(process.env.PORT || '3000', 10), // Explicitly parse to number
  },
};

export let OllamaConfig: IOllamaConfig;
export let ServerConfig: IServerConfig;

/**
 * Loads the configuration from a file or uses default values.
 */
export function loadConfig() {
  if (fs.existsSync(configFilePath)) {
    try {
      const configData = JSON.parse(fs.readFileSync(configFilePath, 'utf-8'));
      OllamaConfig = { ...defaultConfig.ollama, ...configData.ollama };
      ServerConfig = { ...defaultConfig.server, ...configData.server };
    } catch (error) {
      console.error('Error loading config file, using default config:', error);
      OllamaConfig = defaultConfig.ollama;
      ServerConfig = defaultConfig.server;
    }
  } else {
    OllamaConfig = defaultConfig.ollama;
    ServerConfig = defaultConfig.server;
  }
}

/**
 * Saves the current configuration to a file.
 */
export function saveConfig() {
  try {
    if (!fs.existsSync(appDataDir)) {
      fs.mkdirSync(appDataDir, { recursive: true });
    }
    const configToSave = {
      ollama: OllamaConfig,
      server: ServerConfig,
    };
    fs.writeFileSync(configFilePath, JSON.stringify(configToSave, null, 2));
  } catch (error) {
    console.error('Error saving config file:', error);
  }
}

// Load config on initial import
loadConfig();
