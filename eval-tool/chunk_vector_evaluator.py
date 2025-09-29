#!/usr/bin/env python3
"""
Chunk Vector Evaluator Module
Evaluates search performance at the chunk level using various embedding models.
"""

import json
import time
from pathlib import Path
from typing import List, Dict, Optional, Tuple, Any
from dataclasses import dataclass, asdict
import numpy as np
import pandas as pd
from sentence_transformers import SentenceTransformer
from chunk_quality_filter import QualityChunkSample
from chunk_query_generator import ChunkSearchTerms
from ollama_embedding import OllamaEmbedding
from lmstudio_embedding import LMStudioEmbedding
import pickle
from tqdm import tqdm


@dataclass
class ChunkSearchResult:
    """Result of searching for a chunk"""
    query: str
    target_chunk_id: int
    found_in_top_k: bool
    rank: Optional[int]  # Position in results (1-indexed), None if not found
    score: Optional[float]  # Similarity score
    top_chunks: List[int]  # IDs of top chunks returned


@dataclass
class ChunkEvaluation:
    """Evaluation results for a single chunk"""
    chunk_id: int
    document_id: int
    chunk_preview: str
    search_results: List[ChunkSearchResult]
    hit_rate: float  # Percentage of queries that found this chunk
    avg_rank: Optional[float]  # Average rank when found
    mrr: float  # Mean Reciprocal Rank


@dataclass
class ChunkEvaluationReport:
    """Complete evaluation report"""
    metadata: Dict[str, Any]
    evaluations: List[ChunkEvaluation]
    overall_metrics: Dict[str, float]
    timestamp: str


