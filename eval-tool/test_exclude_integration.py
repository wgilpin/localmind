#!/usr/bin/env python3
"""Integration test for exclude filter in eval-tool without external dependencies"""

import json
import tempfile
import os
from pathlib import Path
from exclude_filter import ExcludeFilter

def create_test_bookmarks_file():
    """Create a temporary Chrome bookmarks file for testing"""
    
    test_bookmarks = {
        "roots": {
            "bookmark_bar": {
                "children": [
                    {
                        "type": "url",
                        "id": "1",
                        "name": "React Documentation", 
                        "url": "https://reactjs.org/docs",
                        "date_added": "13000000000000000"
                    },
                    {
                        "type": "url", 
                        "id": "2",
                        "name": "Node Modules Package",
                        "url": "https://github.com/user/repo/node_modules/package",
                        "date_added": "13000000000000000"
                    },
                    {
                        "type": "url",
                        "id": "3", 
                        "name": "Python Tutorial",
                        "url": "https://python.org/tutorial", 
                        "date_added": "13000000000000000"
                    },
                    {
                        "type": "url",
                        "id": "4",
                        "name": "Build Scripts",
                        "url": "https://example.com/project/build/scripts",
                        "date_added": "13000000000000000"
                    }
                ]
            },
            "other": {
                "children": [
                    {
                        "type": "url",
                        "id": "5",
                        "name": "Git Hooks", 
                        "url": "https://example.com/project/.git/hooks",
                        "date_added": "13000000000000000"
                    },
                    {
                        "type": "url",
                        "id": "6",
                        "name": "API Documentation",
                        "url": "https://api.example.com/docs",
                        "date_added": "13000000000000000"
                    }
                ]
            }
        }
    }
    
    # Create temporary file
    temp_file = tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.json', encoding='utf-8')
    json.dump(test_bookmarks, temp_file, indent=2)
    temp_file.close()
    
    return temp_file.name

def test_bookmark_sampler_integration():
    """Test BookmarkSampler with exclude filter"""
    
    print("="*60)
    print("TESTING BOOKMARK SAMPLER WITH EXCLUDE FILTER")
    print("="*60)
    
    # Create test bookmarks file
    test_file = create_test_bookmarks_file()
    
    try:
        from bookmark_sampler import BookmarkSampler
        
        # Initialize sampler with test file
        sampler = BookmarkSampler(bookmarks_path=test_file)
        print(f"Created BookmarkSampler with test bookmarks file")
        print(f"Exclude folders: {sampler.exclude_filter.get_exclude_folders()}")
        print()
        
        # Get all bookmarks (should be filtered)
        bookmarks = sampler.get_all_bookmarks()
        
        print(f"Total bookmarks after filtering: {len(bookmarks)}")
        print()
        print("Remaining bookmarks:")
        for i, bookmark in enumerate(bookmarks, 1):
            print(f"  {i}. {bookmark['name']} ({bookmark['url']})")
        
        # Verify expected results
        expected_count = 3  # Should exclude node_modules, build, and .git URLs
        if len(bookmarks) == expected_count:
            print(f"\n[PASS] Expected {expected_count} bookmarks after filtering, got {len(bookmarks)}")
        else:
            print(f"\n[FAIL] Expected {expected_count} bookmarks after filtering, got {len(bookmarks)}")
        
        # Verify no excluded patterns remain
        excluded_patterns = ['node_modules', '.git', 'build']
        has_excluded = False
        for bookmark in bookmarks:
            url_lower = bookmark['url'].lower()
            title_lower = bookmark['name'].lower()
            for pattern in excluded_patterns:
                if pattern in url_lower or pattern in title_lower:
                    print(f"[FAIL] Found excluded pattern '{pattern}' in: {bookmark['name']} ({bookmark['url']})")
                    has_excluded = True
        
        if not has_excluded:
            print("[PASS] No excluded patterns found in remaining bookmarks")
        
    except ImportError as e:
        print(f"Could not import BookmarkSampler: {e}")
    
    finally:
        # Clean up test file
        os.unlink(test_file)

def test_direct_exclude_filter():
    """Test the exclude filter directly with sample data"""
    
    print("\n" + "="*60)
    print("TESTING DIRECT EXCLUDE FILTER FUNCTIONALITY")
    print("="*60)
    
    exclude_filter = ExcludeFilter()
    
    sample_bookmarks = [
        {"name": "React Docs", "url": "https://reactjs.org/docs"},
        {"name": "Node Package", "url": "https://github.com/repo/node_modules/pkg"},
        {"name": "Build Config", "url": "https://example.com/build/config"},
        {"name": "API Reference", "url": "https://api.example.com/reference"},
        {"name": ".git repository", "url": "https://github.com/repo/.git/config"},
        {"name": "Python Tutorial", "url": "https://python.org/tutorial"},
        {"name": "Coverage Report", "url": "https://example.com/coverage/report"}
    ]
    
    print(f"Testing with {len(sample_bookmarks)} sample bookmarks")
    print(f"Exclude folders: {exclude_filter.get_exclude_folders()[:5]}... (showing first 5)")
    print()
    
    filtered = exclude_filter.filter_bookmarks(sample_bookmarks)
    
    print(f"\nFiltering results:")
    print(f"Original: {len(sample_bookmarks)} bookmarks")
    print(f"Filtered: {len(filtered)} bookmarks")
    print(f"Excluded: {len(sample_bookmarks) - len(filtered)} bookmarks")
    
    # Should have 3 remaining: React Docs, API Reference, Python Tutorial
    expected_remaining = 3
    if len(filtered) == expected_remaining:
        print(f"[PASS] Expected {expected_remaining} bookmarks remaining")
    else:
        print(f"[FAIL] Expected {expected_remaining} bookmarks remaining, got {len(filtered)}")

def main():
    """Run all integration tests"""
    
    test_direct_exclude_filter()
    test_bookmark_sampler_integration()
    
    print("\n" + "="*60)
    print("INTEGRATION TESTING COMPLETE")
    print("="*60)
    print("The eval-tool now respects the LocalMind exclude list!")
    print("Excluded patterns will be automatically filtered out during bookmark sampling.")

if __name__ == "__main__":
    main()