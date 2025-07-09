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
     * Gets immediate vector search results without LLM processing.
     * @param query The user's query string.
     * @returns A promise that resolves to an array of vector search results.
     */
    public async getVectorResults(query: string): Promise<VectorSearchResult[]> {
        const queryEmbedding = await this.ollamaService.getEmbedding(query);
        const k = 10;
        const searchResults = await this.vectorStoreService.search(queryEmbedding, k);

        if (searchResults.I.length === 0) {
            return [];
        }

        const filteredIndices = searchResults.I
            .map((index, i) => ({ index, distance: searchResults.D[i] }))
            .filter(item => item.distance <= SEARCH_DISTANCE_CUTOFF)
            .map(item => item.index);

        const documentIdsToRetrieve = this.databaseService.getDocumentIdsByVectorIds(filteredIndices);

        const retrievedDocuments = this.databaseService.getDocumentsByIds(documentIdsToRetrieve);

        return retrievedDocuments.map(doc => ({
            id: doc.id,
            title: doc.title,
            url: doc.url,
            timestamp: doc.timestamp
        }));
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
            
            // 1. Get the embedding for the user's query using the OllamaService.
            onProgress?.('embedding', 'Processing query...');
            const queryEmbedding = await this.ollamaService.getEmbedding(query);

            // 2. Use the VectorStoreService to search for the top-k (e.g., k=5) most similar document vectors.
            onProgress?.('searching', 'Searching knowledge base...');
            const k = 5;
            const searchResults = await this.vectorStoreService.search(queryEmbedding, k);


            // Handle empty search results
            if (searchResults.I.length === 0) {
                onProgress?.('complete', 'No documents available in the knowledge base');
                return 'No documents available in the knowledge base. Please add some documents first.';
            }

            // 3. Map vector indices to document IDs
            const filteredIndices = searchResults.I
                .map((index, i) => ({ index, distance: searchResults.D[i] }))
                .filter(item => item.distance <= SEARCH_DISTANCE_CUTOFF)
                .map(item => item.index);
            
            const documentIdsToRetrieve = this.databaseService.getDocumentIdsByVectorIds(filteredIndices);

            // 4. Retrieve documents from the document store
            onProgress?.('retrieving', 'Retrieving relevant documents...');
            const retrievedDocuments = this.databaseService.getDocumentsByIds(documentIdsToRetrieve);

            const context = retrievedDocuments.map(doc => doc.content).join("\n\n");

            if (retrievedDocuments.length === 0) {
                onProgress?.('complete', 'No relevant documents found');
                return 'No relevant documents found.';
            }

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
    }

    /**
     * Saves all stores to disk.
     */
    public async saveAllStores(): Promise<void> {
        await this.vectorStoreService.save(this.vectorStoreService.getFilePath());
    }
}