class ChunkVectorEvaluator:
    """Evaluate chunk-based vector search performance"""

    def __init__(self, embedding_model: str = "all-MiniLM-L6-v2",
                use_ollama: bool = False,
                use_lmstudio: bool = False,
                ollama_url: str = "http://localhost:11434",
                lmstudio_url: str = "http://localhost:1234",
                cache_dir: str = "./chunk_embeddings_cache"):
        """
        Initialize the evaluator

        Args:
            embedding_model: Model name for embeddings
            use_ollama: Whether to use Ollama for embeddings
            use_lmstudio: Whether to use LM Studio for embeddings
            ollama_url: Ollama API URL
            lmstudio_url: LM Studio API URL
            cache_dir: Directory for caching embeddings
        """
        self.embedding_model_name = embedding_model
        self.use_ollama = use_ollama
        self.use_lmstudio = use_lmstudio
        self.cache_dir = Path(cache_dir)
        self.cache_dir.mkdir(parents=True, exist_ok=True)

        # Initialize embedding model
        if use_lmstudio:
            self.embedding_model = LMStudioEmbedding(
                model_name=embedding_model,
                base_url=lmstudio_url
            )
            print(f"[BOT] Using LM Studio embedding model: {embedding_model}")
        elif use_ollama:
            self.embedding_model = OllamaEmbedding(
                model_name=embedding_model,
                base_url=ollama_url
            )
            print(f"[BOT] Using Ollama embedding model: {embedding_model}")
        else:
            self.embedding_model = SentenceTransformer(embedding_model)
            print(f"[BOT] Using SentenceTransformer model: {embedding_model}")

        # Warm up the model with a test embedding
        self._warmup_model()

        # Check if model needs special formatting
        prefixes = self._get_instruction_prefixes()
        if prefixes['query_prefix'] or prefixes['document_prefix']:
            print(f"[INFO] Using model-specific formatting for {embedding_model}:")
            if prefixes['query_prefix']:
                print(f"  Query prefix: '{prefixes['query_prefix'][:50]}...'")
            if prefixes['document_prefix']:
                print(f"  Document prefix: '{prefixes['document_prefix'][:50]}...'")

        # Storage for indexed chunks
        self.indexed_chunks = {}  # chunk_id -> QualityChunkSample
        self.chunk_embeddings = {}  # chunk_id -> embedding vector
        self.embedding_matrix = None  # Numpy array for efficient search

    def _warmup_model(self):
        """Warm up the model by generating a test embedding to ensure it's loaded"""
        print("[WARMUP] Loading model with test embedding...")
        warmup_start = time.time()

        # Test with both query and document formats to ensure both paths are warmed up
        test_texts = [
            "This is a test query for model warmup",
            "This is a test document for model warmup to ensure the embedding model is fully loaded"
        ]

        try:
            if self.use_ollama or self.use_lmstudio:
                # For API-based models, test batch processing
                _ = self.embedding_model.encode(test_texts)
            else:
                # For SentenceTransformers
                _ = self.embedding_model.encode(test_texts, convert_to_numpy=True, show_progress_bar=False)

            warmup_time = time.time() - warmup_start
            print(f"[WARMUP] Model loaded successfully in {warmup_time:.2f}s")
        except Exception as e:
            print(f"[WARMUP] Warning: Model warmup failed: {e}")
            print("[WARMUP] Continuing anyway, first real embedding may be slower")

    def _get_instruction_prefixes(self):
        """Get appropriate instruction prefixes for the embedding model"""
        model_name = self.embedding_model_name.lower()

        # Remove common suffixes for checking base model type
        base_model_name = model_name.replace("-gpu", "").replace("text-embedding-", "")

        if "embeddinggemma" in base_model_name:
            return {
                'query_prefix': "task: search result | query: ",
                'document_prefix': "title: {title} | text: "
            }
        elif "nomic-embed-text" in base_model_name:
            return {
                'query_prefix': "search_query: ",
                'document_prefix': "search_document: "
            }
        elif "qwen3-embedding" in base_model_name:
            return {
                'query_prefix': "Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery: ",
                'document_prefix': ""  # Qwen3 doesn't use document prefix
            }
        else:
            # SentenceTransformers and other models typically don't need prefixes
            return {
                'query_prefix': "",
                'document_prefix': ""
            }

    def _format_text_for_embedding(self, text: str, is_query: bool = False,
                                  document_title: str = "", document_url: str = ""):
        """
        Format text with appropriate prefixes for the embedding model

        Args:
            text: The text to embed
            is_query: Whether this is a search query (True) or document (False)
            document_title: Title of the document (for document embeddings)
            document_url: URL of the document (for additional context)
        """
        prefixes = self._get_instruction_prefixes()

        if is_query:
            return prefixes['query_prefix'] + text
        else:
            if "embeddinggemma" in self.embedding_model_name.lower():
                # EmbeddingGemma uses title in the prefix
                title = document_title if document_title else "content"
                prefix = prefixes['document_prefix'].format(title=title)
                return prefix + text
            else:
                return prefixes['document_prefix'] + text

    def get_embedding(self, text: str, is_query: bool = False,
                      document_title: str = "", document_url: str = "") -> np.ndarray:
        """
        Get embedding for text with appropriate formatting

        Args:
            text: The text to embed
            is_query: Whether this is a search query (True) or document (False)
            document_title: Title of the document (for document embeddings)
            document_url: URL of the document (for additional context)
        """
        # Apply model-specific formatting
        formatted_text = self._format_text_for_embedding(
            text, is_query=is_query,
            document_title=document_title,
            document_url=document_url
        )

        if self.use_ollama or self.use_lmstudio:
            # Both Ollama and LMStudio use the encode() method
            embedding = self.embedding_model.encode(formatted_text)
            return np.array(embedding)
        else:
            return self.embedding_model.encode(formatted_text, convert_to_numpy=True)

    def index_chunks(self, chunks: List[QualityChunkSample], force_reindex: bool = False):
        """
        Index chunks for vector search

        Args:
            chunks: List of chunks to index
            force_reindex: Whether to force re-indexing even if cache exists
        """
        # Check for cached embeddings
        cache_file = self.cache_dir / f"{self.embedding_model_name.replace('/', '_')}_chunks.pkl"

        if cache_file.exists() and not force_reindex:
            print(f"[FOLDER] Loading cached embeddings from {cache_file}")
            with open(cache_file, 'rb') as f:
                cache_data = pickle.load(f)
                self.indexed_chunks = cache_data['chunks']
                self.chunk_embeddings = cache_data['embeddings']
                self.embedding_matrix = cache_data['matrix']
            print(f"[OK] Loaded embeddings for {len(self.indexed_chunks)} chunks")
            return

        print(f"[SEARCH] Indexing {len(chunks)} chunks...")
        start_time = time.time()

        self.indexed_chunks = {chunk.chunk_id: chunk for chunk in chunks}
        self.chunk_embeddings = {}

        # Prepare texts for batch processing
        texts_to_embed = []
        chunk_ids_order = []

        for chunk in chunks:
            # Format text with document formatting
            formatted_text = self._format_text_for_embedding(
                chunk.chunk_text,
                is_query=False,
                document_title=chunk.document_title,
                document_url=chunk.document_url
            )
            texts_to_embed.append(formatted_text)
            chunk_ids_order.append(chunk.chunk_id)

        # Batch process embeddings
        if self.use_ollama or self.use_lmstudio:
            # Use batch processing for API-based models
            batch_size = 50  # Reasonable batch size for API calls
            all_embeddings = []

            for i in tqdm(range(0, len(texts_to_embed), batch_size),
                         desc=f"Generating embeddings (batch size {batch_size})",
                         unit="batch"):
                batch = texts_to_embed[i:i + batch_size]
                if self.use_lmstudio:
                    # LMStudio now supports batch processing
                    batch_embeddings = self.embedding_model.encode(batch, batch_size=len(batch))
                else:
                    # Ollama processes one by one (no batch support yet)
                    batch_embeddings = [self.embedding_model.encode(text) for text in batch]

                # Convert to numpy arrays
                for emb in batch_embeddings:
                    all_embeddings.append(np.array(emb))
        else:
            # SentenceTransformers naturally handles batches efficiently
            print("Using SentenceTransformers batch processing...")
            all_embeddings = self.embedding_model.encode(
                texts_to_embed,
                convert_to_numpy=True,
                show_progress_bar=True,
                batch_size=32  # Optimal batch size for SentenceTransformers
            )

        # Map embeddings to chunk IDs
        for chunk_id, embedding in zip(chunk_ids_order, all_embeddings):
            self.chunk_embeddings[chunk_id] = embedding

        if not chunks:
            print("No chunks to index")
            return

        # Create embedding matrix for efficient search
        chunk_ids = list(self.chunk_embeddings.keys())
        embeddings = [self.chunk_embeddings[cid] for cid in chunk_ids]
        self.embedding_matrix = np.array(embeddings)

        # Normalize for cosine similarity
        if self.embedding_matrix.ndim > 1:
            norms = np.linalg.norm(self.embedding_matrix, axis=1, keepdims=True)
            self.embedding_matrix = self.embedding_matrix / (norms + 1e-10)

        elapsed = time.time() - start_time
        print(f"[OK] Indexed {len(chunks)} chunks in {elapsed:.2f} seconds")

        # Cache embeddings
        cache_data = {
            'chunks': self.indexed_chunks,
            'embeddings': self.chunk_embeddings,
            'matrix': self.embedding_matrix
        }
        with open(cache_file, 'wb') as f:
            pickle.dump(cache_data, f)
        print(f"[SAVE] Cached embeddings to {cache_file}")

    def search_chunks(self, query: str, top_k: int = 10) -> List[Tuple[int, float]]:
        """
        Search for chunks matching the query

        Args:
            query: Search query
            top_k: Number of top results to return

        Returns:
            List of (chunk_id, similarity_score) tuples
        """
        if self.embedding_matrix is None:
            raise ValueError("No chunks indexed. Call index_chunks() first.")

        # Get query embedding with query formatting
        query_embedding = self.get_embedding(query, is_query=True)
        query_embedding = query_embedding / (np.linalg.norm(query_embedding) + 1e-10)

        # Compute similarities
        similarities = np.dot(self.embedding_matrix, query_embedding)

        # Get top-k indices
        top_indices = np.argsort(similarities)[-top_k:][::-1]

        # Map back to chunk IDs
        chunk_ids = list(self.chunk_embeddings.keys())
        results = []
        for idx in top_indices:
            chunk_id = chunk_ids[idx]
            score = float(similarities[idx])
            results.append((chunk_id, score))

        return results

    def _evaluate_chunk_with_cached_embeddings(self, chunk: QualityChunkSample,
                                              queries: List[str],
                                              query_embeddings: Dict[str, np.ndarray],
                                              top_k: int = 10) -> ChunkEvaluation:
        """
        Evaluate search performance for a single chunk using pre-computed embeddings

        Args:
            chunk: The chunk to evaluate
            queries: List of search queries for this chunk
            query_embeddings: Pre-computed query embeddings
            top_k: Number of top results to consider

        Returns:
            ChunkEvaluation with results
        """
        search_results = []
        ranks = []

        # Get chunk IDs list once
        chunk_ids = list(self.chunk_embeddings.keys())

        for query in queries:
            # Use pre-computed query embedding
            query_embedding = query_embeddings[query]

            # Compute similarities with vectorized operation
            similarities = np.dot(self.embedding_matrix, query_embedding)

            # Get top-k indices
            top_indices = np.argsort(similarities)[-top_k:][::-1]

            # Check if target chunk was found
            found = False
            rank = None
            score = None
            top_chunk_ids = []

            for i, idx in enumerate(top_indices):
                result_chunk_id = chunk_ids[idx]
                top_chunk_ids.append(result_chunk_id)
                if result_chunk_id == chunk.chunk_id:
                    found = True
                    rank = i + 1  # 1-indexed
                    score = float(similarities[idx])

            search_results.append(ChunkSearchResult(
                query=query,
                target_chunk_id=chunk.chunk_id,
                found_in_top_k=found,
                rank=rank,
                score=score,
                top_chunks=top_chunk_ids
            ))

            if rank:
                ranks.append(rank)

        # Calculate metrics
        hit_rate = sum(1 for r in search_results if r.found_in_top_k) / len(search_results) if search_results else 0
        avg_rank = np.mean(ranks) if ranks else None
        mrr = np.mean([1/r for r in ranks]) if ranks else 0

        return ChunkEvaluation(
            chunk_id=chunk.chunk_id,
            document_id=chunk.document_id,
            chunk_preview=chunk.chunk_text[:100],
            search_results=search_results,
            hit_rate=hit_rate,
            avg_rank=avg_rank,
            mrr=mrr
        )

    def evaluate_chunk(self, chunk: QualityChunkSample,
                      search_terms: ChunkSearchTerms,
                      top_k: int = 10) -> ChunkEvaluation:
        """
        Evaluate search performance for a single chunk

        Args:
            chunk: The chunk to evaluate
            search_terms: Search terms generated for this chunk
            top_k: Number of top results to consider

        Returns:
            ChunkEvaluation with results
        """
        search_results = []
        ranks = []

        for query in search_terms.search_terms:
            # Search for the query
            results = self.search_chunks(query, top_k)

            # Check if target chunk was found
            found = False
            rank = None
            score = None
            top_chunk_ids = []

            for i, (result_chunk_id, result_score) in enumerate(results):
                top_chunk_ids.append(result_chunk_id)
                if result_chunk_id == chunk.chunk_id:
                    found = True
                    rank = i + 1  # 1-indexed
                    score = result_score

            search_results.append(ChunkSearchResult(
                query=query,
                target_chunk_id=chunk.chunk_id,
                found_in_top_k=found,
                rank=rank,
                score=score,
                top_chunks=top_chunk_ids
            ))

            if rank:
                ranks.append(rank)

        # Calculate metrics
        hit_rate = sum(1 for r in search_results if r.found_in_top_k) / len(search_results) if search_results else 0
        avg_rank = np.mean(ranks) if ranks else None
        mrr = np.mean([1/r for r in ranks]) if ranks else 0

        return ChunkEvaluation(
            chunk_id=chunk.chunk_id,
            document_id=chunk.document_id,
            chunk_preview=chunk.chunk_text[:100],
            search_results=search_results,
            hit_rate=hit_rate,
            avg_rank=avg_rank,
            mrr=mrr
        )

    def evaluate_all_chunks(self, chunks: List[QualityChunkSample],
                           terms_dict: Dict[int, ChunkSearchTerms],
                           top_k: int = 10) -> ChunkEvaluationReport:
        """
        Evaluate all chunks

        Args:
            chunks: List of chunks to evaluate
            terms_dict: Dictionary of search terms for each chunk
            top_k: Number of top results to consider

        Returns:
            Complete evaluation report
        """
        print(f"\n[SEARCH] Evaluating {len(chunks)} chunks with top-{top_k} search...")
        start_time = time.time()

        # Step 1: Collect all unique queries and pre-compute their embeddings
        print("[SEARCH] Pre-computing query embeddings...")
        all_queries = set()
        chunk_to_queries = {}

        for chunk in chunks:
            if chunk.chunk_id not in terms_dict:
                continue
            search_terms = terms_dict[chunk.chunk_id]
            chunk_queries = search_terms.search_terms
            chunk_to_queries[chunk.chunk_id] = chunk_queries
            all_queries.update(chunk_queries)

        all_queries = list(all_queries)
        print(f"  Found {len(all_queries)} unique queries across all chunks")

        # Batch compute all query embeddings
        query_embeddings = {}
        batch_size = 50  # Reasonable batch size for APIs

        # Format all queries as search queries
        formatted_queries = [self._format_text_for_embedding(q, is_query=True) for q in all_queries]

        if self.use_ollama or self.use_lmstudio:
            # Process in batches for API-based models
            for i in tqdm(range(0, len(formatted_queries), batch_size),
                         desc=f"Computing query embeddings (batch size {batch_size})",
                         unit="batch"):
                batch = formatted_queries[i:i + batch_size]
                batch_queries = all_queries[i:i + batch_size]

                if self.use_lmstudio:
                    batch_embeddings = self.embedding_model.encode(batch, batch_size=len(batch))
                else:
                    # Ollama with concurrent processing
                    batch_embeddings = self.embedding_model.encode(batch, max_workers=4)

                # Map embeddings to queries and normalize
                for query, embedding in zip(batch_queries, batch_embeddings):
                    emb_array = np.array(embedding)
                    emb_array = emb_array / (np.linalg.norm(emb_array) + 1e-10)
                    query_embeddings[query] = emb_array
        else:
            # SentenceTransformers batch processing
            embeddings = self.embedding_model.encode(
                formatted_queries,
                convert_to_numpy=True,
                show_progress_bar=True,
                batch_size=32
            )
            # Map and normalize
            for query, embedding in zip(all_queries, embeddings):
                embedding = embedding / (np.linalg.norm(embedding) + 1e-10)
                query_embeddings[query] = embedding

        print(f"  Pre-computed {len(query_embeddings)} query embeddings")

        # Step 2: Evaluate chunks using pre-computed embeddings
        evaluations = []
        all_hit_rates = []
        all_ranks = []
        all_mrrs = []

        # Progress bar for evaluation
        for chunk in tqdm(chunks, desc="Evaluating chunks", unit="chunk"):
            if chunk.chunk_id not in chunk_to_queries:
                continue

            # Use pre-computed embeddings for evaluation
            evaluation = self._evaluate_chunk_with_cached_embeddings(
                chunk, chunk_to_queries[chunk.chunk_id], query_embeddings, top_k
            )
            evaluations.append(evaluation)

            all_hit_rates.append(evaluation.hit_rate)
            if evaluation.avg_rank:
                all_ranks.append(evaluation.avg_rank)
            all_mrrs.append(evaluation.mrr)

        # Calculate overall metrics
        overall_metrics = {
            'mean_hit_rate': np.mean(all_hit_rates) if all_hit_rates else 0,
            'median_hit_rate': np.median(all_hit_rates) if all_hit_rates else 0,
            'mean_rank': np.mean(all_ranks) if all_ranks else None,
            'median_rank': np.median(all_ranks) if all_ranks else None,
            'mean_mrr': np.mean(all_mrrs) if all_mrrs else 0,
            'perfect_hit_rate': sum(1 for hr in all_hit_rates if hr == 1.0) / len(all_hit_rates) if all_hit_rates else 0,
            'zero_hit_rate': sum(1 for hr in all_hit_rates if hr == 0.0) / len(all_hit_rates) if all_hit_rates else 0,
            'total_chunks_evaluated': len(evaluations),
            'top_k': top_k
        }

        elapsed = time.time() - start_time

        # Create report
        report = ChunkEvaluationReport(
            metadata={
                'embedding_model': self.embedding_model_name,
                'use_ollama': self.use_ollama,
                'evaluation_time_seconds': elapsed,
                'chunks_per_second': len(evaluations) / elapsed if elapsed > 0 else 0
            },
            evaluations=evaluations,
            overall_metrics=overall_metrics,
            timestamp=time.strftime('%Y-%m-%d %H:%M:%S')
        )

        print(f"[OK] Evaluation complete in {elapsed:.2f} seconds")
        self.print_metrics(report)

        return report

    def print_metrics(self, report: ChunkEvaluationReport):
        """Print evaluation metrics"""
        print("\n" + "="*60)
        print("📊 CHUNK EVALUATION METRICS")
        print("="*60)
        print(f"Model: {report.metadata['embedding_model']}")
        print(f"Chunks evaluated: {report.overall_metrics['total_chunks_evaluated']}")
        print(f"Top-K setting: {report.overall_metrics['top_k']}")
        print("\nPerformance Metrics:")
        print(f"  Mean Hit Rate: {report.overall_metrics['mean_hit_rate']:.2%}")
        print(f"  Median Hit Rate: {report.overall_metrics['median_hit_rate']:.2%}")
        print(f"  Mean MRR: {report.overall_metrics['mean_mrr']:.3f}")
        if report.overall_metrics['mean_rank']:
            print(f"  Mean Rank (when found): {report.overall_metrics['mean_rank']:.2f}")
        print(f"  Perfect Hit Rate (100%): {report.overall_metrics['perfect_hit_rate']:.2%}")
        print(f"  Zero Hit Rate (0%): {report.overall_metrics['zero_hit_rate']:.2%}")
        print("="*60 + "\n")

    def analyze_failures(self, report: ChunkEvaluationReport, top_n: int = 10):
        """Analyze chunks with poor performance"""
        print("\n" + "="*60)
        print("[SEARCH] FAILURE ANALYSIS")
        print("="*60)

        # Find worst performing chunks
        worst_chunks = sorted(report.evaluations, key=lambda e: e.hit_rate)[:top_n]

        print(f"Top {top_n} worst performing chunks:")
        for i, eval in enumerate(worst_chunks, 1):
            print(f"\n{i}. Chunk {eval.chunk_id} (Hit Rate: {eval.hit_rate:.0%}):")
            print(f"   Preview: {eval.chunk_preview}...")

            # Show failed queries
            failed_queries = [r.query for r in eval.search_results if not r.found_in_top_k]
            if failed_queries:
                print(f"   Failed queries: {', '.join(failed_queries[:3])}")

        # Analyze query patterns
        all_queries = []
        successful_queries = []
        failed_queries = []

        for eval in report.evaluations:
            for result in eval.search_results:
                all_queries.append(result.query)
                if result.found_in_top_k:
                    successful_queries.append(result.query)
                else:
                    failed_queries.append(result.query)

        print(f"\nQuery Success Statistics:")
        print(f"  Total queries: {len(all_queries)}")
        print(f"  Successful: {len(successful_queries)} ({100*len(successful_queries)/len(all_queries):.1f}%)")
        print(f"  Failed: {len(failed_queries)} ({100*len(failed_queries)/len(all_queries):.1f}%)")

        print("="*60 + "\n")

    def compare_models(self, reports: List[ChunkEvaluationReport]):
        """Compare performance across different embedding models"""
        print("\n" + "="*60)
        print("📊 MODEL COMPARISON")
        print("="*60)

        for report in reports:
            model = report.metadata['embedding_model']
            metrics = report.overall_metrics
            print(f"\n{model}:")
            print(f"  Hit Rate: {metrics['mean_hit_rate']:.2%}")
            print(f"  MRR: {metrics['mean_mrr']:.3f}")
            if metrics['mean_rank']:
                print(f"  Avg Rank: {metrics['mean_rank']:.2f}")

        print("="*60 + "\n")

    def save_report(self, report: ChunkEvaluationReport,
                   output_path: str = 'results/chunk_evaluation.json'):
        """Save evaluation report to JSON"""
        output_path = Path(output_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)

        # Convert to JSON-serializable format
        report_dict = {
            'metadata': report.metadata,
            'overall_metrics': report.overall_metrics,
            'timestamp': report.timestamp,
            'evaluations': []
        }

        for eval in report.evaluations:
            eval_dict = {
                'chunk_id': eval.chunk_id,
                'document_id': eval.document_id,
                'chunk_preview': eval.chunk_preview,
                'hit_rate': eval.hit_rate,
                'avg_rank': eval.avg_rank,
                'mrr': eval.mrr,
                'search_results': []
            }

            for result in eval.search_results:
                result_dict = {
                    'query': result.query,
                    'found': result.found_in_top_k,
                    'rank': result.rank,
                    'score': result.score
                }
                eval_dict['search_results'].append(result_dict)

            report_dict['evaluations'].append(eval_dict)

        with open(output_path, 'w', encoding='utf-8') as f:
            json.dump(report_dict, f, indent=2, ensure_ascii=False)

        print(f"[SAVE] Saved evaluation report to {output_path}")

    def save_csv_summary(self, report: ChunkEvaluationReport,
                        output_path: str = 'results/model_comparison.csv'):
        """Save summary metrics to CSV for model comparison"""
        output_path = Path(output_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)

        # Create summary row
        summary_data = {
            'model': report.metadata['embedding_model'],
            'use_ollama': report.metadata.get('use_ollama', False),
            'chunks_evaluated': report.overall_metrics['total_chunks_evaluated'],
            'top_k': report.overall_metrics['top_k'],
            'mean_hit_rate': report.overall_metrics['mean_hit_rate'],
            'median_hit_rate': report.overall_metrics['median_hit_rate'],
            'mean_mrr': report.overall_metrics['mean_mrr'],
            'mean_rank': report.overall_metrics.get('mean_rank'),
            'median_rank': report.overall_metrics.get('median_rank'),
            'perfect_hit_rate': report.overall_metrics['perfect_hit_rate'],
            'zero_hit_rate': report.overall_metrics['zero_hit_rate'],
            'evaluation_time': report.metadata['evaluation_time_seconds'],
            'chunks_per_second': report.metadata['chunks_per_second'],
            'timestamp': report.timestamp
        }

        # Convert to DataFrame with proper column order
        columns = ['model', 'use_ollama', 'chunks_evaluated', 'top_k', 'mean_hit_rate',
                  'median_hit_rate', 'mean_mrr', 'mean_rank', 'median_rank',
                  'perfect_hit_rate', 'zero_hit_rate', 'evaluation_time',
                  'chunks_per_second', 'timestamp']

        new_row_df = pd.DataFrame([summary_data], columns=columns)

        # If file exists, load and update; otherwise create new
        if output_path.exists():
            try:
                existing_df = pd.read_csv(output_path)

                # Check if this exact model configuration already exists
                mask = (existing_df['model'] == summary_data['model']) & \
                       (existing_df['use_ollama'] == summary_data['use_ollama']) & \
                       (existing_df['top_k'] == summary_data['top_k'])

                if mask.any():
                    # Update existing row
                    for col in columns:
                        existing_df.loc[mask, col] = summary_data[col]
                    df = existing_df
                    print(f"[EDIT] Updated existing entry for {summary_data['model']}")
                else:
                    # Append new row
                    df = pd.concat([existing_df, new_row_df], ignore_index=True)
                    print(f"[ADD] Added new entry for {summary_data['model']}")

            except Exception as e:
                print(f"[WARN] Error reading existing CSV, creating new: {e}")
                df = new_row_df
        else:
            df = new_row_df
            print(f"[FILE] Created new comparison file for {summary_data['model']}")

        # Sort by hit rate descending for easy comparison
        df = df.sort_values('mean_hit_rate', ascending=False)

        # Save with headers
        df.to_csv(output_path, index=False)

        # Show current rankings
        print(f"\n📊 Current Model Rankings (by hit rate):")
        for i, row in df.iterrows():
            print(f"  {i+1}. {row['model']}: {row['mean_hit_rate']:.2%}")
        print(f"[SAVE] Saved model comparison to {output_path}")

    def save_detailed_csv(self, report: ChunkEvaluationReport,
                         output_path: str = 'results/detailed_results.csv'):
        """Save detailed per-chunk results to CSV"""
        output_path = Path(output_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)

        detailed_data = []
        for eval in report.evaluations:
            for result in eval.search_results:
                detailed_data.append({
                    'model': report.metadata['embedding_model'],
                    'use_ollama': report.metadata.get('use_ollama', False),
                    'top_k': report.overall_metrics['top_k'],
                    'chunk_id': eval.chunk_id,
                    'document_id': eval.document_id,
                    'chunk_preview': eval.chunk_preview[:100],
                    'query': result.query,
                    'found_in_top_k': result.found_in_top_k,
                    'rank': result.rank,
                    'similarity_score': result.score,
                    'chunk_hit_rate': eval.hit_rate,
                    'chunk_mrr': eval.mrr,
                    'timestamp': report.timestamp
                })

        df = pd.DataFrame(detailed_data)
        df.to_csv(output_path, index=False)
        print(f"[SAVE] Saved detailed results to {output_path}")

    def load_report(self, input_path: str = 'results/chunk_evaluation.json') -> Optional[ChunkEvaluationReport]:
        """Load evaluation report from JSON"""
        input_path = Path(input_path)

        if not input_path.exists():
            print(f"[WARN] File not found: {input_path}")
            return None

        with open(input_path, 'r', encoding='utf-8') as f:
            data = json.load(f)

        # Reconstruct report
        evaluations = []
        for eval_data in data['evaluations']:
            search_results = []
            for result_data in eval_data['search_results']:
                search_results.append(ChunkSearchResult(
                    query=result_data['query'],
                    target_chunk_id=eval_data['chunk_id'],
                    found_in_top_k=result_data['found'],
                    rank=result_data.get('rank'),
                    score=result_data.get('score'),
                    top_chunks=[]  # Not saved for space
                ))

            evaluations.append(ChunkEvaluation(
                chunk_id=eval_data['chunk_id'],
                document_id=eval_data['document_id'],
                chunk_preview=eval_data['chunk_preview'],
                search_results=search_results,
                hit_rate=eval_data['hit_rate'],
                avg_rank=eval_data.get('avg_rank'),
                mrr=eval_data['mrr']
            ))

        report = ChunkEvaluationReport(
            metadata=data['metadata'],
            evaluations=evaluations,
            overall_metrics=data['overall_metrics'],
            timestamp=data['timestamp']
        )

        print(f"[FOLDER] Loaded evaluation report from {input_path}")
        return report


