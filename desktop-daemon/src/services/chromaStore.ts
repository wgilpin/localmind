import { ChromaClient, Collection, OpenAIEmbeddingFunction } from 'chromadb';
import { DatabaseService } from './database';
import { OllamaService } from './ollama';
import { OllamaConfig } from '../config';
import { cleanText } from '../utils/textProcessor';
import * as path from 'path';

export class ChromaStoreService {
  private client: ChromaClient;
  private collection?: Collection;
  private readonly collectionName: string = 'localmind_documents';
  private databaseService: DatabaseService;
  private ollamaService: OllamaService;
  private persistDirectory: string;
  private pendingVectors?: number[][];
  private embeddingFunction: OpenAIEmbeddingFunction;

  constructor(
    persistDirectory: string,
    databaseService: DatabaseService,
    ollamaService: OllamaService,
  ) {
    this.persistDirectory = persistDirectory;
    this.databaseService = databaseService;
    this.ollamaService = ollamaService;
    
    // Create a dummy embedding function (we handle embeddings externally with Ollama)
    this.embeddingFunction = new OpenAIEmbeddingFunction({
      openai_api_key: "dummy-key",
      openai_model: "text-embedding-ada-002"
    });
    
    this.client = new ChromaClient({
      path: 'http://localhost:8000'
    });
  }

  async init(): Promise<void> {
    try {
      console.log('Initializing ChromaDB...');
      
      // Check ChromaDB server version for compatibility
      try {
        const version = await this.client.version();
        console.log('ChromaDB server version:', version);
      } catch (versionError) {
        console.warn('Could not get ChromaDB server version:', versionError);
      }
      
      // Test basic connection with heartbeat
      try {
        const heartbeat = await this.client.heartbeat();
        console.log('ChromaDB heartbeat response:', heartbeat);
      } catch (heartbeatError) {
        console.warn('ChromaDB heartbeat failed:', heartbeatError);
      }
      
      const collections = await this.client.listCollections();
      const existingCollection = collections.find(c => c.name === this.collectionName);
      
      if (existingCollection) {
        this.collection = await this.client.getCollection({
          name: this.collectionName,
          embeddingFunction: this.embeddingFunction
        });
        console.log(`ChromaDB collection '${this.collectionName}' loaded.`);
      } else {
        this.collection = await this.client.createCollection({
          name: this.collectionName,
          embeddingFunction: this.embeddingFunction,
          metadata: { 
            dimension: OllamaConfig.embeddingDimension.toString() 
          }
        });
        console.log(`ChromaDB collection '${this.collectionName}' created.`);
        await this.rebuildIndex();
      }
      
      await this.updateVectorCount();
      console.log(`ChromaDB initialized with ${this.vectorCount} vectors.`);
    } catch (error) {
      console.error('Failed to initialize ChromaDB:', error);
      throw error;
    }
  }

  getFilePath(): string {
    return this.persistDirectory;
  }

  add(vectors: number[][]): void {
    if (!this.collection) {
      throw new Error('ChromaDB collection not initialized');
    }
    
    if (vectors.length === 0) {
      return;
    }
    
    // Store vectors temporarily for batch processing
    // They will be persisted with proper IDs when saveWithMappings is called
    this.pendingVectors = this.pendingVectors || [];
    this.pendingVectors.push(...vectors);
  }

  async saveWithMappings(vectors: number[][], mappings: { vectorId: number; documentId: string }[]): Promise<void> {
    if (!this.collection) {
      throw new Error('ChromaDB collection not initialized');
    }
    
    if (vectors.length === 0 || mappings.length === 0) {
      return;
    }
    
    const ids = mappings.map(m => `${m.documentId}_vec_${m.vectorId}`);
    const metadatas = mappings.map(m => ({ 
      documentId: m.documentId,
      vectorId: m.vectorId 
    }));
    
    await this.collection.add({
      ids: ids,
      embeddings: vectors,
      metadatas: metadatas
    });
  }

  ntotal(): number {
    // For compatibility with synchronous interface, we'll track count locally
    // This will be updated when init() is called
    return this.vectorCount || 0;
  }

  private vectorCount: number = 0;

  async updateVectorCount(): Promise<void> {
    if (this.collection) {
      this.vectorCount = await this.collection.count();
    }
  }

