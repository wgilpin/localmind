#!/usr/bin/env python3
"""
Evaluate BM25, vector, and hybrid retrieval using generated_queries.json.

Metrics: MRR, Recall@K, NDCG@K (K = 5, 10, 20)
Hybrid uses Reciprocal Rank Fusion (RRF).
"""

import json
import math
import pickle
import re
import time
from pathlib import Path
from typing import Dict, List, Optional, Tuple, Any

import numpy as np
from tqdm import tqdm

try:
    from rank_bm25 import BM25Okapi
    HAS_BM25 = True
except ImportError:
    HAS_BM25 = False

from chunk_vector_evaluator import ChunkVectorEvaluator
from chunk_quality_filter import QualityChunkSample

EVAL_KS = [5, 10, 20]
RRF_K = 60


def _tokenize(text: str) -> List[str]:
    return re.findall(r'\w+', text.lower())


def _dcg_at_k(rank: Optional[int], k: int) -> float:
    if rank is None or rank > k:
        return 0.0
    return 1.0 / math.log2(rank + 1)


def compute_query_metrics(ranked_ids: List[int], relevant_id: int) -> Dict[str, float]:
    rank = None
    for i, cid in enumerate(ranked_ids):
        if cid == relevant_id:
            rank = i + 1
            break
    metrics = {"rank": rank, "rr": (1.0 / rank) if rank else 0.0}
    for k in EVAL_KS:
        metrics[f"recall_at_{k}"] = 1.0 if rank and rank <= k else 0.0
        metrics[f"ndcg_at_{k}"] = _dcg_at_k(rank, k)  # IDCG=1 for single relevant item
    return metrics


def aggregate(per_query: List[Dict]) -> Dict[str, Any]:
    n = len(per_query)
    if n == 0:
        return {"num_queries": 0}
    out: Dict[str, Any] = {
        "num_queries": n,
        "mrr": sum(q["rr"] for q in per_query) / n,
        "found_any": sum(1 for q in per_query if q["rank"] is not None),
    }
    for k in EVAL_KS:
        out[f"recall_at_{k}"] = sum(q[f"recall_at_{k}"] for q in per_query) / n
        out[f"ndcg_at_{k}"] = sum(q[f"ndcg_at_{k}"] for q in per_query) / n
    return out


