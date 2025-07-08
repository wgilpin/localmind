"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.RagService = void 0;
class RagService {
    /**
     * Constructs a new RagService instance.
     * @param ollamaService The OllamaService instance.
     * @param vectorStoreService The VectorStoreService instance.
     * @param documentStoreService The DocumentStoreService instance.
     */
    constructor(ollamaService, vectorStoreService, documentStoreService) {
        this.ollamaService = ollamaService;
        this.vectorStoreService = vectorStoreService;
        this.documentStoreService = documentStoreService;
    }
    /**
     * Searches for relevant documents and generates a completion based on the query and retrieved context.
     * @param query The user's query string.
     * @returns A promise that resolves to the generated answer string.
     */
    search(query) {
        return __awaiter(this, void 0, void 0, function* () {
            // 1. Get the embedding for the user's query using the OllamaService.
            const queryEmbedding = yield this.ollamaService.getEmbedding(query);
            // 2. Use the VectorStoreService to search for the top-k (e.g., k=5) most similar document vectors.
            const k = 5;
            const topKVectorIndices = yield this.vectorStoreService.search(queryEmbedding, k);
            // 3. Map these indices back to document IDs. Assume a simple 1:1 mapping for now.
            // The vector index corresponds to the document's position in an array or a list.
            // For simplicity, we'll use the indices directly as document IDs for retrieval.
            const documentIdsToRetrieve = topKVectorIndices.I.map(index => index.toString());
            // 4. Retrieve the full content of the top-k documents using the DocumentStoreService.
            const retrievedDocuments = [];
            for (const docId of documentIdsToRetrieve) {
                const doc = yield this.documentStoreService.get(docId);
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
            const finalAnswer = yield this.ollamaService.getCompletion(trimmedPrompt);
            // 7. Return the generated answer.
            return finalAnswer;
        });
    }
    /**
     * Adds a document to the RAG system.
     * @param doc The document to add, including title, content, and optional URL.
     */
    addDocument(doc) {
        return __awaiter(this, void 0, void 0, function* () {
            // 1. Add the document to the DocumentStoreService.
            const newDocument = Object.assign(Object.assign({}, doc), { url: doc.url || '', timestamp: Date.now() });
            const addedDocument = yield this.documentStoreService.add(newDocument);
            // 2. Generate an embedding for the document's content using the OllamaService.
            const embedding = yield this.ollamaService.getEmbedding(addedDocument.content);
            // 3. Add the resulting embedding to the VectorStoreService.
            yield this.vectorStoreService.add([embedding]);
            // 4. Ensure both the document store and the vector index are saved to disk after the addition.
            yield this.documentStoreService.save();
            yield this.vectorStoreService.save(this.vectorStoreService.getFilePath());
        });
    }
}
exports.RagService = RagService;
