#!/usr/bin/env python3
"""
Sample eval chunks by clustering the production SQLite corpus.

Reads chunk embeddings (bincode Vec<f32>) and text from localmind.db,
clusters them, and focuses on the largest clusters for maximum hard-negative
density.

Outputs:
  cluster_corpus.json      : all sampled chunks (search index)
  cluster_eval_chunks.json : same, sorted by proximity to centroid
                             (use for query generation)
"""

import json
import os
import pickle
import struct
import sqlite3
import numpy as np
from pathlib import Path
from typing import List, Tuple
from dataclasses import dataclass

from sklearn.cluster import KMeans
from sklearn.preprocessing import normalize
from tqdm import tqdm


if os.name == "nt":
    DB_PATH = os.path.join(os.environ.get("APPDATA", ""), "localmind", "localmind.db")
else:
    DB_PATH = os.path.join(os.path.expanduser("~"), ".localmind", "localmind.db")


def decode_bincode_vec_f32(blob: bytes) -> np.ndarray:
    """
    Decode a Rust bincode-serialised Vec<f32>.
    Bincode encodes Vec<T> as: u64-LE length prefix + N * T.
    """
    if len(blob) < 8:
        return np.array([], dtype=np.float32)
    n = struct.unpack_from("<Q", blob, 0)[0]
    expected_bytes = 8 + n * 4
    if len(blob) < expected_bytes:
        return np.array([], dtype=np.float32)
    return np.frombuffer(blob, dtype=np.float32, count=n, offset=8).copy()


@dataclass
class ClusterChunk:
    embedding_id: int
    document_id: int
    chunk_text: str
    document_title: str
    document_url: str
    chunk_start: int
    chunk_end: int
    cluster_id: int
    cluster_size: int
    dist_to_centroid: float


def load_sqlite_corpus(db_path: str = DB_PATH):
    """Load all chunk embeddings and text from SQLite."""
    print(f"Loading corpus from {db_path} ...")
    conn = sqlite3.connect(db_path)
    cur = conn.cursor()
    cur.execute("""
        SELECT e.id, e.document_id, e.chunk_start, e.chunk_end,
               e.embedding, d.title, d.content, d.url
        FROM embeddings e
        JOIN documents d ON e.document_id = d.id
        WHERE d.is_dead = 0 OR d.is_dead IS NULL
    """)
    rows = cur.fetchall()
    conn.close()
    print(f"Loaded {len(rows)} chunk rows from SQLite")

    ids, embeddings, metadatas, texts = [], [], [], []
    skipped = 0
    for (emb_id, doc_id, chunk_start, chunk_end, blob, title, content, url) in rows:
        vec = decode_bincode_vec_f32(blob)
        if vec.size == 0:
            skipped += 1
            continue
        chunk_text = (content or "")[chunk_start:chunk_end].strip()
        if not chunk_text:
            skipped += 1
            continue
        # Filter stub bookmarks: URL-only chunks with no real content
        words = chunk_text.split()
        if len(words) < 30:
            skipped += 1
            continue
        if chunk_text.startswith("Bookmark:") or chunk_text.startswith("URL:"):
            skipped += 1
            continue
        ids.append((emb_id, doc_id, chunk_start, chunk_end))
        embeddings.append(vec)
        metadatas.append({"title": title or "", "url": url or "", "doc_id": doc_id})
        texts.append(chunk_text)

    if skipped:
        print(f"Skipped {skipped} rows (empty blob or empty text)")

    embeddings_array = np.array(embeddings, dtype=np.float32)
    print(f"Corpus: {len(ids)} chunks, embedding dim={embeddings_array.shape[1]}")
    return ids, embeddings_array, metadatas, texts


def cluster_and_sample(
    ids, embeddings, metadatas, texts,
    n_clusters: int = 50,
    top_clusters: int = 20,
    per_cluster: int = 15,
    seed: int = 42,
) -> Tuple[List[ClusterChunk], List[ClusterChunk]]:
    """K-means cluster, focus on the largest clusters, sample per_cluster from each."""

    print(f"Clustering {len(ids)} chunks into {n_clusters} clusters...")
    normed = normalize(embeddings)
    km = KMeans(n_clusters=n_clusters, random_state=seed, n_init="auto")
    labels = km.fit_predict(normed)
    centroids = km.cluster_centers_

    cluster_sizes = np.bincount(labels, minlength=n_clusters)
    ranked = np.argsort(cluster_sizes)[::-1][:top_clusters]

    print(f"\nTop {top_clusters} cluster sizes (of {n_clusters} total):")
    for rank, cid in enumerate(ranked):
        print(f"  #{rank+1:2d}  cluster {cid:3d}  size={cluster_sizes[cid]}")

    corpus_chunks: List[ClusterChunk] = []

    for cluster_id in tqdm(ranked, desc="Sampling clusters"):
        mask = np.where(labels == cluster_id)[0]
        cluster_size = int(len(mask))
        cluster_embeddings = normed[mask]
        centroid = centroids[cluster_id]
        dists = np.linalg.norm(cluster_embeddings - centroid, axis=1)

        n_pick = min(per_cluster, cluster_size)
        for local_idx in np.argsort(dists)[:n_pick]:
            global_idx = mask[local_idx]
            emb_id, doc_id, chunk_start, chunk_end = ids[global_idx]
            meta = metadatas[global_idx]
            corpus_chunks.append(ClusterChunk(
                embedding_id=int(emb_id),
                document_id=int(doc_id),
                chunk_text=texts[global_idx],
                document_title=meta["title"],
                document_url=meta["url"],
                chunk_start=int(chunk_start),
                chunk_end=int(chunk_end),
                cluster_id=int(cluster_id),
                cluster_size=cluster_size,
                dist_to_centroid=float(dists[local_idx]),
            ))

    # eval_chunks: sorted so closest-to-centroid comes first within each cluster
    eval_chunks = sorted(corpus_chunks, key=lambda c: (c.cluster_id, c.dist_to_centroid))

    print(f"\nCorpus: {len(corpus_chunks)} chunks across {top_clusters} clusters")
    return corpus_chunks, eval_chunks