  async search(queryVector: number[], k: number): Promise<{ I: number[], D: number[] }> {
    if (!this.collection) {
      throw new Error('ChromaDB collection not initialized');
    }
    
    const count = await this.collection.count();
    console.log(`=== ChromaDB Search Debug ===`);
    console.log(`Requested k: ${k}`);
    console.log(`Available vectors: ${count}`);
    
    if (count === 0) {
      console.log(`Database is empty, returning empty results`);
      return { I: [], D: [] };
    }
    
    const effectiveK = Math.min(k, count);
    console.log(`Using effective k: ${effectiveK}`);
    console.log(`=== End ChromaDB Search Debug ===`);
    
    const results = await this.collection.query({
      query_embeddings: [queryVector],
      n_results: effectiveK,
      include: ['metadatas', 'distances'] as any,
    });

    console.log('ChromaDB query results:', JSON.stringify(results, null, 2));

    // The metadatas property is a double array, so we need to flatten it.
    const metadatas = (results.metadatas ?? []).flat();
    const distances = (results.distances ?? []).flat();

    // Extract vector IDs from metadata for result mapping
    const vectorIds = metadatas.map((m: any) => m?.vectorId ?? 0);
    
    return {
      I: vectorIds,
      D: distances as number[]
    };
  }

  deleteVector(ids: number[]): void {
    // Store deletion requests for batch processing
    this.pendingDeletions = this.pendingDeletions || [];
    this.pendingDeletions.push(...ids);
  }

  private pendingDeletions?: number[];

  async processPendingDeletions(): Promise<void> {
    if (!this.collection || !this.pendingDeletions || this.pendingDeletions.length === 0) {
      return;
    }
    
    const allData = await this.collection.get();
    const idsToDelete = allData.ids.filter((id, index) => {
      const metadata = allData.metadatas?.[index] as any;
      return metadata?.vectorId && this.pendingDeletions?.includes(metadata.vectorId);
    });
    
    if (idsToDelete.length > 0) {
      await this.collection.delete({ ids: idsToDelete });
      await this.updateVectorCount();
    }
    
    this.pendingDeletions = [];
  }

  async save(): Promise<void> {
    console.log('ChromaDB automatically persists data.');
  }

  async rebuildIndex(): Promise<void> {
    if (!this.collection) {
      throw new Error('ChromaDB collection not initialized');
    }
    
    console.log('Rebuilding ChromaDB index from database...');
    const documents = this.databaseService.getAllDocuments();
    
    if (documents.length === 0) {
      console.log('No documents in database to rebuild index from.');
      return;
    }

    // Rebuild all mappings from database
    const allMappings: { vectorId: number; documentId: string }[] = [];
    const allEmbeddings: number[][] = [];
    let currentVectorIndex = 0;
    
    for (const doc of documents) {
      const chunks = this.chunkDocument(doc.content);
      if (chunks.length === 0) continue;
      
      const embeddings = await this.ollamaService.getEmbeddings(chunks);
      
      embeddings.forEach((embedding, index) => {
        allMappings.push({
          vectorId: currentVectorIndex + index,
          documentId: doc.id,
        });
      });
      
      allEmbeddings.push(...embeddings);
      currentVectorIndex += embeddings.length;
    }
    
    // Add all vectors with mappings
    if (allEmbeddings.length > 0) {
      await this.saveWithMappings(allEmbeddings, allMappings);
    }
    
    const count = await this.collection.count();
    console.log(`Index rebuilt with ${count} vectors in ChromaDB.`);
  }

  async load(): Promise<void> {
    await this.init();
  }

  /**
   * Chunks a document using the same logic as RAG service
   */
  private chunkDocument(text: string, chunkSize: number = 512): string[] {
    const sentences = cleanText(text).split(/(?<=[.!?])\s+/).filter(s => s.trim() !== '');

    if (sentences.length === 0) {
      return [];
    }

    const chunks: string[] = [];
    for (let i = 0; i < sentences.length; i++) {
      let currentChunk = sentences[i];
      let left = i - 1;
      let right = i + 1;

      while (currentChunk.length < chunkSize) {
        let expanded = false;

        if (right < sentences.length && currentChunk.length + sentences[right].length + 1 <= chunkSize) {
          currentChunk += ' ' + sentences[right];
          right++;
          expanded = true;
        }

        if (left >= 0 && currentChunk.length + sentences[left].length + 1 <= chunkSize) {
          currentChunk = sentences[left] + ' ' + currentChunk;
          left--;
          expanded = true;
        }

        if (!expanded) break;
      }

      chunks.push(currentChunk);
    }

    return [...new Set(chunks)];
  }
}