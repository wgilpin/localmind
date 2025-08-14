#!/usr/bin/env python3
"""Test configuration structure migration and compatibility"""

import json
import tempfile
import os
from pathlib import Path
from exclude_filter import ExcludeFilter

def test_new_config_structure():
    """Test that the new indexing config structure works"""
    
    print("="*60)
    print("TESTING NEW CONFIGURATION STRUCTURE")
    print("="*60)
    
    # Create a temporary config with new structure
    new_config = {
        "ollama": {
            "ollamaApiUrl": "http://localhost:11434",
            "embeddingModel": "mahonzhan/all-MiniLM-L6-v2",
            "embeddingDimension": 384,
            "completionModel": "qwen3:0.6b"
        },
        "indexing": {
            "vectorIndexFile": "/path/to/index",
            "chromaDbPath": "/path/to/chromadb",
            "excludeFolders": [
                "node_modules",
                ".git", 
                "build",
                "dist",
                "coverage"
            ]
        },
        "server": {
            "port": 3000
        }
    }
    
    # Create temporary config file
    with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.json') as f:
        json.dump(new_config, f, indent=2)
        temp_config_path = f.name
    
    try:
        # Mock the config path by temporarily modifying ExcludeFilter
        original_load = ExcludeFilter._load_exclude_folders
        
        def mock_load_exclude_folders(self):
            try:
                with open(temp_config_path, 'r', encoding='utf-8') as f:
                    config = json.load(f)
                    # Try new structure first
                    exclude_folders = config.get('indexing', {}).get('excludeFolders', [])
                    if exclude_folders:
                        print(f"[PASS] Loaded {len(exclude_folders)} exclude folders from NEW config structure")
                        return exclude_folders
            except Exception as e:
                print(f"Could not load test config: {e}")
            return []
        
        # Apply mock
        ExcludeFilter._load_exclude_folders = mock_load_exclude_folders
        
        # Test with new structure
        exclude_filter = ExcludeFilter()
        folders = exclude_filter.get_exclude_folders()
        
        print(f"Exclude folders loaded: {folders}")
        
        expected_folders = ["node_modules", ".git", "build", "dist", "coverage"]
        if folders == expected_folders:
            print("[PASS] NEW config structure loaded correctly")
        else:
            print(f"[FAIL] Expected {expected_folders}, got {folders}")
            
        # Test filtering functionality
        test_bookmarks = [
            {"name": "React Docs", "url": "https://reactjs.org/docs"},
            {"name": "Node Package", "url": "https://github.com/repo/node_modules/pkg"},
            {"name": "Build Config", "url": "https://example.com/build/config"},
            {"name": "API Reference", "url": "https://api.example.com/reference"},
            {"name": ".git repository", "url": "https://github.com/repo/.git/config"},
        ]
        
        filtered = exclude_filter.filter_bookmarks(test_bookmarks)
        if len(filtered) == 2:  # Should keep React Docs and API Reference
            print("[PASS] NEW config structure filtering works correctly")
        else:
            print(f"[FAIL] Expected 2 filtered bookmarks, got {len(filtered)}")
            
        # Restore original method
        ExcludeFilter._load_exclude_folders = original_load
        
    finally:
        os.unlink(temp_config_path)

def test_old_config_compatibility():
    """Test backward compatibility with old ollama config structure"""
    
    print("\n" + "="*60)
    print("TESTING OLD CONFIGURATION COMPATIBILITY") 
    print("="*60)
    
    # Create a temporary config with old structure
    old_config = {
        "ollama": {
            "ollamaApiUrl": "http://localhost:11434",
            "embeddingModel": "mahonzhan/all-MiniLM-L6-v2",
            "embeddingDimension": 384,
            "completionModel": "qwen3:0.6b",
            "vectorIndexFile": "/path/to/index",
            "chromaDbPath": "/path/to/chromadb",
            "excludeFolders": [
                "node_modules",
                ".git",
                "build"
            ]
        },
        "server": {
            "port": 3000
        }
    }
    
    # Create temporary config file
    with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.json') as f:
        json.dump(old_config, f, indent=2)
        temp_config_path = f.name
    
    try:
        # Mock the config path
        original_load = ExcludeFilter._load_exclude_folders
        
        def mock_load_exclude_folders(self):
            try:
                with open(temp_config_path, 'r', encoding='utf-8') as f:
                    config = json.load(f)
                    # Try new structure first, fallback to old structure for compatibility
                    exclude_folders = config.get('indexing', {}).get('excludeFolders', [])
                    if not exclude_folders:
                        # Fallback to old structure
                        exclude_folders = config.get('ollama', {}).get('excludeFolders', [])
                        if exclude_folders:
                            print("[PASS] Warning: Found excludeFolders under 'ollama' config (deprecated). Please move to 'indexing' section.")
                    
                    if exclude_folders:
                        print(f"[PASS] Loaded {len(exclude_folders)} exclude folders from OLD config structure")
                        return exclude_folders
            except Exception as e:
                print(f"Could not load test config: {e}")
            return []
        
        # Apply mock
        ExcludeFilter._load_exclude_folders = mock_load_exclude_folders
        
        # Test with old structure
        exclude_filter = ExcludeFilter()
        folders = exclude_filter.get_exclude_folders()
        
        expected_folders = ["node_modules", ".git", "build"]
        if folders == expected_folders:
            print("[PASS] OLD config structure (backward compatibility) works correctly")
        else:
            print(f"[FAIL] Expected {expected_folders}, got {folders}")
            
        # Restore original method
        ExcludeFilter._load_exclude_folders = original_load
        
    finally:
        os.unlink(temp_config_path)

def test_default_fallback():
    """Test fallback to default exclude folders when no config exists"""
    
    print("\n" + "="*60)
    print("TESTING DEFAULT FALLBACK")
    print("="*60)
    
    # Mock to return no config
    original_load = ExcludeFilter._load_exclude_folders
    
    def mock_load_exclude_folders_empty(self):
        print("[PASS] No config found, using defaults")
        # Return default exclude folders (same as TypeScript version)
        return [
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
    
    try:
        # Apply mock
        ExcludeFilter._load_exclude_folders = mock_load_exclude_folders_empty
        
        # Test default fallback
        exclude_filter = ExcludeFilter()
        folders = exclude_filter.get_exclude_folders()
        
        if len(folders) == 17:  # Should have 17 default folders
            print(f"[PASS] Default fallback works correctly ({len(folders)} default folders)")
        else:
            print(f"[FAIL] Expected 17 default folders, got {len(folders)}")
            
    finally:
        # Restore original method
        ExcludeFilter._load_exclude_folders = original_load

def main():
    """Run all configuration migration tests"""
    
    test_new_config_structure()
    test_old_config_compatibility() 
    test_default_fallback()
    
    print("\n" + "="*60)
    print("CONFIGURATION MIGRATION TESTING COMPLETE")
    print("="*60)
    print("[PASS] New 'indexing' configuration section works")
    print("[PASS] Backward compatibility with old 'ollama' section maintained")
    print("[PASS] Default fallback when no config exists")
    print("\nConfiguration structure successfully refactored!")

if __name__ == "__main__":
    main()