class RetrievalEvaluator:
    """Evaluate BM25, vector, and hybrid retrieval over the chunk index."""

    def __init__(
        self,
        embedding_model: str = "all-MiniLM-L6-v2",
        cache_dir: str = "./chunk_embeddings_cache",
        use_ollama: bool = False,
        use_lmstudio: bool = False,
        ollama_url: str = "http://localhost:11434",
        lmstudio_url: str = "http://localhost:1234",
    ):
        if not HAS_BM25:
            raise ImportError("rank-bm25 is required. Run: uv add rank-bm25")

        self.embedding_model_name = embedding_model
        self.cache_dir = Path(cache_dir)
        self.use_ollama = use_ollama
        self.use_lmstudio = use_lmstudio

        self.vector_eval = ChunkVectorEvaluator(
            embedding_model=embedding_model,
            cache_dir=cache_dir,
            use_ollama=use_ollama,
            use_lmstudio=use_lmstudio,
            ollama_url=ollama_url,
            lmstudio_url=lmstudio_url,
        )

        self.bm25: Optional[BM25Okapi] = None
        self.bm25_chunk_ids: List[int] = []
        self.chunk_ids_order: List[int] = []

    def load_index(self, chunks: Optional[List[QualityChunkSample]] = None):
        """Load vector index from cache and build BM25 index over the same corpus."""
        if chunks is None:
            chunks = []

        self.vector_eval.index_chunks(chunks)

        indexed = self.vector_eval.indexed_chunks
        if not indexed:
            raise ValueError("No chunks in vector index. Run chunk pipeline first.")

        self.chunk_ids_order = list(self.vector_eval.chunk_embeddings.keys())

        print(f"Building BM25 index over {len(indexed)} chunks...")
        self.bm25_chunk_ids = list(indexed.keys())
        tokenized = [_tokenize(indexed[cid].chunk_text) for cid in self.bm25_chunk_ids]
        self.bm25 = BM25Okapi(tokenized)
        print("BM25 index ready.")

    def _bm25_search(self, query: str, top_k: int) -> List[Tuple[int, float]]:
        tokens = _tokenize(query)
        scores = self.bm25.get_scores(tokens)
        top_idx = np.argsort(scores)[::-1][:top_k]
        return [(self.bm25_chunk_ids[i], float(scores[i])) for i in top_idx]

    def _vector_search(self, query: str, top_k: int) -> List[Tuple[int, float]]:
        return self.vector_eval.search_chunks(query, top_k=top_k)

    @staticmethod
    def _rrf_fuse(
        bm25_results: List[Tuple[int, float]],
        vector_results: List[Tuple[int, float]],
        top_k: int,
        k: int = RRF_K,
    ) -> List[Tuple[int, float]]:
        scores: Dict[int, float] = {}
        for rank, (chunk_id, _) in enumerate(bm25_results, start=1):
            scores[chunk_id] = scores.get(chunk_id, 0.0) + 1.0 / (k + rank)
        for rank, (chunk_id, _) in enumerate(vector_results, start=1):
            scores[chunk_id] = scores.get(chunk_id, 0.0) + 1.0 / (k + rank)
        return sorted(scores.items(), key=lambda x: x[1], reverse=True)[:top_k]

    @staticmethod
    def _rrf_fuse_many(
        result_lists: List[List[Tuple[int, float]]],
        top_k: int,
        k: int = RRF_K,
    ) -> List[Tuple[int, float]]:
        scores: Dict[int, float] = {}
        for results in result_lists:
            for rank, (chunk_id, _) in enumerate(results, start=1):
                scores[chunk_id] = scores.get(chunk_id, 0.0) + 1.0 / (k + rank)
        return sorted(scores.items(), key=lambda x: x[1], reverse=True)[:top_k]

    def evaluate(
        self,
        queries: Dict[str, Dict],
        top_k: int = 20,
    ) -> Dict[str, Any]:
        """
        Run all three retrieval methods and return per-method metrics.

        queries: dict loaded from generated_queries.json
                 each entry has "text" and "chunk_id"
        """
        bm25_per_query: List[Dict] = []
        vector_per_query: List[Dict] = []
        hybrid_per_query: List[Dict] = []

        valid_queries = {
            qid: q for qid, q in queries.items()
            if q.get("chunk_id") is not None
            and q["chunk_id"] in self.vector_eval.indexed_chunks
        }

        skipped = len(queries) - len(valid_queries)
        if skipped:
            print(f"Skipping {skipped} queries whose chunk_id is not in the index.")

        print(f"Evaluating {len(valid_queries)} queries with top_k={top_k}...")

        for qid, q in tqdm(valid_queries.items()):
            query_text = q["text"]
            relevant_id = q["chunk_id"]

            bm25_res = self._bm25_search(query_text, top_k)
            vector_res = self._vector_search(query_text, top_k)
            hybrid_res = self._rrf_fuse(bm25_res, vector_res, top_k)

            bm25_ids = [c for c, _ in bm25_res]
            vector_ids = [c for c, _ in vector_res]
            hybrid_ids = [c for c, _ in hybrid_res]

            bm25_per_query.append(compute_query_metrics(bm25_ids, relevant_id))
            vector_per_query.append(compute_query_metrics(vector_ids, relevant_id))
            hybrid_per_query.append(compute_query_metrics(hybrid_ids, relevant_id))

        return {
            "embedding_model": self.embedding_model_name,
            "corpus_size": len(self.vector_eval.indexed_chunks),
            "num_queries_evaluated": len(valid_queries),
            "top_k": top_k,
            "bm25": aggregate(bm25_per_query),
            "vector": aggregate(vector_per_query),
            "hybrid_rrf": aggregate(hybrid_per_query),
        }

    def evaluate_multi_query(
        self,
        queries: Dict[str, Dict],
        query_variants: Dict[str, List[str]],
        top_k: int = 20,
    ) -> Dict[str, Any]:
        """
        Evaluate with multi-query expansion. For each query, fuses results across
        the original query plus all its variants.

        query_variants: qid -> list of variant query strings (not including original)
        """
        valid_queries = {
            qid: q for qid, q in queries.items()
            if q.get("chunk_id") is not None
            and q["chunk_id"] in self.vector_eval.indexed_chunks
        }

        skipped = len(queries) - len(valid_queries)
        if skipped:
            print(f"Skipping {skipped} queries whose chunk_id is not in the index.")

        print(f"Evaluating {len(valid_queries)} queries with multi-query expansion, top_k={top_k}...")

        bm25_per_query: List[Dict] = []
        vector_per_query: List[Dict] = []
        hybrid_per_query: List[Dict] = []
        multi_bm25_per_query: List[Dict] = []
        multi_vector_per_query: List[Dict] = []
        multi_hybrid_per_query: List[Dict] = []

        for qid, q in tqdm(valid_queries.items()):
            relevant_id = q["chunk_id"]
            all_queries = [q["text"]] + (query_variants.get(qid) or [])

            bm25_lists, vector_lists, hybrid_lists = [], [], []
            for qtext in all_queries:
                b = self._bm25_search(qtext, top_k)
                v = self._vector_search(qtext, top_k)
                h = self._rrf_fuse(b, v, top_k)
                bm25_lists.append(b)
                vector_lists.append(v)
                hybrid_lists.append(h)

            # Single-query metrics (original only)
            bm25_per_query.append(compute_query_metrics([c for c, _ in bm25_lists[0]], relevant_id))
            vector_per_query.append(compute_query_metrics([c for c, _ in vector_lists[0]], relevant_id))
            hybrid_per_query.append(compute_query_metrics([c for c, _ in hybrid_lists[0]], relevant_id))

            # Multi-query fused metrics
            multi_bm25_ids = [c for c, _ in self._rrf_fuse_many(bm25_lists, top_k)]
            multi_vector_ids = [c for c, _ in self._rrf_fuse_many(vector_lists, top_k)]
            multi_hybrid_ids = [c for c, _ in self._rrf_fuse_many(hybrid_lists, top_k)]

            multi_bm25_per_query.append(compute_query_metrics(multi_bm25_ids, relevant_id))
            multi_vector_per_query.append(compute_query_metrics(multi_vector_ids, relevant_id))
            multi_hybrid_per_query.append(compute_query_metrics(multi_hybrid_ids, relevant_id))

        return {
            "embedding_model": self.embedding_model_name,
            "corpus_size": len(self.vector_eval.indexed_chunks),
            "num_queries_evaluated": len(valid_queries),
            "top_k": top_k,
            "bm25": aggregate(bm25_per_query),
            "vector": aggregate(vector_per_query),
            "hybrid_rrf": aggregate(hybrid_per_query),
            "multi_bm25": aggregate(multi_bm25_per_query),
            "multi_vector": aggregate(multi_vector_per_query),
            "multi_hybrid": aggregate(multi_hybrid_per_query),
        }


