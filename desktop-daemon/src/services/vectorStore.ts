import { Index, MetricType } from 'faiss-node';
import * as fs from 'fs';
import { DatabaseService } from './database';
import { OllamaConfig } from '../config';
import { OllamaService } from './ollama';

/**
 * Service for managing a FAISS vector store.
 */
export class VectorStoreService {
  private index: Index;
  private readonly dimension: number;
  public filePath: string;
  private databaseService: DatabaseService;
  private ollamaService: OllamaService;

  constructor(
    filePath: string,
    databaseService: DatabaseService,
    ollamaService: OllamaService,
  ) {
    this.dimension = OllamaConfig.embeddingDimension;
    this.index = new Index(this.dimension);
    this.filePath = filePath;
    this.databaseService = databaseService;
    this.ollamaService = ollamaService;
  }

  /**
   * Initializes the vector store by loading the index from the specified file path.
   * If the file does not exist, it creates a new empty index.
   */
  async init(): Promise<void> {
    try {
      if (fs.existsSync(this.filePath)) {
        await this.load(this.filePath);
      } else {
        this.index = new Index(this.dimension);
        console.log(
          'No existing index file found. A new index has been created.',
        );
        await this.rebuildIndex();
      }
    } catch (error) {
      console.error('Failed to initialize vector store:', error);
      // Fallback to a new index if loading fails
      this.index = new Index(this.dimension);
    }
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
   * Returns the total number of vectors in the index.
   * @returns The total number of vectors.
   */
  ntotal(): number {
    return this.index.ntotal();
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
   * Deletes vectors from the index by their internal FAISS IDs.
   * @param ids The array of internal FAISS IDs to delete.
   */
  deleteVector(ids: number[]): void {
    if (ids.length === 0) {
      return;
    }
    this.index.removeIds(ids);
  }

  /**
   * Saves the index to a file.
   * @param path The path to save the index to.
   */
  async save(path?: string): Promise<void> {
    const pathToSave = path || this.filePath;
    try {
      await this.index.write(pathToSave);
    } catch (error) {
      console.error(`Error saving FAISS index to ${pathToSave}:`, error);
      throw error;
    }
  }

  async rebuildIndex(): Promise<void> {
    console.log('Rebuilding index from database...');
    const documents = this.databaseService.getAllDocuments();
    if (documents.length === 0) {
      console.log('No documents in database to rebuild index from.');
      return;
    }

    const allEmbeddings: number[][] = [];
    for (const doc of documents) {
      const embeddings = await this.ollamaService.getEmbeddings([doc.content]);
      allEmbeddings.push(...embeddings);
    }

    this.add(allEmbeddings);
    await this.save();
    console.log(
      `Index rebuilt with ${this.ntotal()} vectors and saved to ${
        this.filePath
      }`,
    );
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