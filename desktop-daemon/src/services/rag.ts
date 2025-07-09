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

export class RagService {
    private ollamaService: OllamaService;
    private vectorStoreService: VectorStoreService;
    private documentStoreService: DocumentStoreService;

    /**
     * Constructs a new RagService instance.
     * @param ollamaService The OllamaService instance.
     * @param vectorStoreService The VectorStoreService instance.
     * @param documentStoreService The DocumentStoreService instance.
     */
    constructor(
        ollamaService: OllamaService,
        vectorStoreService: VectorStoreService,
        documentStoreService: DocumentStoreService
    ) {
        this.ollamaService = ollamaService;
        this.vectorStoreService = vectorStoreService;
        this.documentStoreService = documentStoreService;
    }

    /**
     * Searches for relevant documents and generates a completion based on the query and retrieved context.
     * @param query The user's query string.
     * @returns A promise that resolves to the generated answer string.
     */
    public async search(query: string): Promise<string> {
        // 1. Get the embedding for the user's query using the OllamaService.
        const queryEmbedding = await this.ollamaService.getEmbedding(query);

        // 2. Use the VectorStoreService to search for the top-k (e.g., k=5) most similar document vectors.
        const k = 5;
        const topKVectorIndices = await this.vectorStoreService.search(queryEmbedding, k);

        console.log(`=== RAG Search Debug ===`);
        console.log(`Query: "${query}"`);
        console.log(`Vector indices found: ${topKVectorIndices.I.length}`);
        console.log(`Indices: [${topKVectorIndices.I.join(', ')}]`);
        console.log(`=== End RAG Search Debug ===`);

        // Handle empty search results
        if (topKVectorIndices.I.length === 0) {
            return 'No documents available in the knowledge base. Please add some documents first.';
        }

        // 3. Map these indices back to document IDs. Assume a simple 1:1 mapping for now.
        // The vector index corresponds to the document's position in an array or a list.
        // For simplicity, we'll use the indices directly as document IDs for retrieval.
        const documentIdsToRetrieve = topKVectorIndices.I.map(index => index.toString());

        // 4. Retrieve the full content of the top-k documents using the DocumentStoreService.
        const retrievedDocuments: Document[] = [];
        for (const docId of documentIdsToRetrieve) {
            const doc = await this.documentStoreService.get(docId);
            if (doc) {
                retrievedDocuments.push(doc);
            }
        }

        const context = retrievedDocuments.map(doc => doc.content).join("\n\n");

        if (retrievedDocuments.length === 0) {
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
        `;

        const trimmedPrompt = prompt.trim().replace(/ {2,}/g, ' ');

        // 6. Use the OllamaService's getCompletion method to get the final answer.
        const finalAnswer = await this.ollamaService.getCompletion(trimmedPrompt);

        // 7. Return the generated answer.
        return finalAnswer;
    }
    /**
     * Adds a document to the RAG system.
     * @param doc The document to add, including title, content, and optional URL.
     */
    public async addDocument(doc: { title: string; content: string; url?: string }): Promise<void> {
        // 1. Add the document to the DocumentStoreService.
        const newDocument = { ...doc, url: doc.url || '', timestamp: Date.now() };
        const addedDocument = await this.documentStoreService.add(newDocument);

        // 2. Generate an embedding for the document's content using the OllamaService.
        const embedding = await this.ollamaService.getEmbedding(addedDocument.content);

        // 3. Add the resulting embedding to the VectorStoreService.
        await this.vectorStoreService.add([embedding]);

        // 4. Ensure both the document store and the vector index are saved to disk after the addition.
        await this.documentStoreService.save();
        await this.vectorStoreService.save(this.vectorStoreService.getFilePath());
    }
}