/**
 * RagService
 *
 * This service orchestrates the RAG pipeline,
 * integrating Ollama, VectorStore, and DocumentStore services
 * to process user queries and generate informed responses.
 */
import { OllamaService } from './ollama';
import { VectorStoreService } from './vectorStore';
import { DatabaseService, Document } from './database';
import { RecursiveCharacterTextSplitter } from 'langchain/text_splitter';
import { v4 as uuidv4 } from 'uuid';
import * as path from 'path';
import { DocumentStoreConfig } from '../config'; // Using DocumentStoreConfig for dbPath

const SEARCH_DISTANCE_CUTOFF = 55.0;

export type SearchProgressStatus =
  | 'idle'
  | 'starting'
  | 'embedding'
  | 'searching'
  | 'retrieving'
  | 'generating'
  | 'complete'
  | 'error';

export type ProgressCallback = (status: SearchProgressStatus, message?: string) => void;

export type VectorSearchResult = {
    id: string;
    title: string;
    url?: string;
    timestamp: number;
};

export type SearchResult = {
    vectorResults: VectorSearchResult[];
    llmResult?: string;
};

export type RetrievedChunk = {
    chunkId: number;
    documentId: string;
    distance: number;
    content: string;
    title: string;
    url: string;
    timestamp: number;
  };

/**
 * RagService
 *
 * This service orchestrates the RAG pipeline,
 * integrating Ollama, VectorStore, and Database services
 * to process user queries and generate informed responses.
 */
export class RagService {
    private ollamaService: OllamaService;
    private vectorStoreService: VectorStoreService;
    private databaseService: DatabaseService;
    private textSplitter: RecursiveCharacterTextSplitter;

    /**
     * Constructs a new RagService instance.
     * @param ollamaService The OllamaService instance.
     * @param vectorStoreService The VectorStoreService instance.
     * @param databaseService The DatabaseService instance.
     * @param textSplitter The RecursiveCharacterTextSplitter instance.
     */
    private constructor(
        ollamaService: OllamaService,
        vectorStoreService: VectorStoreService,
        databaseService: DatabaseService,
        textSplitter: RecursiveCharacterTextSplitter
    ) {
        this.ollamaService = ollamaService;
        this.vectorStoreService = vectorStoreService;
        this.databaseService = databaseService;
        this.textSplitter = textSplitter;
    }

    public static async create(
        ollamaService: OllamaService,
        vectorStoreService: VectorStoreService,
    ): Promise<RagService> {
        const dbPath = path.join(DocumentStoreConfig.documentStoreFile, '..', 'localmind.db');
        const databaseService = new DatabaseService(dbPath);
        const textSplitter = new RecursiveCharacterTextSplitter({
            chunkSize: 1000,
            chunkOverlap: 200,
        });
        const ragService = new RagService(ollamaService, vectorStoreService, databaseService, textSplitter);
        return ragService;
    }

    /**
     * Searches for and re-ranks chunks to ensure relevance and diversity.
     * @param query The user's query string.
     * @returns A promise that resolves to an array of the most relevant text chunks.
     */
    public async getRankedChunks(query: string): Promise<RetrievedChunk[]> {
        // 1. Fetch a larger pool of candidate chunks (not documents)
        const k = 100; // Retrieve more to re-rank
        const queryEmbedding = await this.ollamaService.getEmbedding(query);
        const searchResults = await this.vectorStoreService.search(queryEmbedding, k);

        if (searchResults.I.length === 0) {
            return [];
        }

        const candidates = searchResults.I
            .map((index, i) => ({ chunkId: index, distance: searchResults.D[i] }))
            .filter(item => item.distance <= SEARCH_DISTANCE_CUTOFF);

        if (candidates.length === 0) {
            return [];
        }

        // 2. Group chunks by their parent document ID
        const vectorIds = candidates.map(c => c.chunkId);
        const mappings = this.databaseService.getVectorMappingsByIds(vectorIds);
        
        const candidatesWithDocIds = candidates.map(candidate => {
            const mapping = mappings.find(m => m.vectorId === candidate.chunkId);
            return {
                ...candidate,
                documentId: mapping ? mapping.documentId : 'unknown'
            };
        });

        const docsWithChunks: Map<string, any[]> = new Map();
        for (const chunk of candidatesWithDocIds) {
            if (!docsWithChunks.has(chunk.documentId)) {
                docsWithChunks.set(chunk.documentId, []);
            }
            docsWithChunks.get(chunk.documentId)!.push(chunk);
        }

        // 3. Calculate an aggregate score for each document
        const docScores = [];
        for (const [docId, chunks] of docsWithChunks.entries()) {
            const bestChunk = chunks.reduce((prev, curr) => curr.distance < prev.distance ? curr : prev);
            const numHits = chunks.length;
            
            // Scoring: lower is better (distance-based). Add a penalty for fewer hits.
            const score = bestChunk.distance - (0.1 * Math.log(1 + numHits)); // Example heuristic
            
            docScores.push({ docId, score, bestChunk });
        }

        // 4. Sort documents by the new score and take the top N
        docScores.sort((a, b) => a.score - b.score);
        const finalTopChunksIds = docScores.slice(0, 5).map(item => item.bestChunk.chunkId); // Return best chunk from top 5 docs

        // 5. Hydrate the chunk data with full document info for context/citation
        const finalMappings = this.databaseService.getVectorMappingsByIds(finalTopChunksIds);
        const documentIdsToRetrieve = [...new Set(finalMappings.map(m => m.documentId))];
        const retrievedDocuments = this.databaseService.getDocumentsByIds(documentIdsToRetrieve);
        
        const hydratedChunks = finalTopChunksIds.map(chunkId => {
            const mapping = finalMappings.find(m => m.vectorId === chunkId);
            if (!mapping) return null;

            const document = retrievedDocuments.find(d => d.id === mapping.documentId);
            if (!document) return null;

            const candidate = candidates.find(c => c.chunkId === chunkId);
            if (!candidate) return null;

            return {
                chunkId,
                documentId: document.id,
                distance: candidate.distance,
                content: document.content, // This is the whole doc content, need to change to chunk content later
                title: document.title,
                url: document.url,
                timestamp: document.timestamp
            };
        }).filter((chunk): chunk is RetrievedChunk => chunk !== null);

        return hydratedChunks;
    }

