/**
 * RagService
 *
 * This service orchestrates the RAG pipeline,
 * integrating Ollama, VectorStore, and DocumentStore services
 * to process user queries and generate informed responses.
 */
import { OllamaService } from './ollama';
import { VectorStoreService } from './vectorStore';
import { DocumentStoreService, Document } from './documentStore';

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

export class RagService {
    private ollamaService: OllamaService;
    private vectorStoreService: VectorStoreService;
    private documentStoreService: DocumentStoreService;
    private vectorIdToDocumentIdMap: string[] = [];


    /**
     * Constructs a new RagService instance.
     * @param ollamaService The OllamaService instance.
     * @param vectorStoreService The VectorStoreService instance.
     * @param documentStoreService The DocumentStoreService instance.
     */
    private constructor(
        ollamaService: OllamaService,
        vectorStoreService: VectorStoreService,
        documentStoreService: DocumentStoreService
    ) {
        this.ollamaService = ollamaService;
        this.vectorStoreService = vectorStoreService;
        this.documentStoreService = documentStoreService;
    }

    private async initialize(): Promise<void> {
        this.vectorIdToDocumentIdMap = await this.documentStoreService.getIds();
    }

    public static async create(
        ollamaService: OllamaService,
        vectorStoreService: VectorStoreService,
        documentStoreService: DocumentStoreService
    ): Promise<RagService> {
        const ragService = new RagService(ollamaService, vectorStoreService, documentStoreService);
        await ragService.initialize();
        return ragService;
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
            const topKVectorIndices = await this.vectorStoreService.search(queryEmbedding, k);

            console.log(`=== RAG Search Debug ===`);
            console.log(`Query: "${query}"`);
            console.log(`Vector indices found: ${topKVectorIndices.I.length}`);
            console.log(`Indices: [${topKVectorIndices.I.join(', ')}]`);
            console.log(`=== End RAG Search Debug ===`);

            // Handle empty search results
            if (topKVectorIndices.I.length === 0) {
                onProgress?.('complete', 'No documents available in the knowledge base');
                return 'No documents available in the knowledge base. Please add some documents first.';
            }

            // 3. Map vector indices to document IDs
            const documentIdsToRetrieve = topKVectorIndices.I.map(index => this.vectorIdToDocumentIdMap[index]).filter(id => id);

            // 4. Retrieve documents from the document store
            onProgress?.('retrieving', 'Retrieving relevant documents...');
            const retrievedDocuments = await this.documentStoreService.getMany(documentIdsToRetrieve);

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
     * Adds a document to the RAG system.
     * @param doc The document to add, including title, content, and optional URL.
     */
    public async addDocument(doc: { title: string; content: string; url?: string }): Promise<void> {
        // 1. Add the document to the DocumentStoreService.
        const newDocument = { ...doc, url: doc.url || '', timestamp: Date.now() };
        const addedDocument = await this.documentStoreService.add(newDocument);
        this.vectorIdToDocumentIdMap.push(addedDocument.id);


        // 2. Generate an embedding for the document's content using the OllamaService.
        const embedding = await this.ollamaService.getEmbedding(addedDocument.content);

        // 3. Add the resulting embedding to the VectorStoreService.
        await this.vectorStoreService.add([embedding]);

        // 4. Ensure both the document store and the vector index are saved to disk after the addition.
        await this.documentStoreService.save();
        await this.vectorStoreService.save(this.vectorStoreService.getFilePath());
    }
}