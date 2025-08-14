import json
import random
from pathlib import Path
from typing import List, Dict, Any
import os
import requests
from datetime import datetime
from tqdm import tqdm
from exclude_filter import ExcludeFilter

class BookmarkSampler:
    def __init__(self, bookmarks_path: str = None):
        if bookmarks_path is None:
            # Default Chrome bookmarks location on Windows
            local_app_data = os.getenv('LOCALAPPDATA')
            bookmarks_path = Path(local_app_data) / 'Google' / 'Chrome' / 'User Data' / 'Default' / 'Bookmarks'
        self.bookmarks_path = Path(bookmarks_path)
        self.exclude_filter = ExcludeFilter()
        
    def should_exclude_folder(self, folder_name: str) -> bool:
        """Check if a folder name should be excluded based on the exclude list."""
        if not folder_name or not self.exclude_filter.get_exclude_folders():
            return False
        
        lower_folder_name = folder_name.lower()
        return any(exclude_pattern.lower() == lower_folder_name 
                  for exclude_pattern in self.exclude_filter.get_exclude_folders())

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
                    # Check if this folder should be excluded
                    if self.should_exclude_folder(item['name']):
                        continue  # Skip this entire folder and all its children
                    
                    # Process children if folder is not excluded
                    self.extract_bookmarks_from_folder(item, bookmarks)
        
        return bookmarks
    
    def get_all_bookmarks(self) -> List[Dict[str, Any]]:
        with open(self.bookmarks_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        all_bookmarks = []
        
        # Extract from bookmark bar
        if 'bookmark_bar' in data['roots']:
            self.extract_bookmarks_from_folder(data['roots']['bookmark_bar'], all_bookmarks)
        
        # Extract from other bookmarks
        if 'other' in data['roots']:
            self.extract_bookmarks_from_folder(data['roots']['other'], all_bookmarks)
        
        # Extract from synced bookmarks
        if 'synced' in data['roots']:
            self.extract_bookmarks_from_folder(data['roots']['synced'], all_bookmarks)
        
        return all_bookmarks
    
    def fetch_content(self, url: str, timeout: int = 10) -> str:
        """Fetch the actual content of a webpage"""
        try:
            headers = {
                'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
            }
            response = requests.get(url, headers=headers, timeout=timeout)
            response.raise_for_status()
            
            # Simple content extraction - just get text from HTML
            from bs4 import BeautifulSoup
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
    
    def has_meaningful_title(self, title: str) -> bool:
        """Check if a bookmark title has more than 3 words"""
        if not title or not title.strip():
            return False
        words = title.strip().split()
        return len(words) > 3
    
    def sample_bookmarks_with_content(self, sample_size: int = 200, fetch_content: bool = True) -> List[Dict[str, Any]]:
        all_bookmarks = self.get_all_bookmarks()
        print(f"Found {len(all_bookmarks)} total bookmarks")
        
        # Sample bookmarks
        if len(all_bookmarks) <= sample_size:
            sampled = all_bookmarks
        else:
            sampled = random.sample(all_bookmarks, min(sample_size * 2, len(all_bookmarks)))  # Sample extra in case some fail
        
        # Fetch content if requested
        bookmarks_with_content = []
        if fetch_content:
            print("Fetching page content for sampled bookmarks...")
            for bookmark in tqdm(sampled):
                if len(bookmarks_with_content) >= sample_size:
                    break
                    
                content = self.fetch_content(bookmark['url'])
                if content and len(content) > 200:  # Keep if we got meaningful content
                    bookmark['content'] = content
                    bookmarks_with_content.append(bookmark)
                elif self.has_meaningful_title(bookmark.get('name', '')):  # Fallback: keep if title has 3+ words
                    bookmark['content'] = None  # Mark that content couldn't be retrieved
                    bookmarks_with_content.append(bookmark)
        else:
            bookmarks_with_content = sampled[:sample_size]
        
        print(f"Successfully sampled {len(bookmarks_with_content)} bookmarks")
        return bookmarks_with_content
    
    def save_samples(self, samples: List[Dict[str, Any]], output_path: str = "data/sampled_bookmarks.json"):
        output_file = Path(output_path)
        output_file.parent.mkdir(parents=True, exist_ok=True)
        
        # Save with timestamp
        metadata = {
            'timestamp': datetime.now().isoformat(),
            'count': len(samples),
            'samples': samples
        }
        
        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(metadata, f, indent=2, ensure_ascii=False)
        
        print(f"Saved {len(samples)} samples to {output_file}")
        return output_file
    
    def load_samples(self, input_path: str = "data/sampled_bookmarks.json") -> List[Dict[str, Any]]:
        with open(input_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
            return data['samples'] if 'samples' in data else data