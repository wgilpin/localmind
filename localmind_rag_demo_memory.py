# %% [markdown]
# # LocalMind RAG Demo (Memory-Only)
#
# This notebook demonstrates LocalMind's RAG functionality entirely in memory:
# - Embedding generation using sentence-transformers
# - In-memory FAISS index creation and similarity search
# - Document chunking and distance calculations
# - No disk I/O - everything stays in RAM

# %%
import numpy as np
from sentence_transformers import SentenceTransformer
from sklearn.metrics.pairwise import cosine_distances
import re
from typing import List, Dict
from dataclasses import dataclass
print("Loaded required libraries...")

# %% [markdown]
# ## Configuration and Setup


# %%
@dataclass
class RAGConfig:
    """Configuration for RAG operations."""

    model_name: str = "all-MiniLM-L6-v2"
    embedding_dim: int = 384
    chunk_size: int = 512
    chunk_overlap: int = 50


config = RAGConfig()

# Initialize embedding model
model = SentenceTransformer(config.model_name)
print(f"Loaded model: {config.model_name}")
print(f"Embedding dimension: {config.embedding_dim}")

# %% [markdown]
# ## Document Processing Functions


# %%
def clean_text(text: str) -> str:
    """Clean and normalize text content."""
    text = re.sub(r"\s+", " ", text)
    text = text.strip()
    return text


def chunk_document(text: str, chunk_size: int = 512) -> List[str]:
    """
    Splits a document into chunks using a sentence-based sliding window.
    Each chunk is centered around a sentence and expanded outwards until
    the chunk_size is reached, without breaking sentences.
    """
    sentences = re.split(r'(?<=[.!?])\s+', text)
    sentences = [s.strip() for s in sentences if s.strip()]

    if not sentences:
        return []

    chunks = []
    for i in range(len(sentences)):
        current_chunk = sentences[i]
        left = i - 1
        right = i + 1

        while len(current_chunk) < chunk_size:
            if left >= 0:
                current_chunk = sentences[left] + " " + current_chunk
                left -= 1
                if len(current_chunk) >= chunk_size:
                    break
            if right < len(sentences) and len(current_chunk) < chunk_size:
                current_chunk += " " + sentences[right]
                right += 1
            if left < 0 and right >= len(sentences):
                break    
            
        chunks.append(current_chunk)
        print(f"Chunk {i}: {current_chunk}")

    return chunks


def get_embeddings(texts: List[str]) -> np.ndarray:
    """Generate embeddings for a list of texts."""
    embeddings = model.encode(texts, normalize_embeddings=True)
    return embeddings.astype(np.float32)


# %% [markdown]
# ## In-Memory FAISS Document Store


# %%
class InMemoryDocumentStore:
    """In-memory document store for similarity search."""

    def __init__(self, embedding_dim: int = 384):
        self.embedding_dim = embedding_dim
        self.embeddings = None
        self.documents = []

    def add_documents(self, documents: List[Dict], embeddings: np.ndarray):
        """Add documents and their embeddings to memory."""
        self.documents.extend(documents)
        if self.embeddings is None:
            self.embeddings = embeddings
        else:
            self.embeddings = np.vstack([self.embeddings, embeddings])

    def search(self, query: str, top_k: int = 5) -> List[Dict]:
        """Search for similar documents in memory."""
        if self.embeddings is None:
            return []

        query_embedding = get_embeddings([query])

        # Calculate cosine similarities manually
        similarities = np.dot(self.embeddings, query_embedding.T).flatten()

        # Get top-k indices
        top_indices = np.argsort(similarities)[-top_k:][::-1]

        results = []
        for idx in top_indices:
            if idx < len(self.documents):
                doc = self.documents[idx].copy()
                doc["score"] = float(similarities[idx])
                results.append(doc)

        return results

    def get_document_count(self) -> int:
        """Get total number of documents in memory."""
        return len(self.documents)


# %% [markdown]
# ## Sample Documents

# %%
sample_documents = [
    {
        "id": "doc1",
        "title": "Attention Is All You Need",
        "content": """
            We propose a new simple network architecture, the Transformer, based solely on attention mechanisms, dispensing with recurrence and convolutions entirely.
            The dominant sequence transduction models are based on complex recurrent or convolutional neural networks that include an encoder and a decoder.  
            The best performing models also connect the encoder and decoder through an attention mechanism.  We propose a new simple network architecture, the Transformer, based solely on attention mechanisms, dispensing with recurrence and convolutions entirely.  
            Experiments on two machine translation tasks show these models to be superior in quality while being more parallelizable and requiring significantly less time to train.  Our model achieves 28.4 BLEU on the WMT 2014 English- to-German translation task, improving over the existing best results, including ensembles, by over 2 BLEU. On the WMT 2014 English-to-French translation task, our model establishes a new single-model state-of-the-art BLEU score of 41.8 after training for 3.5 days on eight GPUs, a small fraction of the training costs of the best models from the literature. 
            We show that the Transformer generalizes well to other tasks by applying it successfully to English constituency parsing both with large and limited training data.
            """,
    },
    {
        "id": "doc2",
        "title": "The Future of AI",
        "content": "Artificial intelligence is rapidly transforming industries from healthcare to finance with machine learning models becoming more sophisticated.",
    },
    {
        "id": "doc3",
        "title": "Local-First Software",
        "content": "Local-first software prioritizes user ownership and control of data, storing data on user devices while still enabling collaboration.",
    },
]

