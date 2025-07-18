import Database from 'better-sqlite3';
import * as path from 'path';

/**
 * Represents a document stored in the database.
 */
export type Document = {
  id: string;
  content: string;
  url: string;
  title: string;
  timestamp: number;
};

/**
 * Represents a mapping between a vector index and a document ID.
 */
export type VectorMapping = {
  vectorId: number;
  documentId: string;
};

/**
 * Manages the SQLite database for documents and vector mappings.
 */
export class DatabaseService {
  private db: Database.Database;

  /**
   * Constructs a new DatabaseService instance.
   * @param dbPath The path to the SQLite database file.
   */
  constructor(dbPath: string) {
    this.db = new Database(dbPath);
    this.initializeSchema();
  }

  /**
   * Initializes the database schema, creating tables if they do not exist.
   */
  private initializeSchema(): void {
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS documents (
        id TEXT PRIMARY KEY,
        title TEXT,
        url TEXT,
        timestamp INTEGER,
        content TEXT
      );

      CREATE TABLE IF NOT EXISTS vector_mappings (
        vector_id INTEGER PRIMARY KEY,
        document_id TEXT,
        FOREIGN KEY(document_id) REFERENCES documents(id)
      );
    `);
  }

  /**
   * Inserts a single document into the documents table.
   * @param document The document to insert.
   */
  insertDocument(document: Document): void {
    const stmt = this.db.prepare(`
      INSERT INTO documents (id, title, url, timestamp, content)
      VALUES (?, ?, ?, ?, ?)
    `);
    stmt.run(document.id, document.title, document.url, document.timestamp, document.content);
  }

  /**
   * Inserts multiple vector mappings into the vector_mappings table.
   * @param mappings An array of vector mappings to insert.
   */
  insertVectorMappings(mappings: VectorMapping[]): void {
    const stmt = this.db.prepare(`
      INSERT INTO vector_mappings (vector_id, document_id)
      VALUES (?, ?)
    `);
    this.db.transaction(() => {
      for (const mapping of mappings) {
        stmt.run(mapping.vectorId, mapping.documentId);
      }
    })();
  }

  /**
   * Retrieves a single document by its ID.
   * @param id The ID of the document to retrieve.
   * @returns The Document if found, otherwise undefined.
   */
  getDocumentById(id: string): Document | undefined {
    const stmt = this.db.prepare(`SELECT * FROM documents WHERE id = ?`);
    return stmt.get(id) as Document | undefined;
  }

  /**
   * Retrieves a single document by its URL.
   * @param url The URL of the document to retrieve.
   * @returns The Document if found, otherwise undefined.
   */
  getDocumentByUrl(url: string): Document | undefined {
    const stmt = this.db.prepare(`SELECT * FROM documents WHERE url = ?`);
    return stmt.get(url) as Document | undefined;
  }

  /**
   * Retrieves multiple documents by their IDs.
   * @param ids An array of document IDs to retrieve.
   * @returns An array of found Documents.
   */
  getDocumentsByIds(ids: string[]): Document[] {
    if (ids.length === 0) {
      return [];
    }
    const placeholders = ids.map(() => '?').join(',');
    const stmt = this.db.prepare(`SELECT * FROM documents WHERE id IN (${placeholders})`);
    return stmt.all(...ids) as Document[];
  }

  /**
   * Retrieves all documents from the database.
   * @returns An array of all documents.
   */
  getAllDocuments(): Document[] {
    const stmt = this.db.prepare(`SELECT * FROM documents`);
    return stmt.all() as Document[];
  }

  /**
   * Retrieves the document ID for a given vector ID.
   * @param vectorId The ID of the vector.
   * @returns The document ID if found, otherwise undefined.
   */
  getDocumentIdByVectorId(vectorId: number): string | undefined {
    const stmt = this.db.prepare(`SELECT document_id FROM vector_mappings WHERE vector_id = ?`);
    const result = stmt.get(vectorId) as { document_id: string } | undefined;
    return result?.document_id;
  }

  /**
   * Retrieves document IDs for a given array of vector IDs.
   * @param vectorIds An array of vector IDs.
   * @returns An array of document IDs.
   */
  getDocumentIdsByVectorIds(vectorIds: number[]): string[] {
    if (vectorIds.length === 0) {
      return [];
    }
    const placeholders = vectorIds.map(() => '?').join(',');
    const stmt = this.db.prepare(`SELECT document_id FROM vector_mappings WHERE vector_id IN (${placeholders})`);
    const results = stmt.all(...vectorIds) as { document_id: string }[];
    return results.map(row => row.document_id);
  }

  /**
   * Retrieves vector IDs associated with a given document ID.
   * @param documentId The ID of the document.
   * @returns An array of vector IDs.
   */
  getVectorIdsByDocumentId(documentId: string): number[] {
    const stmt = this.db.prepare(`SELECT vector_id FROM vector_mappings WHERE document_id = ?`);
    const results = stmt.all(documentId) as { vector_id: number }[];
    return results.map(row => row.vector_id);
  }

  /**
   * Retrieves vector mappings for a given array of vector IDs.
   * @param vectorIds An array of vector IDs.
   * @returns An array of vector mappings.
   */
  getVectorMappingsByIds(vectorIds: number[]): VectorMapping[] {
    if (vectorIds.length === 0) {
      return [];
    }
    const placeholders = vectorIds.map(() => '?').join(',');
    const stmt = this.db.prepare(`SELECT vector_id, document_id FROM vector_mappings WHERE vector_id IN (${placeholders})`);
    const results = stmt.all(...vectorIds) as { vector_id: number; document_id: string }[];
    return results.map(row => ({ vectorId: row.vector_id, documentId: row.document_id }));
  }

  /**
   * Deletes a document and its associated vector mappings from the database.
   * @param documentId The ID of the document to delete.
   * @returns True if the document was deleted, false otherwise.
   */
  deleteDocument(documentId: string): boolean {
    const deleteMappingsStmt = this.db.prepare(`DELETE FROM vector_mappings WHERE document_id = ?`);
    const deleteDocStmt = this.db.prepare(`DELETE FROM documents WHERE id = ?`);

    const result = this.db.transaction(() => {
      deleteMappingsStmt.run(documentId);
      const deleteDocResult = deleteDocStmt.run(documentId);
      return deleteDocResult;
    })();

    return result.changes > 0;
  }

  /**
   * Updates an existing document's title and content.
   * @param id The ID of the document to update.
   * @param title The new title for the document.
   * @param content The new content for the document.
   * @returns True if the document was updated, false otherwise.
   */
  updateDocument(id: string, title: string, content: string): boolean {
    const stmt = this.db.prepare(`
      UPDATE documents
      SET title = ?, content = ?, timestamp = ?
      WHERE id = ?
    `);
    const result = stmt.run(title, content, Date.now(), id);
    return result.changes > 0;
  }

  /**
   * Retrieves a paginated list of recent documents, ordered by timestamp descending.
   * @param limit The maximum number of documents to retrieve.
   * @param offset The number of documents to skip.
   * @returns An array of recent Documents.
   */
  getRecentDocuments(limit: number, offset: number): Document[] {
    const stmt = this.db.prepare(`
      SELECT * FROM documents
      ORDER BY timestamp DESC
      LIMIT ? OFFSET ?
    `);
    return stmt.all(limit, offset) as Document[];
  }

  /**
   * Begins a database transaction.
   * @param fn The function to execute within the transaction.
   */
  transaction<T>(fn: (...args: any[]) => T): (...args: any[]) => T {
    return this.db.transaction(fn);
  }

  /**
   * Closes the database connection.
   */
  close(): void {
    this.db.close();
  }
}