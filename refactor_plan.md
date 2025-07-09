# Plan to Refactor the RAG Service with SQLite

This plan outlines the steps to refactor the RAG (Retrieval-Augmented Generation) service to be more robust, scalable, and performant by integrating a SQLite database for metadata management and improving the data ingestion pipeline.

### 1. Introduce a Text Chunking Strategy

- **Problem:** Embedding entire documents dilutes semantic meaning, leading to poor retrieval quality.
- **Solution:** Break down documents into smaller, semantically coherent chunks before embedding.
- **Actions:**
  1.  Add the `langchain/text_splitter` library to `desktop-daemon/package.json`.
  2.  Use `RecursiveCharacterTextSplitter` within the `RAGService` to handle the chunking logic.

### 2. Replace File Stores with a Unified Database Service

- **Problem:** The current file-based storage for documents and mappings is slow, not transactional, and does not scale well.
- **Solution:** Create a new `DatabaseService` to manage all metadata (documents and vector mappings) in a central SQLite database.
- **Actions:**
  1.  Add the `better-sqlite3` and `@types/better-sqlite3` libraries to `desktop-daemon/package.json`.
  2.  Create a new file: `desktop-daemon/src/services/database.ts`.
  3.  This service will manage the SQLite connection and define the database schema with two primary tables:
      -   **`documents`**: `(id TEXT PRIMARY KEY, title TEXT, url TEXT, timestamp INTEGER, content TEXT)`
      -   **`vector_mappings`**: `(vector_id INTEGER PRIMARY KEY, document_id TEXT, FOREIGN KEY(document_id) REFERENCES documents(id))`
  4.  The `RAGService` will be refactored to use this new `DatabaseService`, removing the need for the old `DocumentStoreService` and the proposed `VectorDocumentMappingService`.

### 3. Rework Ingestion Logic for Batching and Transactions

- **Problem:** The data ingestion process must be atomic to prevent data corruption and should handle document batches efficiently.
- **Solution:** Wrap the entire ingestion process for a batch of documents within a single database transaction.
- **Actions:**
  1.  In `RAGService`, the `addDocuments` method's logic will be wrapped in a `db.transaction(() => { ... })` block.
  2.  The transaction will perform the following steps for each document:
      a. Insert the document metadata into the `documents` table.
      b. Chunk the document content.
      c. Generate embeddings for each chunk via the `OllamaService`.
      d. Add the resulting vectors to the `VectorStoreService` (FAISS).
      e. For each vector, insert a mapping row into the `vector_mappings` table, linking the vector's index to the parent document's ID.
  3.  **Error Handling:** The transaction ensures that if any step fails, all changes for that batch are automatically rolled back, maintaining data consistency.

### 4. Update Search and Persistence Logic

- **Problem:** The search and data persistence logic must be updated to align with the new database-centric architecture.
- **Solution:** Adapt the search logic to query the database for document retrieval and rely on SQLite for metadata persistence.
- **Actions:**
  1.  The `search` method in `RAGService` will first get relevant vector indices from the `VectorStoreService`. It will then query the `vector_mappings` table to find the parent `document_id`s and retrieve the full documents from the `documents` table.
  2.  A `saveAllStores` method will be kept in `RAGService`, but its responsibility will be reduced to only saving the FAISS index via `vectorStoreService.save()`. SQLite will handle the persistence of all other metadata automatically.

### 5. Update Ollama Service for Batching

- **Problem:** The `OllamaService` can only process one embedding request at a time.
- **Solution:** Add a method to handle batch embedding requests to encapsulate the iteration logic.
- **Actions:**
  1.  Create a new public method, `getEmbeddings(texts: string[]): Promise<number[][]>`, in the `OllamaService`. This will streamline the process of generating embeddings for multiple text chunks at once.

### Revised Architecture Diagram

```mermaid
graph TD
    subgraph "RAG Service: Data Ingestion (Single Transaction)"
        A[addDocuments] -- documents --> B(DatabaseService: BEGIN TRANSACTION);
        B -- doc --> C[SQLite: INSERT INTO documents];
        C -- content --> D{TextSplitter};
        D -- chunks --> E{OllamaService: getEmbeddings};
        subgraph "For Each Embedding"
            E -- embedding --> F[VectorStoreService: add];
            F -- vector_id --> G[SQLite: INSERT INTO vector_mappings];
        end
        G --> H(DatabaseService: COMMIT/ROLLBACK);
    end

    subgraph "RAG Service: Data Retrieval"
        I[search] -- query --> J[VectorStoreService: search];
        J -- vector_ids --> K[DatabaseService: SELECT document_id FROM vector_mappings];
        K -- document_ids --> L[DatabaseService: SELECT * FROM documents];
        L -- retrieved documents --> M((Output));
    end