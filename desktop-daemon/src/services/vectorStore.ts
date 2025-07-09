import { Index, MetricType } from 'faiss-node';
import * as fs from 'fs';
import { OllamaConfig } from '../config';

/**
 * Service for managing a FAISS vector store.
 */
export class VectorStoreService {
  private index: Index;
  private readonly dimension: number;
  public filePath: string;

  constructor(filePath: string) {
    this.dimension = OllamaConfig.embeddingDimension;
    this.index = new Index(this.dimension);
    this.filePath = filePath;
  }

  /**
   * Returns the file path for the vector store.
   * @returns The file path.
   */
  getFilePath(): string {
    return this.filePath;
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
    const ntotal = this.index.ntotal();
    console.log(`=== VectorStore Search Debug ===`);
    console.log(`Requested k: ${k}`);
    console.log(`Available ntotal: ${ntotal}`);
    
    if (ntotal === 0) {
      console.log(`Database is empty, returning empty results`);
      return { I: [], D: [] };
    }
    
    const effectiveK = Math.min(k, ntotal);
    console.log(`Using effective k: ${effectiveK}`);
    console.log(`=== End VectorStore Search Debug ===`);
    
    const result = this.index.search(queryVector, effectiveK);
    return { I: result.labels, D: result.distances };
  }

  /**
   * Saves the index to a file.
   * @param path The path to save the index to.
   */
  async save(path?: string): Promise<void> {
    const pathToSave = path || this.filePath;
    try {
      await this.index.write(pathToSave);
      console.log(`FAISS index saved to ${pathToSave}`);
    } catch (error) {
      console.error(`Error saving FAISS index to ${pathToSave}:`, error);
      throw error;
    }
  }

  /**
   * Loads the index from a file, creating it if it doesn't exist.
   * @param path The path to load the index from.
   */
  async load(path?: string): Promise<void> {
    const pathToLoad = path || this.filePath;
    try {
      if (fs.existsSync(pathToLoad)) {
        this.index = await Index.read(pathToLoad);
        console.log(`FAISS index loaded from ${pathToLoad}`);
      } else {
        console.log(`FAISS index file not found at ${pathToLoad}. Creating a new index.`);
        this.index = new Index(this.dimension);
      }
    } catch (error) {
      console.error(`Error loading FAISS index from ${pathToLoad}:`, error);
      throw error;
    }
  }
}