def print_results(results: Dict[str, Any]):
    print()
    print("=" * 70)
    print("RETRIEVAL EVALUATION RESULTS")
    print("=" * 70)
    print(f"Embedding model : {results['embedding_model']}")
    print(f"Corpus size     : {results['corpus_size']} chunks")
    print(f"Queries         : {results['num_queries_evaluated']}")
    print(f"Top-K           : {results['top_k']}")
    print()

    has_multi = "multi_bm25" in results
    methods = ["bm25", "vector", "hybrid_rrf"]
    labels = {"bm25": "BM25", "vector": "Vector", "hybrid_rrf": "Hybrid (RRF)"}
    if has_multi:
        methods += ["multi_bm25", "multi_vector", "multi_hybrid"]
        labels.update({"multi_bm25": "MQ-BM25", "multi_vector": "MQ-Vector", "multi_hybrid": "MQ-Hybrid"})

    col_w = 15
    header = f"{'Metric':<18}" + "".join(f"{labels[m]:>{col_w}}" for m in methods)
    sep = "-" * (18 + col_w * len(methods))
    print(header)
    print(sep)

    def row(name: str, key: str):
        vals = [results[m].get(key, float("nan")) for m in methods]
        line = f"{name:<18}" + "".join(f"{v:>{col_w}.4f}" for v in vals)
        print(line)

    row("MRR", "mrr")
    for k in EVAL_KS:
        row(f"Recall@{k}", f"recall_at_{k}")
    for k in EVAL_KS:
        row(f"NDCG@{k}", f"ndcg_at_{k}")

    print(sep)
    for m in methods:
        found = results[m].get("found_any", 0)
        total = results[m].get("num_queries", 0)
        print(f"{labels[m]} found {found}/{total} relevant chunks in top-{results['top_k']}")
    print("=" * 70)
