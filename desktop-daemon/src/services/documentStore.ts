import { v4 as uuidv4 } from 'uuid';
import * as fs from 'fs/promises';
import * as path from 'path';
import { DocumentStoreConfig } from '../config';

/**
 * Represents a document stored in the DocumentStoreService.
 */
export type Document = {
  id: string;
  content: string;
  url: string;
  title: string;
  timestamp: number;
};

/**
 * Manages documents in a simple JSON file, acting as a key-value database.
 */
export class DocumentStoreService {
  private documents: Map<string, Document> = new Map();
  private filePath: string;

  /**
   * Constructs a new DocumentStoreService.
   * @param filePath The path to the JSON file where documents are stored.
   */
  constructor(filePath: string) {
    this.filePath = filePath;
  }

  /**
   * Adds a new document, generates a unique ID, and returns the full document.
   * @param document The document content to add, excluding the ID.
   * @returns A Promise that resolves to the full Document object with its generated ID.
   */
  async add(document: Omit<Document, 'id'>): Promise<Document> {
    const newDocument: Document = {
      id: uuidv4(),
      ...document,
    };
    this.documents.set(newDocument.id, newDocument);
    return newDocument;
  }

  /**
   * Retrieves a single document by its ID.
   * @param id The ID of the document to retrieve.
   * @returns A Promise that resolves to the Document if found, otherwise undefined.
   */
  async get(id: string): Promise<Document | undefined> {
    return this.documents.get(id);
  }

  /**
   * Retrieves multiple documents by their IDs.
   * @param ids An array of document IDs to retrieve.
   * @returns A Promise that resolves to an array of found Documents.
   */
  async getMany(ids: string[]): Promise<Document[]> {
    return ids.map(id => this.documents.get(id)).filter((doc): doc is Document => doc !== undefined);
  }

  /**
   * Retrieves all document IDs from the store.
   * @returns A Promise that resolves to an array of all document IDs.
   */
  async getIds(): Promise<string[]> {
    return Array.from(this.documents.keys());
  }

  /**
   * Saves the entire document database to the JSON file.
   * Creates the directory if it doesn't exist.
   * @returns A Promise that resolves when the save operation is complete.
   */
  async save(): Promise<void> {
    const dir = path.dirname(this.filePath);
    await fs.mkdir(dir, { recursive: true });
    const data = JSON.stringify(Array.from(this.documents.values()), null, 2);
    await fs.writeFile(this.filePath, data, 'utf8');
  }

  /**
   * Loads the database from the JSON file, creating it if it doesn't exist.
   * @returns A Promise that resolves when the load operation is complete.
   */
  async load(): Promise<void> {
    try {
      const data = await fs.readFile(this.filePath, 'utf8');
      const docsArray: Document[] = JSON.parse(data);
      this.documents = new Map(docsArray.map(doc => [doc.id, doc]));
    } catch (error: any) {
      if (error.code === 'ENOENT') {
        // File does not exist, initialize with empty map and save
        this.documents = new Map();
        await this.save();
      } else {
        throw error;
      }
    }
  }
}

export const documentStoreService = new DocumentStoreService(DocumentStoreConfig.documentStoreFile);