# Process documents
processed_docs = []
for doc in sample_documents:
    chunks = chunk_document(
        clean_text(doc["content"]), config.chunk_size
    )
    for i, chunk in enumerate(chunks):
        processed_docs.append(
            {
                "doc_id": doc["id"],
                "chunk_id": f"{doc['id']}_chunk_{i}",
                "title": doc["title"],
                "content": chunk,
                "chunk_index": i,
            }
        )

print(f"Processed {len(sample_documents)} documents into {len(processed_docs)} chunks")

# %% [markdown]
# ## Embedding Generation and Pairwise Distances

# %%
# Generate embeddings for all chunks
chunk_texts = [doc["content"] for doc in processed_docs]
chunk_embeddings = get_embeddings(chunk_texts)

print(f"Generated embeddings shape: {chunk_embeddings.shape}")

# Calculate pairwise distances between chunks
pairwise_distances = cosine_distances(chunk_embeddings)
print(f"Pairwise distance matrix shape: {pairwise_distances.shape}")

# Show distance between first few chunks
print("\nPairwise distances (first 3 chunks):")
print(pairwise_distances[:3, :3])

# %% [markdown]
# ## In-Memory FAISS Index Creation and Search


# %%
# Create and populate in-memory document store
document_store = InMemoryDocumentStore(config.embedding_dim)
document_store.add_documents(processed_docs, chunk_embeddings)

print(f"Created in-memory index with {document_store.get_document_count()} documents")

# %% [markdown]
# ## Search Examples

# %%
# Test search queries
queries = [
    "attention mechanisms in neural networks",
    "AI safety and ethics",
    "local-first software principles",
]

for query in queries:
    print(f"\nQuery: '{query}'")
    results = document_store.search(query, top_k=3)

    for i, result in enumerate(results, 1):
        print(f"  {i}. {result['title']} (score: {result['score']:.3f})")
        print(f"     Chunk {result['chunk_index']}: {result['content'][:100]}...")

# %% [markdown]
# ## Distance Calculation Between Search String and Chunks


# %%
def calculate_query_chunk_distances_memory(
    query: str, chunks: List[Dict], embeddings: np.ndarray
) -> List[Dict]:
    """Calculate distances between query and document chunks in memory."""
    query_embedding = get_embeddings([query])

    # Calculate cosine distances
    distances = cosine_distances(query_embedding, embeddings)[0]

    results = []
    for chunk, distance in zip(chunks, distances):
        result = chunk.copy()
        result["distance"] = float(distance)
        results.append(result)

    return sorted(results, key=lambda x: x["distance"])


# Example distance calculation
test_query = "machine learning translation models"
distances = calculate_query_chunk_distances_memory(
    test_query, processed_docs, chunk_embeddings
)

print(f"\nDistances for query: '{test_query}'")
for result in distances[:3]:
    print(
        f"  {result['title']} (chunk {result['chunk_index']}): distance = {result['distance']:.3f}"
    )

# %% [markdown]
# ## Performance Analysis


# %%
def analyze_embedding_performance_memory(texts: List[str]) -> Dict:
    """Analyze embedding generation performance in memory."""
    import time

    start_time = time.time()
    embeddings = get_embeddings(texts)
    end_time = time.time()

    return {
        "num_texts": len(texts),
        "embedding_dim": embeddings.shape[1],
        "total_time": end_time - start_time,
        "time_per_text": (end_time - start_time) / len(texts),
        "memory_usage_mb": embeddings.nbytes / (1024 * 1024),
    }


# Performance test
performance_stats = analyze_embedding_performance_memory(
    [doc["content"] for doc in processed_docs]
)
print("In-Memory Performance:")
for key, value in performance_stats.items():
    print(f"  {key}: {value:.3f}")

# %% [markdown]
# ## Summary

# %%
print("LocalMind RAG Memory Demo Summary:")
print(f"- Model: {config.model_name}")
print(f"- Embedding dimension: {config.embedding_dim}")
print(f"- Total documents: {len(sample_documents)}")
print(f"- Total chunks: {len(processed_docs)}")
print(f"- In-memory index size: {document_store.get_document_count()}")
print("\nCore functionality demonstrated:")
print("1. ✅ Document chunking with overlap")
print("2. ✅ Embedding generation and pairwise distances")
print("3. ✅ In-memory similarity search (no disk I/O)")
print("4. ✅ Query-to-chunk distance calculation")
print("5. ✅ Performance analysis")
print("6. ✅ Everything runs in RAM - no persistence")
# %%
