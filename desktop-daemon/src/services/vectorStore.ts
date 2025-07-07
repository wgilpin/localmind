import { Index, MetricType } from 'faiss-node';
import * as fs from 'fs';
import { OllamaConfig } from '../config';

/**
 * Service for managing a FAISS vector store.
 */
export class VectorStoreService {
  private index: Index;
  private readonly dimension: number;

  constructor() {
    this.dimension = OllamaConfig.embeddingDimension;
    this.index = new Index(this.dimension);
  }

  /**
   * Adds a batch of vectors to the index.
   * @param vectors The vectors to add.
   */
  add(vectors: number[][]): void {
    // faiss-node add expects a 2D array of vectors
    vectors.forEach(vector => this.index.add(vector));
  }

  /**
   * Searches the index for the k nearest neighbors to the queryVector.
   * @param queryVector The vector to query.
   * @param k The number of nearest neighbors to retrieve.
   * @returns A promise that resolves to an object containing indices (I) and distances (D).
   */
  async search(queryVector: number[], k: number): Promise<{ I: number[], D: number[] }> {
    const result = this.index.search(queryVector, k);
    return { I: result.labels, D: result.distances };
  }

  /**
   * Saves the index to a file.
   * @param path The path to save the index to.
   */
  async save(path: string): Promise<void> {
    try {
      await this.index.write(path);
      console.log(`FAISS index saved to ${path}`);
    } catch (error) {
      console.error(`Error saving FAISS index to ${path}:`, error);
      throw error;
    }
  }

  /**
   * Loads the index from a file, creating it if it doesn't exist.
   * @param path The path to load the index from.
   */
  async load(path: string): Promise<void> {
    try {
      if (fs.existsSync(path)) {
        this.index = await Index.read(path);
        console.log(`FAISS index loaded from ${path}`);
      } else {
        console.log(`FAISS index file not found at ${path}. Creating a new index.`);
        this.index = new Index(this.dimension);
      }
    } catch (error) {
      console.error(`Error loading FAISS index from ${path}:`, error);
      throw error;
    }
  }
}