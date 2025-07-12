import faiss
import numpy as np
import requests
import sqlite3
from pathlib import Path
import sys


# --- Configuration (should match your faiss-node script) ---
OLLAMA_API_URL = 'http://localhost:11434'
EMBEDDING_MODEL = 'mahonzhan/all-MiniLM-L6-v2'
INDEX_DIR = Path.home() / '.localmind'
INDEX_PATH = INDEX_DIR / 'localmind.index'
DB_PATH = INDEX_DIR / 'localmind.db' # Assuming the database is also in .localmind
DIMENSION = 384 # Must be the same dimension as the index was created with.
K = 10

def get_embedding(text: str) -> np.ndarray:
    """
    Gets an embedding for the given text from the configured embedding model using Ollama API.
    """
    try:
        response = requests.post(
            f"{OLLAMA_API_URL}/api/embeddings",
            json={
                "model": EMBEDDING_MODEL,
                "prompt": text,
            }
        )
        response.raise_for_status()
        embedding = response.json().get("embedding")
        if embedding:
            return np.array(embedding).astype(np.float32)
        else:
            raise ValueError("Invalid embedding response from Ollama API")
    except requests.exceptions.RequestException as e:
        print(f"Error getting embedding from Ollama API: {e}")
        raise

def get_document_titles_by_vector_ids(vector_ids: list[int]) -> dict[int, str]:
    """
    Retrieves document titles from the SQLite database based on vector IDs.
    """
    conn = None
    try:
        conn = sqlite3.connect(str(DB_PATH))
        cursor = conn.cursor()

        # First, get document_ids from vector_mappings table
        placeholders = ','.join('?' for _ in vector_ids)
        cursor.execute(f"SELECT vector_id, document_id FROM vector_mappings WHERE vector_id IN ({placeholders})", vector_ids)
        vector_mappings = cursor.fetchall()

        doc_ids = list(set([mapping[1] for mapping in vector_mappings]))
        if not doc_ids:
            return {}

        # Then, get title from documents table
        doc_placeholders = ','.join('?' for _ in doc_ids)
        cursor.execute(f"SELECT id, title FROM documents WHERE id IN ({doc_placeholders})", doc_ids)
        documents = cursor.fetchall()

        doc_title_map = {doc[0]: doc[1] for doc in documents}
        
        # Map vector_id to title
        result_map = {mapping[0]: doc_title_map.get(mapping[1], "Title not found") for mapping in vector_mappings}
        return result_map

    except sqlite3.Error as e:
        print(f"SQLite error: {e}")
        return {}
    finally:
        if conn:
            conn.close()

def main():
    # --- Query Handling ---
    if len(sys.argv) > 1:
        query_text = sys.argv[1]
    else:
        print("Usage: python debug_embeddings.py <query_string>")
        query_text = input("\nEnter a query to search for (or type 'exit'): ")
        if query_text.lower() == 'exit':
            return

    print(f"Loading Faiss index from: {INDEX_PATH}")
    try:
        # --- Load the index created by faiss-node ---
        index = faiss.read_index(str(INDEX_PATH))
        print(f"Index loaded successfully. It contains {index.ntotal} vectors with dimension {index.d}.")

    except Exception as e:
        print(f"Could not load index. Error: {e}")
        print("Please run the Node.js script first to generate the index file.")
        return

    # 1. Get the embedding for the query
    query_vector = get_embedding(query_text)
    print(f"Query vector dimension: {query_vector.shape[0]}")
    # Faiss expects a 2D array for searching (batch of 1)
    query_vector_batch = np.array([query_vector]).astype('float32')


    # 2. Perform the search
    # The search method returns distances and labels (the original IDs)
    distances, labels = index.search(query_vector_batch, K)

    # 3. Retrieve original titles from SQLite
    result_vector_ids = labels[0].tolist()
    id_to_title_mapping = get_document_titles_by_vector_ids(result_vector_ids)

    # 4. Display results
    print("\n--- Search Results ---")
    for i in range(K):
        result_id = labels[0][i]
        distance = distances[0][i]
        original_title = id_to_title_mapping.get(result_id, "Title not found")
        print(f"ID: {result_id}, Distance: {distance:.4f}")
        print(f"  Title: \"{original_title}\"")


if __name__ == "__main__":
    main()