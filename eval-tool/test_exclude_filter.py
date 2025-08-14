#!/usr/bin/env python3
"""Test script to verify exclude filter functionality in eval-tool"""

import json
from exclude_filter import ExcludeFilter

def test_exclude_filter():
    """Test the exclude filter functionality"""
    
    print("="*60)
    print("TESTING EXCLUDE FILTER")
    print("="*60)
    
    # Initialize filter
    exclude_filter = ExcludeFilter()
    
    print(f"Loaded exclude folders: {exclude_filter.get_exclude_folders()}")
    print()
    
    # Test URLs that should be excluded
    test_urls_excluded = [
        "https://github.com/user/repo/tree/main/node_modules/package",
        "https://example.com/project/.git/config", 
        "https://site.com/app/build/index.html",
        "https://cdn.com/dist/bundle.js",
        "https://docs.example.com/coverage/report.html"
    ]
    
    # Test URLs that should NOT be excluded
    test_urls_included = [
        "https://github.com/user/repo/src/index.js",
        "https://docs.example.com/api/reference",
        "https://reactjs.org/docs/getting-started",
        "https://blog.site.com/article",
        "https://stackoverflow.com/questions/12345"
    ]
    
    # Test bookmarks that should be excluded
    test_bookmarks_excluded = [
        {"name": "Package Documentation", "url": "https://github.com/user/repo/node_modules/pkg"},
        {"name": "node_modules info", "url": "https://example.com/page"},
        {"name": "Build Configuration", "url": "https://example.com/page"},
        {"name": "Git Repository", "url": "https://example.com/project/.git/hooks"}
    ]
    
    # Test bookmarks that should NOT be excluded
    test_bookmarks_included = [
        {"name": "React Documentation", "url": "https://reactjs.org/docs"},
        {"name": "API Reference", "url": "https://api.example.com/docs"},
        {"name": "Python Tutorial", "url": "https://python.org/tutorial"},
        {"name": "Stack Overflow Question", "url": "https://stackoverflow.com/questions/123"}
    ]
    
    print("Testing URL exclusion...")
    print("-" * 30)
    
    for url in test_urls_excluded:
        result = exclude_filter.should_exclude_url(url)
        status = "[PASS] EXCLUDED" if result else "[FAIL] NOT EXCLUDED"
        print(f"{status}: {url}")
    
    print()
    for url in test_urls_included:
        result = exclude_filter.should_exclude_url(url)
        status = "[FAIL] EXCLUDED" if result else "[PASS] INCLUDED"
        print(f"{status}: {url}")
    
    print()
    print("Testing bookmark exclusion...")
    print("-" * 30)
    
    for bookmark in test_bookmarks_excluded:
        result = exclude_filter.should_exclude_bookmark(bookmark['name'], bookmark['url'])
        status = "[PASS] EXCLUDED" if result else "[FAIL] NOT EXCLUDED"
        print(f"{status}: {bookmark['name']} ({bookmark['url']})")
    
    print()
    for bookmark in test_bookmarks_included:
        result = exclude_filter.should_exclude_bookmark(bookmark['name'], bookmark['url'])
        status = "[FAIL] EXCLUDED" if result else "[PASS] INCLUDED"
        print(f"{status}: {bookmark['name']} ({bookmark['url']})")
    
    print()
    print("Testing bookmark list filtering...")
    print("-" * 30)
    
    all_test_bookmarks = test_bookmarks_excluded + test_bookmarks_included
    print(f"Original bookmarks: {len(all_test_bookmarks)}")
    
    filtered_bookmarks = exclude_filter.filter_bookmarks(all_test_bookmarks)
    print(f"After filtering: {len(filtered_bookmarks)}")
    
    print()
    print("Remaining bookmarks:")
    for bookmark in filtered_bookmarks:
        print(f"  - {bookmark['name']} ({bookmark['url']})")

def test_bookmark_sampler():
    """Test the bookmark sampler with exclude filter"""
    
    print("\n" + "="*60)
    print("TESTING BOOKMARK SAMPLER INTEGRATION")
    print("="*60)
    
    try:
        from bookmark_sampler import BookmarkSampler
        
        sampler = BookmarkSampler()
        print(f"BookmarkSampler initialized with exclude filter")
        print(f"Exclude folders: {sampler.exclude_filter.get_exclude_folders()}")
        
        # Try to get all bookmarks (will filter automatically)
        try:
            bookmarks = sampler.get_all_bookmarks()
            print(f"Found {len(bookmarks)} bookmarks after filtering")
        except FileNotFoundError:
            print("Chrome bookmarks file not found (this is expected if Chrome isn't installed)")
        except Exception as e:
            print(f"Error reading bookmarks: {e}")
            
    except ImportError as e:
        print(f"Could not import BookmarkSampler: {e}")

def test_integrated_sampler():
    """Test the integrated sampler with exclude filter"""
    
    print("\n" + "="*60)
    print("TESTING INTEGRATED SAMPLER INTEGRATION")
    print("="*60)
    
    try:
        from integrated_sampler import IntegratedSampler
        
        sampler = IntegratedSampler()
        print(f"IntegratedSampler initialized with exclude filter")
        print(f"Exclude folders: {sampler.exclude_filter.get_exclude_folders()}")
        
        # Try to get all bookmarks (will filter automatically)  
        try:
            bookmarks = sampler.get_all_bookmarks()
            print(f"Found {len(bookmarks)} bookmarks after filtering")
        except FileNotFoundError:
            print("Chrome bookmarks file not found (this is expected if Chrome isn't installed)")
        except Exception as e:
            print(f"Error reading bookmarks: {e}")
            
    except ImportError as e:
        print(f"Could not import IntegratedSampler: {e}")

if __name__ == "__main__":
    test_exclude_filter()
    test_bookmark_sampler()
    test_integrated_sampler()
    
    print("\n" + "="*60)
    print("TESTING COMPLETE")
    print("="*60)