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
import { v4 as uuidv4 } from 'uuid';
import { cleanText } from '../utils/textProcessor';

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

export type ProgressCallback = (status: SearchProgressStatus, message?: string, data?: any) => void;

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

    /**
     * Constructs a new RagService instance.
     * @param ollamaService The OllamaService instance.
     * @param vectorStoreService The VectorStoreService instance.
     * @param databaseService The DatabaseService instance.
     */
    public constructor(
        ollamaService: OllamaService,
        vectorStoreService: VectorStoreService,
        databaseService: DatabaseService
    ) {
        this.ollamaService = ollamaService;
        this.vectorStoreService = vectorStoreService;
        this.databaseService = databaseService;
    }

    /**
     * Splits a document into chunks using a sentence-based sliding window.
     * Each chunk is centered around a sentence and expanded outwards until
     * the chunk_size is reached, without breaking sentences.
     * @param text The input text to chunk.
     * @param chunkSize The desired size of each chunk.
     * @returns An array of text chunks.
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
                if (left >= 0) {
                    const newChunk = sentences[left] + " " + currentChunk;
                    if (newChunk.length <= chunkSize) {
                        currentChunk = newChunk;
                        left--;
                        expanded = true;
                    }
                }
                if (right < sentences.length && currentChunk.length < chunkSize) {
                    const newChunk = currentChunk + " " + sentences[right];
                    if (newChunk.length <= chunkSize) {
                        currentChunk = newChunk;
                        right++;
                        expanded = true;
                    }
                }
                
                if (!expanded) {
                    break; 
                }
            }    
            chunks.push(currentChunk);
            // console.log(`Chunk ${i}: ${currentChunk}`); // For debugging, match Python output
        }
        return chunks;
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

        console.log(`=== Search Debug: Raw Results ===`);
        console.log(`Total raw matches: ${searchResults.I.length}`);
        searchResults.I.forEach((index, i) => {
            console.log(`  Raw ${i}: chunkId=${index}, distance=${searchResults.D[i]}`);
        });
        console.log(`=================================`);

        if (searchResults.I.length === 0) {
            console.log('No results found in vector search');
            return [];
        }

        const candidates = searchResults.I
            .map((index, i) => ({ chunkId: index, distance: searchResults.D[i] }))
            .filter(item => item.distance <= SEARCH_DISTANCE_CUTOFF);

        console.log(`=== Search Debug: Candidates after cutoff ===`);
        console.log(`Candidates passing distance cutoff (${SEARCH_DISTANCE_CUTOFF}): ${candidates.length}`);
        
        if (candidates.length === 0) {
            console.log('No candidates passed distance cutoff');
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

        console.log(`=== Search Debug: Final Document Selection ===`);
        console.log(`Top ${docScores.length} documents selected:`);
        docScores.slice(0, 5).forEach((item, idx) => {
            console.log(`  ${idx + 1}. docId=${item.docId}, score=${item.score.toFixed(2)}, bestDistance=${item.bestChunk.distance}`);
        });
        console.log(`=================================`);

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

            console.log(`=== Search Debug: Final Chunk ===`);
            console.log(`  Document: "${document.title}" (id=${document.id})`);
            console.log(`  Distance: ${candidate.distance}`);
            console.log(`=================================`);

            return {
                chunkId,
                documentId: document.id,
                distance: candidate.distance,
                content: document.content,
                title: document.title,
                url: document.url,
                timestamp: document.timestamp
            };
        }).filter((chunk): chunk is RetrievedChunk => chunk !== null);

        console.log('getRankedChunks. Got '+hydratedChunks.length);
        return hydratedChunks;
    }

    /**
     * Searches for relevant documents and streams the AI response.
     * @param query The user's query string.
     * @param onProgress Callback for progress updates, including streaming the response.
     */
    public async searchAndStream(query: string, onProgress: ProgressCallback): Promise<void> {
        try {
            onProgress('starting', 'Starting search...');

            onProgress('retrieving', 'Retrieving relevant documents...');
            const retrievedDocuments = await this.getRankedChunks(query);
            onProgress('retrieving', 'Retrieved documents.', { documents: retrievedDocuments });

            if (retrievedDocuments.length === 0) {
                onProgress('complete', 'No relevant documents found');
                return;
            }

            const context = retrievedDocuments.map(doc => doc.content).join("\n\n");
            const prompt = `
            You are a helpful AI assistant.
            Answer the following question based on the provided context:

            Question: ${query}

            Context:
            ${context}

            Instructions:
            Be concise.
            Do not refer to the context. Do not refer to the provided information. Do not refer to the user or the user's request.
            Constrain your answers very strongly to the provided material and if you do need to refer to you your built-in knowledge tell the user where you have done so.
            Some of the provided material may not be relevant. If it's irrelevant, ignore it and do not refer to it.
            Never mention the context. The user is not reading the context.
            Never mention the user.
            Never mention the prompt. The user can't see the prompt.
            Never mention the prompt.
            Do not explain your thinking or reasoning, only provide the answer.
            `;
            const trimmedPrompt = prompt.trim().replace(/ {2,}/g, ' ');

            onProgress('generating', 'Building response...');
            const stream = await this.ollamaService.getCompletionStream(trimmedPrompt);

            let finalAnswer = '';
            for await (const chunk of stream) {
                finalAnswer += chunk;
                onProgress('generating', 'Streaming response...', { chunk });
            }
            
            onProgress('complete', 'Search complete.', { response: finalAnswer });
        } catch (error) {
            onProgress('error', 'Search failed');
            console.error('Error in searchAndStream:', error);
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

            const chunks = this.chunkDocument(newDocument.content);
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
                await this.saveAllStores();
            }
        }

        console.log('deleteDocument: deleted '+documentId)
        return deletedFromDb;
    }

    /**
     * Saves all stores to disk.
     */
    public async saveAllStores(): Promise<void> {
        await this.vectorStoreService.save(this.vectorStoreService.getFilePath());
    }
}