def main():
    """Test the chunk evaluator"""
    import click

    @click.command()
    @click.option('--chunks', default='data/quality_chunks.json', help='Quality chunks file')
    @click.option('--terms', default='data/chunk_terms.json', help='Search terms file')
    @click.option('--embedding-model', default='all-MiniLM-L6-v2', help='Embedding model')
    @click.option('--ollama', is_flag=True, help='Use Ollama embeddings')
    @click.option('--top-k', default=10, help='Top-K for evaluation')
    @click.option('--analyze', is_flag=True, help='Analyze failures')
    def test(chunks, terms, embedding_model, ollama, top_k, analyze):
        """Test chunk evaluation"""

        from chunk_quality_filter import ChunkQualityFilter
        from chunk_query_generator import ChunkQueryGenerator

        # Load chunks and terms
        filter = ChunkQualityFilter()
        quality_chunks = filter.load_filtered_chunks(chunks)

        generator = ChunkQueryGenerator()
        terms_dict = generator.load_search_terms(terms)

        if not quality_chunks or not terms_dict:
            print("Missing data. Run quality filtering and term generation first.")
            return

        # Initialize evaluator
        evaluator = ChunkVectorEvaluator(
            embedding_model=embedding_model,
            use_ollama=ollama
        )

        # Index chunks
        evaluator.index_chunks(quality_chunks)

        # Evaluate
        report = evaluator.evaluate_all_chunks(quality_chunks, terms_dict, top_k=top_k)

        # Save report
        evaluator.save_report(report)

        # Analyze if requested
        if analyze:
            evaluator.analyze_failures(report)

    test()


if __name__ == '__main__':
    main()