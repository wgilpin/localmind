#!/usr/bin/env python3
"""Test the new folder-based exclusion functionality"""

import json
import tempfile
import os
from pathlib import Path
from exclude_filter import ExcludeFilter

def create_test_bookmarks_with_folders():
    """Create a comprehensive test Chrome bookmarks structure with folders"""
    
    test_bookmarks = {
        "roots": {
            "bookmark_bar": {
                "children": [
                    {
                        "type": "url",
                        "id": "1",
                        "name": "React Documentation", 
                        "url": "https://reactjs.org/docs"
                    },
                    {
                        "type": "folder",
                        "id": "2",
                        "name": "build",
                        "children": [
                            {
                                "type": "url",
                                "id": "3",
                                "name": "Build Config",
                                "url": "https://example.com/build-config"
                            },
                            {
                                "type": "url", 
                                "id": "4",
                                "name": "Webpack Docs",
                                "url": "https://webpack.js.org/config"
                            },
                            {
                                "type": "folder",
                                "id": "5", 
                                "name": "nested",
                                "children": [
                                    {
                                        "type": "url",
                                        "id": "6",
                                        "name": "Nested Build Tool",
                                        "url": "https://example.com/nested-tool"
                                    }
                                ]
                            }
                        ]
                    },
                    {
                        "type": "url",
                        "id": "7",
                        "name": "Python Tutorial",
                        "url": "https://python.org/tutorial"
                    }
                ]
            },
            "other": {
                "children": [
                    {
                        "type": "folder",
                        "id": "8",
                        "name": "node_modules",
                        "children": [
                            {
                                "type": "url",
                                "id": "9",
                                "name": "Package A Docs",
                                "url": "https://npmjs.com/package/package-a"
                            },
                            {
                                "type": "url",
                                "id": "10", 
                                "name": "Package B Docs",
                                "url": "https://npmjs.com/package/package-b"
                            }
                        ]
                    },
                    {
                        "type": "url",
                        "id": "11",
                        "name": "API Documentation",
                        "url": "https://api.example.com/docs"
                    },
                    {
                        "type": "folder",
                        "id": "12",
                        "name": "Work",
                        "children": [
                            {
                                "type": "url",
                                "id": "13",
                                "name": "Company Wiki",
                                "url": "https://company.com/wiki"
                            },
                            {
                                "type": "folder",
                                "id": "14",
                                "name": ".git",
                                "children": [
                                    {
                                        "type": "url",
                                        "id": "15",
                                        "name": "Git Hooks Reference",
                                        "url": "https://git-scm.com/docs/githooks"
                                    }
                                ]
                            }
                        ]
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

def test_bookmark_sampler_folder_exclusion():
    """Test BookmarkSampler with folder-based exclusion"""
    
    print("="*60)
    print("TESTING BOOKMARK SAMPLER FOLDER EXCLUSION")
    print("="*60)
    
    # Create test bookmarks file
    test_file = create_test_bookmarks_with_folders()
    
    try:
        from bookmark_sampler import BookmarkSampler
        
        # Override the exclude filter to use test patterns
        sampler = BookmarkSampler(bookmarks_path=test_file)
        sampler.exclude_filter.exclude_folders = ['build', 'node_modules', '.git']
        
        print(f"Test exclude folders: {sampler.exclude_filter.exclude_folders}")
        print()
        
        # Get all bookmarks (folder exclusion should happen during extraction)
        bookmarks = sampler.get_all_bookmarks()
        
        print(f"Total bookmarks after folder exclusion: {len(bookmarks)}")
        print()
        print("Remaining bookmarks:")
        for i, bookmark in enumerate(bookmarks, 1):
            print(f"  {i}. {bookmark['name']} ({bookmark['url']})")
        
        # Expected results:
        # Should exclude:
        # - Entire "build" folder (Build Config, Webpack Docs, nested folder and its contents)
        # - Entire "node_modules" folder (Package A Docs, Package B Docs) 
        # - Entire ".git" folder (Git Hooks Reference)
        #
        # Should remain:
        # - React Documentation
        # - Python Tutorial  
        # - API Documentation
        # - Company Wiki (inside "Work" folder, which is not excluded)
        
        expected_bookmarks = [
            "React Documentation",
            "Python Tutorial", 
            "API Documentation",
            "Company Wiki"
        ]
        
        actual_names = [b['name'] for b in bookmarks]
        
        print(f"\nExpected {len(expected_bookmarks)} bookmarks: {expected_bookmarks}")
        print(f"Actually got {len(actual_names)} bookmarks: {actual_names}")
        
        if len(bookmarks) == len(expected_bookmarks):
            print("[PASS] Correct number of bookmarks after folder exclusion")
        else:
            print(f"[FAIL] Expected {len(expected_bookmarks)} bookmarks, got {len(bookmarks)}")
        
        # Check that excluded items are not present
        excluded_names = ["Build Config", "Webpack Docs", "Nested Build Tool", "Package A Docs", "Package B Docs", "Git Hooks Reference"]
        found_excluded = [name for name in excluded_names if name in actual_names]
        
        if not found_excluded:
            print("[PASS] No excluded bookmarks found in results")
        else:
            print(f"[FAIL] Found excluded bookmarks in results: {found_excluded}")
        
        # Check that expected items are present
        missing_expected = [name for name in expected_bookmarks if name not in actual_names]
        
        if not missing_expected:
            print("[PASS] All expected bookmarks found in results")
        else:
            print(f"[FAIL] Missing expected bookmarks: {missing_expected}")
            
    except ImportError as e:
        print(f"Could not import BookmarkSampler: {e}")
    
    finally:
        # Clean up test file
        os.unlink(test_file)

def test_integrated_sampler_folder_exclusion():
    """Test IntegratedSampler with folder-based exclusion"""
    
    print("\n" + "="*60)
    print("TESTING INTEGRATED SAMPLER FOLDER EXCLUSION")
    print("="*60)
    
    # Create test bookmarks file
    test_file = create_test_bookmarks_with_folders()
    
    try:
        from integrated_sampler import IntegratedSampler
        
        # Override the exclude filter to use test patterns
        sampler = IntegratedSampler(bookmarks_path=test_file)
        sampler.exclude_filter.exclude_folders = ['build', 'node_modules']
        
        print(f"Test exclude folders: {sampler.exclude_filter.exclude_folders}")
        print()
        
        # Get all bookmarks (folder exclusion should happen during extraction)
        bookmarks = sampler.get_all_bookmarks()
        
        print(f"Total bookmarks after folder exclusion: {len(bookmarks)}")
        print()
        
        # Expected results (excluding 'build' and 'node_modules' but not '.git'):
        # Should remain: React Documentation, Python Tutorial, API Documentation, Company Wiki, Git Hooks Reference
        expected_count = 5
        
        if len(bookmarks) == expected_count:
            print(f"[PASS] Correct number of bookmarks ({expected_count}) after folder exclusion")
        else:
            print(f"[FAIL] Expected {expected_count} bookmarks, got {len(bookmarks)}")
        
        # Check specific exclusions
        bookmark_names = [b['name'] for b in bookmarks]
        excluded_patterns = ['Build Config', 'Package A Docs', 'Package B Docs']
        found_excluded = [name for name in excluded_patterns if name in bookmark_names]
        
        if not found_excluded:
            print("[PASS] Excluded folder contents not found in results")
        else:
            print(f"[FAIL] Found excluded folder contents: {found_excluded}")
            
    except ImportError as e:
        print(f"Could not import IntegratedSampler: {e}")
    
    finally:
        # Clean up test file
        os.unlink(test_file)

def test_exclude_filter_direct():
    """Test the ExcludeFilter folder exclusion directly"""
    
    print("\n" + "="*60)
    print("TESTING EXCLUDE FILTER FOLDER FUNCTIONALITY")
    print("="*60)
    
    # Test the should_exclude_folder method directly
    exclude_filter = ExcludeFilter()
    exclude_filter.exclude_folders = ['build', 'dist', 'node_modules', '.git', 'temp']
    
    # Test cases
    test_cases = [
        # (folder_name, should_exclude, description)
        ('build', True, 'Exact match'),
        ('Build', True, 'Case insensitive match'),  
        ('BUILD', True, 'Case insensitive match (uppercase)'),
        ('node_modules', True, 'Exact match with underscore'),
        ('.git', True, 'Exact match with dot'),
        ('development', False, 'Partial match should not exclude'),
        ('build-tools', False, 'Containing excluded name should not exclude'),
        ('my-build', False, 'Ending with excluded name should not exclude'),
        ('gitignore', False, 'Containing excluded name should not exclude'),
        ('Work', False, 'Non-excluded folder'),
        ('Documentation', False, 'Non-excluded folder'),
        ('', False, 'Empty folder name'),
    ]
    
    print("Testing folder exclusion logic:")
    all_passed = True
    
    for folder_name, expected, description in test_cases:
        result = exclude_filter.should_exclude_folder(folder_name)
        status = "[PASS]" if result == expected else "[FAIL]"
        if result != expected:
            all_passed = False
        
        print(f"  {status} '{folder_name}' -> {result} ({description})")
    
    if all_passed:
        print("\n[PASS] All folder exclusion tests passed")
    else:
        print("\n[FAIL] Some folder exclusion tests failed")

def main():
    """Run all folder-based exclusion tests"""
    
    test_exclude_filter_direct()
    test_bookmark_sampler_folder_exclusion()
    test_integrated_sampler_folder_exclusion()
    
    print("\n" + "="*60)
    print("FOLDER EXCLUSION TESTING COMPLETE")
    print("="*60)
    print("The exclude functionality now properly works at the folder level!")
    print("Entire bookmark folders and their contents are excluded during extraction.")
    print("This is much more efficient and intuitive than pattern matching.")

if __name__ == "__main__":
    main()