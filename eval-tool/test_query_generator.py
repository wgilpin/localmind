import sys
import json
from query_generator import QueryGenerator

def test_extract_short_query():
    """Test the _extract_short_query method with various inputs"""
    generator = QueryGenerator()
    
    test_cases = [
        # (title, url, expected_contains)
        ("LIGO data analysis tutorials", "https://example.com", ["LIGO"]),
        ("Asparagus fettuccine recipe", "https://cooking.com", ["Asparagus"]),
        ("Django App Engine 1.6.0 Installation Guide", "https://django.com", ["Django", "Engine"]),
        ("UCL portal login", "https://ucl.ac.uk", ["UCL"]),
        ("How to install Python", "https://python.org", ["Python", "install"]),
        ("The complete guide to JavaScript", "https://js.com", ["JavaScript", "complete", "guide"]),
        ("ATX magnetic add-on weights", "https://fitness.com", ["ATX"]),
        ("Zenbivy Light Bed - Ultralight Backpacking", "https://zenbivy.com", ["Zenbivy"]),
        ("", "https://github.com", ["github"]),
        ("", "", ["search"]),
    ]
    
    print("Testing _extract_short_query method:\n")
    print("-" * 60)
    
    for title, url, expected_options in test_cases:
        result = generator._extract_short_query(title, url)
        
        # Check if result contains expected words
        success = any(exp in result for exp in expected_options) if expected_options else True
        status = "PASS" if success else "FAIL"
        
        print(f"{status} Title: '{title[:40]}{'...' if len(title) > 40 else ''}'")
        print(f"  URL: '{url}'")
        print(f"  Result: '{result}'")
        print(f"  Expected one of: {expected_options}")
        print()

def test_full_generation():
    """Test the full query generation with mock bookmarks"""
    generator = QueryGenerator()
    
    test_bookmarks = [
        {
            'id': '466',
            'name': 'LIGO data analysis tutorials',
            'url': 'https://gwosc.org',
            'content': 'Learn gravitational wave data analysis with Python. LIGO collaboration provides tutorials for analyzing GWOSC data.'
        },
        {
            'id': '842', 
            'name': 'Asparagus fettuccine recipe',
            'url': 'https://cooking.com/asparagus-pasta',
            'content': 'Delicious asparagus and brown shrimp pasta recipe. Quick 30 minute meal with fresh ingredients.'
        },
        {
            'id': '1001',
            'name': 'UCL portal login',
            'url': 'https://ucl.ac.uk/portal',
            'content': 'Access the UCL student portal for course information, grades, and university resources.'
        }
    ]
    
    print("\nTesting full query generation:\n")
    print("-" * 60)
    
    for bookmark in test_bookmarks:
        print(f"\nBookmark ID: {bookmark['id']}")
        print(f"Title: {bookmark['name']}")
        print(f"Content length: {len(bookmark['content'])} chars")
        
        queries = generator.generate_queries_for_bookmark(bookmark, max_queries=5)
        
        print(f"Generated {len(queries)} queries:")
        for i, query in enumerate(queries, 1):
            word_count = len(query.split())
            print(f"  {i}. '{query}' ({word_count} words)")
        
        # Check if first query is short (1-2 words)
        if queries:
            first_query_words = len(queries[0].split())
            if first_query_words <= 2:
                print(f"  PASS: First query is short ({first_query_words} words)")
            else:
                print(f"  FAIL: First query is NOT short ({first_query_words} words) - PROBLEM!")
        print("-" * 40)

if __name__ == "__main__":
    test_extract_short_query()
    print("\n" + "=" * 60 + "\n")
    test_full_generation()