import json
import os
from pathlib import Path
from typing import List, Dict, Any
from urllib.parse import urlparse


class ExcludeFilter:
    """Python implementation of the LocalMind exclude filter functionality"""
    
    def __init__(self):
        self.exclude_folders = self._load_exclude_folders()
    
    def _load_exclude_folders(self) -> List[str]:
        """Load exclude folders from LocalMind configuration or use defaults"""
        
        # Try to load from LocalMind config
        try:
            localmind_config_dir = Path.home() / '.localmind'
            config_path = localmind_config_dir / 'config.json'
            
            if config_path.exists():
                with open(config_path, 'r', encoding='utf-8') as f:
                    config = json.load(f)
                    # Try new structure first, fallback to old structure for compatibility
                    exclude_folders = config.get('indexing', {}).get('excludeFolders', [])
                    if not exclude_folders:
                        # Fallback to old structure
                        exclude_folders = config.get('ollama', {}).get('excludeFolders', [])
                        if exclude_folders:
                            print("Warning: Found excludeFolders under 'ollama' config (deprecated). Please move to 'indexing' section.")
                    
                    if exclude_folders:
                        print(f"Loaded {len(exclude_folders)} exclude folders from LocalMind config")
                        return exclude_folders
        except Exception as e:
            print(f"Could not load LocalMind config: {e}")
        
        # Default exclude folders (same as TypeScript version)
        default_exclude_folders = [
            'node_modules',
            '.git',
            '.svn',
            '.hg',
            'target',
            'build',
            'dist',
            '.next',
            '.nuxt',
            'coverage',
            '.nyc_output',
            '.cache',
            'tmp',
            'temp',
            'logs',
            '.DS_Store',
            'Thumbs.db'
        ]
        
        print(f"Using {len(default_exclude_folders)} default exclude folders")
        return default_exclude_folders
    
    def should_exclude_folder(self, folder_name: str) -> bool:
        """Check if a bookmark folder name should be excluded based on the exclude list.
        
        This is the primary exclusion method - excludes entire folders and their contents.
        """
        if not folder_name or not self.exclude_folders:
            return False
        
        lower_folder_name = folder_name.lower()
        return any(exclude_pattern.lower() == lower_folder_name 
                  for exclude_pattern in self.exclude_folders)

    def should_exclude_url(self, url: str) -> bool:
        """Legacy function: Check if a URL should be excluded based on folder patterns.
        
        @deprecated Use folder-based exclusion instead. This is kept for backwards compatibility.
        """
        if not url or not self.exclude_folders:
            return False
        
        try:
            parsed_url = urlparse(url)
            pathname = parsed_url.path.lower()
            
            # Check if any exclude folder pattern matches the URL path
            for folder in self.exclude_folders:
                folder_pattern = folder.lower()
                
                # Check various path patterns
                if (f'/{folder_pattern}/' in pathname or 
                    f'/{folder_pattern}' in pathname or 
                    pathname.endswith(f'/{folder_pattern}') or 
                    f'{folder_pattern}/' in pathname):
                    return True
            
            return False
            
        except Exception as e:
            # If URL parsing fails, don't exclude it
            print(f"Warning: Failed to parse URL for exclusion check: {url} ({e})")
            return False
    
    def should_exclude_bookmark(self, title: str, url: str) -> bool:
        """Legacy function: Check if a bookmark should be excluded based on title or URL.
        
        @deprecated Use folder-based exclusion instead. This is kept for backwards compatibility.
        """
        # First check the URL
        if self.should_exclude_url(url):
            return True
        
        # Then check if the title contains any exclude patterns
        if not title or not self.exclude_folders:
            return False
        
        lower_title = title.lower()
        for folder in self.exclude_folders:
            folder_pattern = folder.lower()
            if folder_pattern in lower_title:
                return True
        
        return False
    
    def filter_bookmarks(self, bookmarks: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Filter a list of bookmarks, removing those that should be excluded"""
        
        if not bookmarks:
            return bookmarks
        
        original_count = len(bookmarks)
        filtered_bookmarks = []
        excluded_count = 0
        
        for bookmark in bookmarks:
            title = bookmark.get('name', '')
            url = bookmark.get('url', '')
            
            if self.should_exclude_bookmark(title, url):
                excluded_count += 1
                print(f"SKIP: Excluding bookmark: {title} ({url})")
            else:
                filtered_bookmarks.append(bookmark)
        
        if excluded_count > 0:
            print(f"Filtered out {excluded_count} bookmarks from {original_count} total")
            print(f"Remaining: {len(filtered_bookmarks)} bookmarks")
        
        return filtered_bookmarks
    
    def get_exclude_folders(self) -> List[str]:
        """Get the current list of exclude folders"""
        return self.exclude_folders.copy()
    
    def add_exclude_folder(self, folder: str):
        """Add a folder to the exclude list"""
        if folder not in self.exclude_folders:
            self.exclude_folders.append(folder)
    
    def remove_exclude_folder(self, folder: str):
        """Remove a folder from the exclude list"""
        if folder in self.exclude_folders:
            self.exclude_folders.remove(folder)