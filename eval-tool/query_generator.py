import ollama
import json
from pathlib import Path
from typing import List, Dict, Any
from tqdm import tqdm
import math

class QueryGenerator:
    def __init__(self, model: str = "qwen3:4b"):
        self.model = model
        self.client = ollama.Client()
        
    def generate_queries_for_bookmark(self, bookmark: Dict[str, Any], max_queries: int = 5) -> List[str]:
        """Generate search queries that should return this bookmark"""
        
        content = bookmark.get('content', '')
        title = bookmark.get('name', '')
        url = bookmark.get('url', '')
        
        if not content:
            return []
        
        # Calculate number of queries based on content length (1 per 200 chars, max 5)
        content_length = len(content)
        num_queries = min(max(1, content_length // 200), max_queries)
        
        prompt = f"""Based on the following webpage, generate {num_queries} different search queries that someone might use to find this content.
Each query should be on a new line and be a realistic search query someone would type.
Generate one or two queries for simple searches with 1 or 2 words only.
Also generate up to 3 longer (3-6 words) question form queries.
There must be at least 1 short query.

Title: {title}
URL: {url}
Content (first 2000 chars): {content[:2000]}

Generate exactly {num_queries} queries:"""

        try:
            response = self.client.generate(
                model=self.model,
                prompt=prompt,
                options={
                    'temperature': 0.7,
                    'top_p': 0.9,
                }
            )
            
            # Parse the response to extract queries
            queries_text = response['response'].strip()
            queries = [q.strip() for q in queries_text.split('\n') if q.strip()]
            
            # Clean up queries (remove numbering if present)
            cleaned_queries = []
            for q in queries[:num_queries]:
                # Remove common numbering patterns
                q = q.lstrip('0123456789.-) ')
                if q:
                    cleaned_queries.append(q)
            
            return cleaned_queries[:num_queries]
            
        except Exception as e:
            print(f"Error generating queries for bookmark {bookmark.get('id')}: {e}")
            return []
    
    def generate_queries_for_samples(self, samples: List[Dict[str, Any]], output_path: str = "data/generated_queries.json", resume: bool = True) -> Dict[str, List[str]]:
        """Generate queries for all sampled bookmarks with incremental saving"""
        
        output_file = Path(output_path)
        output_file.parent.mkdir(parents=True, exist_ok=True)
        
        # Load existing queries if resuming
        existing_queries = {}
        if resume and output_file.exists():
            try:
                with open(output_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                    existing_queries = data.get('queries', {})
                    print(f"Resuming from existing file with {len(existing_queries)} bookmarks already processed")
            except Exception as e:
                print(f"Could not load existing queries: {e}")
        
        queries_map = existing_queries.copy()
        
        # Filter out already processed bookmarks
        bookmarks_to_process = []
        for bookmark in samples:
            bookmark_id = bookmark.get('id', bookmark.get('guid', str(hash(bookmark['url']))))
            if bookmark_id not in queries_map:
                bookmarks_to_process.append(bookmark)
        
        if not bookmarks_to_process:
            print(f"All {len(samples)} bookmarks already have queries generated")
            return queries_map
        
        print(f"Generating queries for {len(bookmarks_to_process)} bookmarks (skipping {len(samples) - len(bookmarks_to_process)} already processed)...")
        
        for bookmark in tqdm(bookmarks_to_process):
            bookmark_id = bookmark.get('id', bookmark.get('guid', str(hash(bookmark['url']))))
            queries = self.generate_queries_for_bookmark(bookmark)
            
            if queries:
                queries_map[bookmark_id] = queries
                
                # Save incrementally after each bookmark
                self._save_queries_incremental(queries_map, output_file)
                
                print(f"Generated {len(queries)} queries for bookmark {bookmark_id} (total: {len(queries_map)}/{len(samples)})")
        
        print(f"Generated queries for {len(queries_map)} bookmarks total")
        return queries_map
    
    def _save_queries_incremental(self, queries_map: Dict[str, List[str]], output_file: Path):
        """Save queries incrementally to disk"""
        data = {
            'model': self.model,
            'total_bookmarks': len(queries_map),
            'total_queries': sum(len(queries) for queries in queries_map.values()),
            'queries': queries_map
        }
        
        # Write to temporary file first, then rename (atomic operation)
        temp_file = output_file.with_suffix('.tmp')
        with open(temp_file, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
        
        # Rename temp file to actual file
        temp_file.replace(output_file)
    
    def save_queries(self, queries_map: Dict[str, List[str]], output_path: str = "data/generated_queries.json"):
        """Save generated queries to file"""
        output_file = Path(output_path)
        output_file.parent.mkdir(parents=True, exist_ok=True)
        
        # Include metadata
        data = {
            'model': self.model,
            'total_bookmarks': len(queries_map),
            'total_queries': sum(len(queries) for queries in queries_map.values()),
            'queries': queries_map
        }
        
        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
        
        print(f"Saved queries to {output_file}")
        return output_file
    
    def load_queries(self, input_path: str = "data/generated_queries.json") -> Dict[str, List[str]]:
        """Load queries from file"""
        with open(input_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
            return data.get('queries', data)