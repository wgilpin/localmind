import chromadb
from chromadb.config import Settings
from sentence_transformers import SentenceTransformer
import json
from pathlib import Path
from typing import List, Dict, Any, Tuple
from tqdm import tqdm
import pandas as pd
from datetime import datetime

class ChromaEvaluator:
    def __init__(self, persist_directory: str = "./chroma_db", embedding_model: str = "all-MiniLM-L6-v2"):
        self.persist_directory = Path(persist_directory)
        self.persist_directory.mkdir(parents=True, exist_ok=True)
        
        # Initialize ChromaDB client
        self.client = chromadb.PersistentClient(
            path=str(self.persist_directory),
            settings=Settings(
                anonymized_telemetry=False,
                allow_reset=True
            )
        )
        
        # Initialize embedding model
        self.embedding_model = SentenceTransformer(embedding_model)
        self.collection = None
        
    def create_collection(self, collection_name: str = "bookmarks_eval"):
        """Create or get a collection for evaluation"""
        try:
            # Delete existing collection if it exists
            self.client.delete_collection(name=collection_name)
        except:
            pass
        
        # Create new collection
        self.collection = self.client.create_collection(
            name=collection_name,
            metadata={"hnsw:space": "cosine"}
        )
        
        print(f"Created collection '{collection_name}'")
        
    def index_bookmarks(self, bookmarks: List[Dict[str, Any]]):
        """Index bookmarks into ChromaDB"""
        if not self.collection:
            self.create_collection()
        
        print(f"Indexing {len(bookmarks)} bookmarks...")
        
        for bookmark in tqdm(bookmarks):
            bookmark_id = bookmark.get('id', bookmark.get('guid', str(hash(bookmark['url']))))
            content = bookmark.get('content', '')
            title = bookmark.get('name', '')
            
            if not content:
                continue
            
            # Combine title and content for embedding
            text_to_embed = f"{title}\n\n{content}"
            
            # Generate embedding
            embedding = self.embedding_model.encode(text_to_embed).tolist()
            
            # Add to collection
            self.collection.add(
                ids=[str(bookmark_id)],
                embeddings=[embedding],
                documents=[text_to_embed],
                metadatas=[{
                    'title': title,
                    'url': bookmark['url'],
                    'content_length': len(content)
                }]
            )
        
        print(f"Indexed {self.collection.count()} documents")
        
    def search(self, query: str, n_results: int = 10) -> List[Dict[str, Any]]:
        """Search for documents matching the query"""
        if not self.collection:
            raise ValueError("Collection not initialized. Index bookmarks first.")
        
        # Generate query embedding
        query_embedding = self.embedding_model.encode(query).tolist()
        
        # Search
        results = self.collection.query(
            query_embeddings=[query_embedding],
            n_results=n_results
        )
        
        # Format results
        formatted_results = []
        if results['ids'] and results['ids'][0]:
            for i in range(len(results['ids'][0])):
                formatted_results.append({
                    'id': results['ids'][0][i],
                    'distance': results['distances'][0][i] if results['distances'] else None,
                    'metadata': results['metadatas'][0][i] if results['metadatas'] else {},
                    'document': results['documents'][0][i][:200] if results['documents'] else ''
                })
        
        return formatted_results
    
    def evaluate_queries(self, queries_map: Dict[str, List[str]], top_k: int = 20) -> Dict[str, Any]:
        """Evaluate all queries and calculate metrics"""
        
        results = {
            'timestamp': datetime.now().isoformat(),
            'total_queries': sum(len(queries) for queries in queries_map.values()),
            'total_bookmarks': len(queries_map),
            'top_k': top_k,
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
                
                for i, result in enumerate(search_results):
                    if result['id'] == str(bookmark_id):
                        rank = i + 1
                        distance = result['distance']
                        break
                
                query_result = {
                    'query': query,
                    'found': rank is not None,
                    'rank': rank,
                    'distance': distance,
                    'top_result_id': search_results[0]['id'] if search_results else None,
                    'top_result_distance': search_results[0]['distance'] if search_results else None
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
        
        # Calculate metrics
        results['metrics'] = {
            'recall_at_k': found_count / total_queries if total_queries > 0 else 0,
            'mean_rank': sum(all_ranks) / len(all_ranks) if all_ranks else None,
            'median_rank': sorted(all_ranks)[len(all_ranks)//2] if all_ranks else None,
            'mean_distance': sum(all_distances) / len(all_distances) if all_distances else None,
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
                    'distance': query_result['distance']
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
        
        print(f"Mean Reciprocal Rank (MRR): {metrics['mrr']:.4f}")
        print("="*60)