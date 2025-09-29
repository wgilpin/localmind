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
import csv

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

        # Timing tracking
        self.embedding_times = []
        self.new_embeddings_count = 0  # Track newly generated embeddings in this session

        # Print instruction prefix info
        prefixes = self._get_instruction_prefixes()
        if prefixes['query_prefix'] or prefixes['document_prefix']:
            print(f"Using instruction-aware embedding with prefixes:")
            if prefixes['query_prefix']:
                print(f"  Query: '{prefixes['query_prefix']}...'")
            if prefixes['document_prefix']:
                print(f"  Document: '{prefixes['document_prefix']}...'")
        else:
            print("Using standard embedding (no instruction prefixes)")

    def _get_instruction_prefixes(self):
        """Get appropriate instruction prefixes for the embedding model"""
        model_name = self.embedding_model_name.lower()

        # Remove -gpu suffix for checking base model type
        base_model_name = model_name.replace("-gpu", "")

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
            # SentenceTransformers and other models
            return {
                'query_prefix': "",
                'document_prefix': ""
            }

    def _format_text_for_embedding(self, text: str, is_query: bool = False, title: str = ""):
        """Format text with appropriate prefixes for the embedding model"""
        prefixes = self._get_instruction_prefixes()

        if is_query:
            return prefixes['query_prefix'] + text
        else:
            if "embeddinggemma" in self.embedding_model_name.lower():
                # EmbeddingGemma needs title in the prefix
                prefix = prefixes['document_prefix'].format(title=title if title else "none")
                return prefix + text
            else:
                return prefixes['document_prefix'] + text

    def index_bookmarks(self, bookmarks: List[Dict[str, Any]], save_every: int = 10):
        """Index bookmarks into vector storage with incremental saves"""
        print(f"Indexing {len(bookmarks)} bookmarks (saving every {save_every} embeddings)...")

        # Load existing data if available
        existing_embeddings = None
        existing_metadata = None

        if self.load_vectors():
            print(f"Found {len(self.vectors_df)} existing embeddings, continuing from there...")
            existing_embeddings = self.vectors_df
            existing_metadata = self.metadata_df

            # Find bookmarks already processed
            existing_ids = set(existing_metadata['bookmark_id'].values)
            bookmarks = [b for b in bookmarks if str(b.get('id', b.get('guid', str(hash(b['url']))))) not in existing_ids]
            print(f"Skipping {len(existing_ids)} already processed bookmarks, {len(bookmarks)} remaining")

            if not bookmarks:
                print("All bookmarks already processed!")
                return

        rows = []
        embeddings = []

        for i, bookmark in enumerate(tqdm(bookmarks)):
            bookmark_id = bookmark.get('id', bookmark.get('guid', str(hash(bookmark['url']))))
            content = bookmark.get('content', '')
            title = bookmark.get('name', '')

            if not content:
                continue

            # Combine title and content for embedding
            text_to_embed = f"{title}\n\n{content}"

            # Format text with appropriate instruction prefix for documents
            formatted_text = self._format_text_for_embedding(text_to_embed, is_query=False, title=title)

            # Generate embedding with timing
            start_time = datetime.now()
            embedding = self.embedding_model.encode(formatted_text)
            end_time = datetime.now()
            embedding_time = (end_time - start_time).total_seconds()
            self.embedding_times.append(embedding_time)
            self.new_embeddings_count += 1

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

            # Incremental save every N embeddings
            if (i + 1) % save_every == 0 and embeddings:
                self._save_partial_embeddings(embeddings, rows, existing_embeddings, existing_metadata)
                # Reset for next batch
                embeddings = []
                rows = []
                # Reload to get updated existing data
                self.load_vectors()
                existing_embeddings = self.vectors_df
                existing_metadata = self.metadata_df

        # Save any remaining embeddings
        if embeddings:
            self._save_partial_embeddings(embeddings, rows, existing_embeddings, existing_metadata)

        # Final load to ensure everything is in memory
        self.load_vectors()
        print(f"Indexed {len(self.vectors_df)} total documents")

    def _save_partial_embeddings(self, new_embeddings: List, new_rows: List, existing_embeddings=None, existing_metadata=None):
        """Save a batch of embeddings, combining with existing data"""
        if not new_embeddings:
            return

        # Create DataFrames for new data
        embeddings_array = np.vstack(new_embeddings)
        embedding_columns = [f'emb_{i}' for i in range(embeddings_array.shape[1])]
        new_embeddings_df = pd.DataFrame(embeddings_array, columns=embedding_columns)
        new_embeddings_df['bookmark_id'] = [row['bookmark_id'] for row in new_rows]
        new_metadata_df = pd.DataFrame(new_rows)

        # Combine with existing data if available
        if existing_embeddings is not None and len(existing_embeddings) > 0:
            combined_embeddings_df = pd.concat([existing_embeddings, new_embeddings_df], ignore_index=True)
            combined_metadata_df = pd.concat([existing_metadata, new_metadata_df], ignore_index=True)
        else:
            combined_embeddings_df = new_embeddings_df
            combined_metadata_df = new_metadata_df

        # Save combined data
        try:
            combined_embeddings_df.to_parquet(self.embeddings_path, compression='snappy')
            combined_metadata_df.to_parquet(self.metadata_path, compression='snappy')
            print(f"Saved {len(combined_embeddings_df)} embeddings to disk (Parquet)")
        except ImportError:
            embeddings_pkl = self.persist_directory / "embeddings.pkl"
            metadata_pkl = self.persist_directory / "metadata.pkl"
            combined_embeddings_df.to_pickle(embeddings_pkl)
            combined_metadata_df.to_pickle(metadata_pkl)
            print(f"Saved {len(combined_embeddings_df)} embeddings to disk (Pickle)")

    def load_vectors(self):
        """Load vectors from disk if they exist"""
        # Try parquet first
        if self.embeddings_path.exists() and self.metadata_path.exists():
            try:
                self.vectors_df = pd.read_parquet(self.embeddings_path)
                self.metadata_df = pd.read_parquet(self.metadata_path)
                print(f"Loaded {len(self.vectors_df)} embeddings from Parquet")
                return True
            except Exception as e:
                print(f"Failed to load Parquet files: {e}")

        # Try pickle fallback
        embeddings_pkl = self.persist_directory / "embeddings.pkl"
        metadata_pkl = self.persist_directory / "metadata.pkl"
        if embeddings_pkl.exists() and metadata_pkl.exists():
            self.vectors_df = pd.read_pickle(embeddings_pkl)
            self.metadata_df = pd.read_pickle(metadata_pkl)
            print(f"Loaded {len(self.vectors_df)} embeddings from Pickle")
            return True

        return False

    def search(self, query: str, n_results: int = 10) -> List[Dict[str, Any]]:
        """Search for documents matching the query using cosine similarity"""
        if self.vectors_df is None:
            if not self.load_vectors():
                raise ValueError("No embeddings found. Index bookmarks first.")

        # Format query with appropriate instruction prefix
        formatted_query = self._format_text_for_embedding(query, is_query=True)

        # Generate query embedding
        query_embedding = self.embedding_model.encode(formatted_query)
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

        # Calculate timing metrics (only for newly generated embeddings in this session)
        avg_embedding_time = sum(self.embedding_times) / len(self.embedding_times) if self.embedding_times else None
        total_embedding_time = sum(self.embedding_times) if self.embedding_times else None
        new_embeddings_count = self.new_embeddings_count

        # Calculate metrics
        results['metrics'] = {
            'recall_at_k': found_count / total_queries if total_queries > 0 else 0,
            'mean_rank': sum(all_ranks) / len(all_ranks) if all_ranks else None,
            'median_rank': sorted(all_ranks)[len(all_ranks)//2] if all_ranks else None,
            'mean_distance': sum(all_distances) / len(all_distances) if all_distances else None,
            'mean_similarity': sum(all_similarities) / len(all_similarities) if all_similarities else None,
            'queries_found': found_count,
            'queries_total': total_queries,
            'mrr': self._calculate_mrr(results['evaluations']),  # Mean Reciprocal Rank
            'avg_embedding_time_seconds': avg_embedding_time,
            'total_embedding_time_seconds': total_embedding_time,
            'new_embeddings_generated': new_embeddings_count,
            'total_embeddings_in_store': len(self.vectors_df) if self.vectors_df is not None else 0
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

        # Convert numpy types to native Python types for JSON serialization
        results = self._convert_numpy_types(results)

        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(results, f, indent=2, ensure_ascii=False)

        print(f"Saved results to {output_file}")

        # Also save a summary CSV
        self._save_summary_csv(results, output_file.parent / "evaluation_summary.csv")

        # Save to model comparison CSV
        self._append_to_comparison_csv(results, output_file.parent / "model_comparison.csv")

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

    def _append_to_comparison_csv(self, results: Dict[str, Any], output_path: Path):
        """Append results as a single row to comparison CSV for comparing multiple models"""
        metrics = results['metrics']

        # Create row with all important metrics
        row = {
            'timestamp': results['timestamp'],
            'model': results['embedding_model'],
            'use_ollama': results['use_ollama'],
            'total_queries': results['total_queries'],
            'total_bookmarks': results['total_bookmarks'],
            'queries_found': metrics['queries_found'],
            'recall_at_20': metrics['recall_at_k'],
            'recall_at_1': metrics.get('recall_at_1', 0),
            'recall_at_3': metrics.get('recall_at_3', 0),
            'recall_at_5': metrics.get('recall_at_5', 0),
            'recall_at_10': metrics.get('recall_at_10', 0),
            'mean_rank': metrics.get('mean_rank', None),
            'median_rank': metrics.get('median_rank', None),
            'mean_distance': metrics.get('mean_distance', None),
            'mean_similarity': metrics.get('mean_similarity', None),
            'mrr': metrics['mrr'],
            'avg_embedding_time_seconds': metrics.get('avg_embedding_time_seconds', None),
            'total_embedding_time_seconds': metrics.get('total_embedding_time_seconds', None),
            'new_embeddings_generated': metrics.get('new_embeddings_generated', None),
            'total_embeddings_in_store': metrics.get('total_embeddings_in_store', None)
        }

        # Check if file exists and has content to determine if we need headers
        file_exists = output_path.exists()
        file_has_content = file_exists and output_path.stat().st_size > 0

        # Write row to CSV
        with open(output_path, 'a', newline='', encoding='utf-8') as f:
            writer = csv.DictWriter(f, fieldnames=row.keys())

            # Write header if file is new or empty
            if not file_has_content:
                writer.writeheader()

            writer.writerow(row)

        print(f"Appended results to model comparison CSV: {output_path}")

    def _convert_numpy_types(self, obj):
        """Convert numpy types to native Python types for JSON serialization"""
        if isinstance(obj, np.generic):
            return obj.item()
        elif isinstance(obj, dict):
            return {key: self._convert_numpy_types(value) for key, value in obj.items()}
        elif isinstance(obj, list):
            return [self._convert_numpy_types(element) for element in obj]
        else:
            return obj

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

        # Print timing information
        if metrics.get('avg_embedding_time_seconds') is not None:
            print(f"\nEmbedding Generation (this session):")
            print(f"New Embeddings Generated: {metrics['new_embeddings_generated']}")
            print(f"Total Embeddings in Store: {metrics['total_embeddings_in_store']}")
            print(f"Average Time per Embedding: {metrics['avg_embedding_time_seconds']:.2f} seconds")
            print(f"Total Embedding Time (this session): {metrics['total_embedding_time_seconds']:.2f} seconds")

        print("="*60)