def _to_records(chunks: List[ClusterChunk]) -> List[dict]:
    return [
        {
            "chunk_id": c.embedding_id,
            "embedding_id": c.embedding_id,
            "document_id": c.document_id,
            "chunk_index": 0,
            "chunk_text": c.chunk_text,
            "document_title": c.document_title,
            "document_url": c.document_url,
            "chunk_start": c.chunk_start,
            "chunk_end": c.chunk_end,
            "parent_content": c.chunk_text,
            "quality_status": "SUITABLE",
            "quality_reason": f"cluster {c.cluster_id} (size {c.cluster_size}), dist={c.dist_to_centroid:.4f}",
            "confidence_score": 1.0,
        }
        for c in chunks
    ]


def save_chunks(chunks: List[ClusterChunk], output_path: str):
    records = _to_records(chunks)
    Path(output_path).parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(records, f, indent=2, ensure_ascii=False)
    print(f"Saved {len(records)} chunks to {output_path}")


def save_corpus_embeddings(
    corpus_chunks: List[ClusterChunk],
    all_ids, all_embeddings,
    cache_path: str,
    embedding_model_name: str,
):
    """Save corpus embeddings as pkl cache for the retrieval evaluator."""
    from chunk_quality_filter import QualityChunkSample

    emb_id_to_idx = {ids[0]: i for i, ids in enumerate(all_ids)}

    raw_chunks = {}
    raw_embeddings = {}

    for chunk in corpus_chunks:
        idx = emb_id_to_idx.get(chunk.embedding_id)
        if idx is None:
            continue
        cid = chunk.embedding_id
        qs = QualityChunkSample(
            chunk_id=cid,
            document_id=chunk.document_id,
            chunk_index=0,
            chunk_text=chunk.chunk_text,
            document_title=chunk.document_title,
            document_url=chunk.document_url,
            chunk_start=chunk.chunk_start,
            chunk_end=chunk.chunk_end,
            parent_content=chunk.chunk_text,
            embedding_id=cid,
            quality_status="SUITABLE",
            quality_reason="cluster sample",
            confidence_score=1.0,
        )
        raw_chunks[cid] = qs
        raw_embeddings[cid] = all_embeddings[idx]

    # Store in sorted key order so list(keys()) matches the matrix row order in search_chunks
    indexed_chunks = {k: raw_chunks[k] for k in sorted(raw_chunks)}
    chunk_embeddings = {k: raw_embeddings[k] for k in sorted(raw_embeddings)}

    matrix = np.array([chunk_embeddings[k] for k in chunk_embeddings], dtype=np.float32)
    norms = np.linalg.norm(matrix, axis=1, keepdims=True)
    matrix = matrix / (norms + 1e-10)

    safe_name = embedding_model_name.replace("/", "_")
    cache_file = Path(cache_path) / f"{safe_name}_chunks.pkl"
    cache_file.parent.mkdir(parents=True, exist_ok=True)
    with open(cache_file, "wb") as f:
        pickle.dump({"chunks": indexed_chunks, "embeddings": chunk_embeddings, "matrix": matrix}, f)
    print(f"Saved corpus cache ({len(indexed_chunks)} chunks) to {cache_file}")


if __name__ == "__main__":
    import click

    @click.command()
    @click.option("--n-clusters", default=50, help="Total K-means clusters to fit")
    @click.option("--top-clusters", default=20, help="Keep only the N largest clusters")
    @click.option("--per-cluster", default=15, help="Chunks to sample per cluster")
    @click.option("--corpus-output", default="data/cluster_corpus.json")
    @click.option("--eval-output", default="data/cluster_eval_chunks.json")
    @click.option("--db-path", default=DB_PATH, help="Path to localmind.db")
    @click.option("--embedding-model", default="text-embedding-embeddinggemma-300m-qat")
    @click.option("--cache-dir", default="./chunk_embeddings_cache")
    def main(n_clusters, top_clusters, per_cluster, corpus_output, eval_output,
             db_path, embedding_model, cache_dir):
        """Sample eval chunks from SQLite by focusing on the largest topic clusters."""

        ids, embeddings, metadatas, texts = load_sqlite_corpus(db_path)
        corpus_chunks, eval_chunks = cluster_and_sample(
            ids, embeddings, metadatas, texts,
            n_clusters=n_clusters,
            top_clusters=top_clusters,
            per_cluster=per_cluster,
        )

        save_chunks(corpus_chunks, corpus_output)
        save_chunks(eval_chunks, eval_output)

        print(f"\nNext steps:")
        print(f"  1. python main.py generate-chunk-terms --chunks {eval_output}")
        print(f"  2. python make_qrels.py --chunks {eval_output} --output data/cluster_queries.json")
        print(f"  3. python run_retrieval_eval.py evaluate --queries data/cluster_queries.json --chunks {corpus_output} --lmstudio --embedding-model {embedding_model}")

    main()
