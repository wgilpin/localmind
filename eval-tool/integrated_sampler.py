import json
import random
from pathlib import Path
from typing import List, Dict, Any, Optional
import os
import requests
from datetime import datetime
from tqdm import tqdm
from bs4 import BeautifulSoup
import ollama

class IntegratedSampler:
    """Samples bookmarks and generates queries incrementally, saving after each step"""
    
    def __init__(self, bookmarks_path: str = None, model: str = "qwen3:4b"):
        if bookmarks_path is None:
            # Default Chrome bookmarks location on Windows
            local_app_data = os.getenv('LOCALAPPDATA')
            bookmarks_path = Path(local_app_data) / 'Google' / 'Chrome' / 'User Data' / 'Default' / 'Bookmarks'
        self.bookmarks_path = Path(bookmarks_path)
        self.model = model
        self.ollama_client = ollama.Client()
        
    def extract_bookmarks_from_folder(self, folder: Dict, bookmarks: List[Dict] = None) -> List[Dict]:
        if bookmarks is None:
            bookmarks = []
            
        if 'children' in folder:
            for item in folder['children']:
                if item['type'] == 'url':
                    bookmark = {
                        'id': item['id'],
                        'name': item['name'],
                        'url': item['url'],
                        'date_added': item.get('date_added'),
                        'guid': item.get('guid')
                    }
                    bookmarks.append(bookmark)
                elif item['type'] == 'folder':
                    self.extract_bookmarks_from_folder(item, bookmarks)
        
        return bookmarks
    
    def get_all_bookmarks(self) -> List[Dict[str, Any]]:
        with open(self.bookmarks_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        all_bookmarks = []
        
        # Extract from all bookmark locations
        for root in ['bookmark_bar', 'other', 'synced']:
            if root in data['roots']:
                self.extract_bookmarks_from_folder(data['roots'][root], all_bookmarks)
            
        return all_bookmarks
    
    def fetch_content(self, url: str, timeout: int = 10) -> Optional[str]:
        """Fetch the actual content of a webpage"""
        try:
            headers = {
                'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
            }
            response = requests.get(url, headers=headers, timeout=timeout)
            response.raise_for_status()
            
            # Simple content extraction
            soup = BeautifulSoup(response.text, 'html.parser')
            
            # Remove script and style elements
            for script in soup(["script", "style"]):
                script.decompose()
            
            # Get text
            text = soup.get_text()
            
            # Clean up whitespace
            lines = (line.strip() for line in text.splitlines())
            chunks = (phrase.strip() for line in lines for phrase in line.split("  "))
            text = ' '.join(chunk for chunk in chunks if chunk)
            
            return text[:10000]  # Limit content length
            
        except Exception as e:
            print(f"Error fetching {url}: {e}")
            return None
    
    def generate_queries_for_bookmark(self, bookmark: Dict[str, Any], max_queries: int = 5) -> List[str]:
        """Generate search queries for a bookmark"""
        
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
Vary the queries: some short (2-3 words), some longer (5-7 words), some questions, some keyword-based.

Title: {title}
URL: {url}
Content (first 2000 chars): {content[:2000]}

Generate exactly {num_queries} queries:"""

        try:
            response = self.ollama_client.generate(
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
            
            # Clean up queries (remove numbering and filter out instructional text)
            cleaned_queries = []
            for q in queries:
                q = q.strip()
                # Skip empty lines
                if not q:
                    continue
                    
                # Skip instructional/introductory text
                if any(skip_phrase in q.lower() for skip_phrase in [
                    'here are', 'based on', 'search queries', 'different queries',
                    'webpage content', 'varying in', 'specificity:', 'generated queries',
                    'following are', 'below are'
                ]):
                    continue
                    
                # Remove numbering patterns
                q = q.lstrip('0123456789.-) ')
                
                # Skip if it's just punctuation or very short after cleaning
                if len(q) < 3:
                    continue
                    
                cleaned_queries.append(q)
                
                # Stop when we have enough queries
                if len(cleaned_queries) >= num_queries:
                    break
            
            return cleaned_queries[:num_queries]
            
        except Exception as e:
            print(f"Error generating queries for bookmark {bookmark.get('id')}: {e}")
            raise Exception(f"Query generation failed: {e}") from e
    
    def load_existing_data(self, samples_path: str, queries_path: str) -> tuple:
        """Load existing samples and queries if they exist"""
        existing_samples = {}
        existing_queries = {}
        
        # Load existing samples
        samples_file = Path(samples_path)
        if samples_file.exists():
            try:
                with open(samples_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                    samples_list = data.get('samples', data) if isinstance(data, dict) else data
                    for sample in samples_list:
                        sample_id = sample.get('id', sample.get('guid', str(hash(sample['url']))))
                        existing_samples[sample_id] = sample
                print(f"Loaded {len(existing_samples)} existing samples")
            except Exception as e:
                print(f"Could not load existing samples: {e}")
        
        # Load existing queries
        queries_file = Path(queries_path)
        if queries_file.exists():
            try:
                with open(queries_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                    existing_queries = data.get('queries', {})
                print(f"Loaded queries for {len(existing_queries)} bookmarks")
            except Exception as e:
                print(f"Could not load existing queries: {e}")
        
        return existing_samples, existing_queries
    
    def save_data_incremental(self, samples: Dict[str, Any], queries: Dict[str, List[str]], 
                             samples_path: str, queries_path: str):
        """Save samples and queries incrementally"""
        
        # Save samples
        samples_file = Path(samples_path)
        samples_file.parent.mkdir(parents=True, exist_ok=True)
        
        samples_data = {
            'timestamp': datetime.now().isoformat(),
            'count': len(samples),
            'samples': list(samples.values())
        }
        
        # Windows-compatible file saving
        try:
            temp_samples = samples_file.with_suffix('.tmp')
            with open(temp_samples, 'w', encoding='utf-8') as f:
                json.dump(samples_data, f, indent=2, ensure_ascii=False)
            
            # On Windows, remove target file first if it exists
            if samples_file.exists():
                samples_file.unlink()
            temp_samples.rename(samples_file)
        except Exception as e:
            # Fallback to direct write if atomic operation fails
            print(f"Warning: atomic write failed ({e}), writing directly")
            with open(samples_file, 'w', encoding='utf-8') as f:
                json.dump(samples_data, f, indent=2, ensure_ascii=False)
        
        # Save queries
        queries_file = Path(queries_path)
        queries_file.parent.mkdir(parents=True, exist_ok=True)
        
        queries_data = {
            'model': self.model,
            'total_bookmarks': len(queries),
            'total_queries': sum(len(q) for q in queries.values()),
            'queries': queries
        }
        
        # Windows-compatible file saving for queries
        try:
            temp_queries = queries_file.with_suffix('.tmp')
            with open(temp_queries, 'w', encoding='utf-8') as f:
                json.dump(queries_data, f, indent=2, ensure_ascii=False)
            
            # On Windows, remove target file first if it exists
            if queries_file.exists():
                queries_file.unlink()
            temp_queries.rename(queries_file)
        except Exception as e:
            # Fallback to direct write if atomic operation fails
            print(f"Warning: atomic write failed ({e}), writing directly")
            with open(queries_file, 'w', encoding='utf-8') as f:
                json.dump(queries_data, f, indent=2, ensure_ascii=False)
    
    def sample_and_generate(self, sample_size: int = 200, 
                           samples_path: str = "data/sampled_bookmarks.json",
                           queries_path: str = "data/generated_queries.json") -> tuple:
        """Sample bookmarks and generate queries incrementally"""
        
        # Always load existing data
        existing_samples, existing_queries = self.load_existing_data(samples_path, queries_path)
        
        samples = existing_samples.copy()
        queries = existing_queries.copy()
        
        # Check if we already have enough samples
        if len(samples) >= sample_size:
            print(f"Already have {len(samples)} samples (target: {sample_size})")
            return list(samples.values()), queries
        
        # Need more samples
        needed = sample_size - len(samples)
        print(f"Have {len(samples)} samples, need {needed} more to reach {sample_size}")
        
        # Get all bookmarks
        all_bookmarks = self.get_all_bookmarks()
        print(f"Found {len(all_bookmarks)} total bookmarks")
        
        # Filter out already processed bookmarks
        unprocessed_bookmarks = []
        for bookmark in all_bookmarks:
            bookmark_id = bookmark.get('id', bookmark.get('guid', str(hash(bookmark['url']))))
            if bookmark_id not in samples:
                unprocessed_bookmarks.append(bookmark)
        
        if not unprocessed_bookmarks:
            print(f"No more unprocessed bookmarks available. Have {len(samples)} samples total.")
            return list(samples.values()), queries
        
        # Randomly sample from unprocessed bookmarks (sample extra in case some fail)
        to_process = random.sample(unprocessed_bookmarks, min(needed * 2, len(unprocessed_bookmarks)))
        
        print(f"Processing up to {len(to_process)} bookmarks to add {needed} more samples...")
        
        # Process bookmarks one by one
        added = 0
        with tqdm(total=needed, desc="Adding samples") as pbar:
            for bookmark in to_process:
                if added >= needed:
                    break
                
                bookmark_id = bookmark.get('id', bookmark.get('guid', str(hash(bookmark['url']))))
                
                # Fetch content
                content = self.fetch_content(bookmark['url'])
                if not content or len(content) < 200:
                    continue
                
                # Add content to bookmark
                bookmark['content'] = content
                samples[bookmark_id] = bookmark
                
                # Generate queries immediately - stop on error
                try:
                    title = bookmark.get('name', '') or bookmark.get('url', bookmark_id)
                    print(f"Generating queries for: {title}")
                    bookmark_queries = self.generate_queries_for_bookmark(bookmark)
                    if bookmark_queries:
                        queries[bookmark_id] = bookmark_queries
                    else:
                        print(f"Warning: No queries generated for: {title}")
                except Exception as e:
                    print(f"\nStopping due to query generation error: {e}")
                    print(f"Successfully processed {added} bookmarks before error.")
                    print(f"Current total: {len(samples)} samples with {sum(len(q) for q in queries.values())} queries")
                    return list(samples.values()), queries
                
                # Save incrementally
                self.save_data_incremental(samples, queries, samples_path, queries_path)
                
                added += 1
                pbar.update(1)
                pbar.set_postfix({
                    'total': len(samples),
                    'new': added,
                    'queries': sum(len(q) for q in queries.values())
                })
        
        print(f"\nCompleted: {len(samples)} samples total ({added} new)")
        print(f"Total queries: {sum(len(q) for q in queries.values())}")
        
        return list(samples.values()), queries