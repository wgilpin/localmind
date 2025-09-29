import numpy as np
import pandas as pd
from sentence_transformers import SentenceTransformer
from ollama_embedding import OllamaEmbedding
import json
from pathlib import Path
from typing import List, Dict, Any, Tuple, Union
from tqdm import tqdm
from datetime import datetime
from sklearn.metrics.pairwise import cosine_similarity

class VectorEvaluator:
    def __init__(self, persist_directory: str = "./vector_store", embedding_model: str = "all-MiniLM-L6-v2", use_ollama: bool = False, ollama_url: str = "http://localhost:11434"):
        self.persist_directory = Path(persist_directory)
        self.persist_directory.mkdir(parents=True, exist_ok=True)
        self.use_ollama = use_ollama
        self.embedding_model_name = embedding_model

        # Initialize embedding model
        if use_ollama:
            self.embedding_model = OllamaEmbedding(embedding_model, ollama_url)
            print(f"Using Ollama embedding model: {embedding_model}")
        else:
            self.embedding_model = SentenceTransformer(embedding_model)
            print(f"Using SentenceTransformer model: {embedding_model}")

        # Vector storage
        self.vectors_df = None
        self.embeddings_path = self.persist_directory / "embeddings.parquet"
        self.metadata_path = self.persist_directory / "metadata.parquet"

    def index_bookmarks(self, bookmarks: List[Dict[str, Any]]):
        """Index bookmarks into vector storage"""
        print(f"Indexing {len(bookmarks)} bookmarks...")

        rows = []
        embeddings = []

        for bookmark in tqdm(bookmarks):
            bookmark_id = bookmark.get('id', bookmark.get('guid', str(hash(bookmark['url']))))
            content = bookmark.get('content', '')
            title = bookmark.get('name', '')

            if not content:
                continue

            # Combine title and content for embedding
            text_to_embed = f"{title}\n\n{content}"

            # Generate embedding
            embedding = self.embedding_model.encode(text_to_embed)

            # Ensure embedding is a numpy array
            if not isinstance(embedding, np.ndarray):
                embedding = np.array(embedding)

            # Store metadata
            rows.append({
                'bookmark_id': str(bookmark_id),
                'title': title,
                'url': bookmark['url'],
                'content_length': len(content),
                'document': text_to_embed
            })

            embeddings.append(embedding)

        # Create embeddings DataFrame
        embeddings_array = np.vstack(embeddings)
        embedding_columns = [f'emb_{i}' for i in range(embeddings_array.shape[1])]
        embeddings_df = pd.DataFrame(embeddings_array, columns=embedding_columns)
        embeddings_df['bookmark_id'] = [row['bookmark_id'] for row in rows]

        # Create metadata DataFrame
        metadata_df = pd.DataFrame(rows)

        # Save to disk efficiently
        print(f"Saving {len(embeddings_df)} embeddings to disk...")
        try:
            embeddings_df.to_parquet(self.embeddings_path, compression='snappy')
            metadata_df.to_parquet(self.metadata_path, compression='snappy')
            print("Saved as Parquet files")
        except ImportError as e:
            print(f"Parquet not available ({e}), using pickle fallback...")
            embeddings_pkl = self.persist_directory / "embeddings.pkl"
            metadata_pkl = self.persist_directory / "metadata.pkl"
            embeddings_df.to_pickle(embeddings_pkl)
            metadata_df.to_pickle(metadata_pkl)
            print("Saved as pickle files")

        # Store in memory for search
        self.vectors_df = embeddings_df
        self.metadata_df = metadata_df

        print(f"Indexed {len(self.vectors_df)} documents")

    def load_vectors(self):
        """Load vectors from disk if they exist"""
        if self.embeddings_path.exists() and self.metadata_path.exists():
            print("Loading existing embeddings from disk...")
            self.vectors_df = pd.read_parquet(self.embeddings_path)
            self.metadata_df = pd.read_parquet(self.metadata_path)
            print(f"Loaded {len(self.vectors_df)} embeddings")
            return True
        return False

    def search(self, query: str, n_results: int = 10) -> List[Dict[str, Any]]:
        """Search for documents matching the query using cosine similarity"""
        if self.vectors_df is None:
            if not self.load_vectors():
                raise ValueError("No embeddings found. Index bookmarks first.")

        # Generate query embedding
        query_embedding = self.embedding_model.encode(query)
        if not isinstance(query_embedding, np.ndarray):
            query_embedding = np.array(query_embedding)

        # Get embedding columns
        embedding_cols = [col for col in self.vectors_df.columns if col.startswith('emb_')]
        document_embeddings = self.vectors_df[embedding_cols].values

        # Compute cosine similarities
        similarities = cosine_similarity([query_embedding], document_embeddings)[0]

        # Get top results
        top_indices = np.argsort(similarities)[::-1][:n_results]

        # Format results
        results = []
        for idx in top_indices:
            bookmark_id = self.vectors_df.iloc[idx]['bookmark_id']
            similarity = similarities[idx]
            distance = 1.0 - similarity  # Convert similarity to distance

            # Get metadata
            metadata_row = self.metadata_df[self.metadata_df['bookmark_id'] == bookmark_id].iloc[0]

            results.append({
                'id': bookmark_id,
                'distance': distance,
                'similarity': similarity,
                'metadata': {
                    'title': metadata_row['title'],
                    'url': metadata_row['url'],
                    'content_length': metadata_row['content_length']
                },
                'document': metadata_row['document'][:200] if len(metadata_row['document']) > 200 else metadata_row['document']
            })

        return results

    def evaluate_queries(self, queries_map: Dict[str, List[str]], top_k: int = 20) -> Dict[str, Any]:
        """Evaluate all queries and calculate metrics"""

        results = {
            'timestamp': datetime.now().isoformat(),
            'total_queries': sum(len(queries) for queries in queries_map.values()),
            'total_bookmarks': len(queries_map),
            'top_k': top_k,
            'embedding_model': self.embedding_model_name,
            'use_ollama': self.use_ollama,
            'evaluations': []
        }

        print(f"Evaluating {results['total_queries']} queries for {results['total_bookmarks']} bookmarks...")

        for bookmark_id, queries in tqdm(queries_map.items()):
            bookmark_results = {
                'bookmark_id': bookmark_id,
                'queries': []
            }

            for query in queries:
                search_results = self.search(query, n_results=top_k)

                # Find the rank of the target bookmark
                rank = None
                distance = None
                similarity = None

                for i, result in enumerate(search_results):
                    if result['id'] == str(bookmark_id):
                        rank = i + 1
                        distance = result['distance']
                        similarity = result['similarity']
                        break

                query_result = {
                    'query': query,
                    'found': rank is not None,
                    'rank': rank,
                    'distance': distance,
                    'similarity': similarity,
                    'top_result_id': search_results[0]['id'] if search_results else None,
                    'top_result_distance': search_results[0]['distance'] if search_results else None,
                    'top_result_similarity': search_results[0]['similarity'] if search_results else None
                }

                bookmark_results['queries'].append(query_result)

            results['evaluations'].append(bookmark_results)

        # Calculate summary statistics
        self._calculate_metrics(results)

        return results

    def _calculate_metrics(self, results: Dict[str, Any]):
        """Calculate evaluation metrics"""

        all_ranks = []
        all_distances = []
        all_similarities = []
        found_count = 0
        total_queries = 0

        for eval in results['evaluations']:
            for query_result in eval['queries']:
                total_queries += 1
                if query_result['found']:
                    found_count += 1
                    all_ranks.append(query_result['rank'])
                    if query_result['distance'] is not None:
                        all_distances.append(query_result['distance'])
                    if query_result['similarity'] is not None:
                        all_similarities.append(query_result['similarity'])

        # Calculate metrics
        results['metrics'] = {
            'recall_at_k': found_count / total_queries if total_queries > 0 else 0,
            'mean_rank': sum(all_ranks) / len(all_ranks) if all_ranks else None,
            'median_rank': sorted(all_ranks)[len(all_ranks)//2] if all_ranks else None,
            'mean_distance': sum(all_distances) / len(all_distances) if all_distances else None,
            'mean_similarity': sum(all_similarities) / len(all_similarities) if all_similarities else None,
            'queries_found': found_count,
            'queries_total': total_queries,
            'mrr': self._calculate_mrr(results['evaluations'])  # Mean Reciprocal Rank
        }

        # Calculate recall at different k values
        for k in [1, 3, 5, 10, 20]:
            if k <= results['top_k']:
                count_at_k = sum(1 for r in all_ranks if r <= k)
                results['metrics'][f'recall_at_{k}'] = count_at_k / total_queries if total_queries > 0 else 0

    def _calculate_mrr(self, evaluations: List[Dict]) -> float:
        """Calculate Mean Reciprocal Rank"""
        reciprocal_ranks = []

        for eval in evaluations:
            for query_result in eval['queries']:
                if query_result['found'] and query_result['rank']:
                    reciprocal_ranks.append(1.0 / query_result['rank'])
                else:
                    reciprocal_ranks.append(0.0)

        return sum(reciprocal_ranks) / len(reciprocal_ranks) if reciprocal_ranks else 0.0

    def save_results(self, results: Dict[str, Any], output_path: str = "results/evaluation_results.json"):
        """Save evaluation results"""
        output_file = Path(output_path)
        output_file.parent.mkdir(parents=True, exist_ok=True)

        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(results, f, indent=2, ensure_ascii=False)

        print(f"Saved results to {output_file}")

        # Also save a summary CSV
        self._save_summary_csv(results, output_file.parent / "evaluation_summary.csv")

        return output_file

    def _save_summary_csv(self, results: Dict[str, Any], output_path: Path):
        """Save a CSV summary of the results"""

        rows = []
        for eval in results['evaluations']:
            for query_result in eval['queries']:
                rows.append({
                    'bookmark_id': eval['bookmark_id'],
                    'query': query_result['query'],
                    'found': query_result['found'],
                    'rank': query_result['rank'],
                    'distance': query_result['distance'],
                    'similarity': query_result['similarity']
                })

        df = pd.DataFrame(rows)
        df.to_csv(output_path, index=False)
        print(f"Saved summary CSV to {output_path}")

    def print_metrics_summary(self, results: Dict[str, Any]):
        """Print a formatted summary of the metrics"""

        metrics = results['metrics']

        print("\n" + "="*60)
        print("EVALUATION METRICS SUMMARY")
        print("="*60)
        print(f"Embedding Model: {results.get('embedding_model', 'unknown')}")
        print(f"Using Ollama: {results.get('use_ollama', False)}")
        print(f"Total Bookmarks: {results['total_bookmarks']}")
        print(f"Total Queries: {results['total_queries']}")
        print(f"Queries Found: {metrics['queries_found']} / {metrics['queries_total']}")
        print(f"\nRecall@{results['top_k']}: {metrics['recall_at_k']:.2%}")

        for k in [1, 3, 5, 10, 20]:
            if f'recall_at_{k}' in metrics:
                print(f"Recall@{k}: {metrics[f'recall_at_{k}']:.2%}")

        if metrics['mean_rank']:
            print(f"\nMean Rank: {metrics['mean_rank']:.2f}")
            print(f"Median Rank: {metrics['median_rank']}")

        if metrics['mean_distance'] is not None:
            print(f"Mean Distance: {metrics['mean_distance']:.4f}")

        if metrics['mean_similarity'] is not None:
            print(f"Mean Similarity: {metrics['mean_similarity']:.4f}")

        print(f"Mean Reciprocal Rank (MRR): {metrics['mrr']:.4f}")
        print("="*60)