    /**
     * Searches for relevant documents and generates a completion based on the query and retrieved context.
     * @param query The user's query string.
     * @param onProgress Optional callback for progress updates.
     * @returns A promise that resolves to the generated answer string.
     */
    public async search(query: string, onProgress?: ProgressCallback): Promise<string> {
        try {
            onProgress?.('starting', 'Starting search...');
            
            onProgress?.('retrieving', 'Retrieving relevant documents...');
            const retrievedDocuments = await this.getRankedChunks(query);

            if (retrievedDocuments.length === 0) {
                onProgress?.('complete', 'No relevant documents found');
                return 'No relevant documents found.';
            }

            const context = retrievedDocuments.map(doc => doc.content).join("\n\n");

            // 5. Construct a detailed prompt for the completion model.
            // This prompt should include the retrieved document content as context and the original user query.
            const prompt = `
            You are a helpful AI assistant.
            Answer the following question based on the provided context:

            Question: ${query}

            Context:
            ${context}

            Instructions:
            Be concise.
            Do not refer to the context or the provided information .
            Constrain your answers very strongly to the provided material and if you do need to refer to you your built-in knowledge tell the user where you have done so.
            `;

            const trimmedPrompt = prompt.trim().replace(/ {2,}/g, ' ');

            // 6. Use the OllamaService's getCompletion method to get the final answer.
            onProgress?.('generating', 'Building response...');
            const finalAnswer = await this.ollamaService.getCompletion(trimmedPrompt);

            // 7. Return the generated answer.
            onProgress?.('complete', 'Search complete');
            return finalAnswer;
        } catch (error) {
            onProgress?.('error', 'Search failed');
            throw error;
        }
    }
    /**
     * Adds documents to the RAG system in a batch.
     * @param docs An array of documents to add, including title, content, and optional URL.
     */
    public async addDocuments(docs: { title: string; content: string; url?: string }[]): Promise<void> {
        const documentsToAdd: Document[] = [];
        const allEmbeddings: number[][] = [];
        const allMappings: { vectorId: number; documentId: string }[] = [];

        let currentVectorIndex = this.vectorStoreService.ntotal();

        for (const doc of docs) {
            const newDocument: Document = { ...doc, id: uuidv4(), url: doc.url || '', timestamp: Date.now() };
            documentsToAdd.push(newDocument);

            const chunks = await this.textSplitter.splitText(newDocument.content);
            if (chunks.length === 0) continue;

            const embeddings: number[][] = await this.ollamaService.getEmbeddings(chunks);
            
            embeddings.forEach((_embedding, index) => {
                allMappings.push({
                    vectorId: currentVectorIndex + index,
                    documentId: newDocument.id,
                });
            });
            allEmbeddings.push(...embeddings);
            currentVectorIndex += embeddings.length;
        }

        this.databaseService.transaction(() => {
            documentsToAdd.forEach(doc => this.databaseService.insertDocument(doc));
            if (allEmbeddings.length > 0) {
                this.vectorStoreService.add(allEmbeddings);
            }
            this.databaseService.insertVectorMappings(allMappings);
        })();
        await this.saveAllStores();
    }

    /**
     * Deletes a document and its associated vector entries from the RAG system.
     * @param documentId The ID of the document to delete.
     * @returns A promise that resolves to true if the document was deleted, false otherwise.
     */
    public async deleteDocument(documentId: string): Promise<boolean> {
        const vectorIds = this.databaseService.getVectorIdsByDocumentId(documentId);
        const deletedFromDb = this.databaseService.deleteDocument(documentId);
    
        if (deletedFromDb) {
            if (vectorIds.length > 0) {
                this.vectorStoreService.deleteVector(vectorIds);
                await this.saveAllStores(); // Save the vector store after deletion
            }
        }
    
        return deletedFromDb;
    }

    /**
     * Saves all stores to disk.
     */
    public async saveAllStores(): Promise<void> {
        await this.vectorStoreService.save(this.vectorStoreService.getFilePath());
    }
}