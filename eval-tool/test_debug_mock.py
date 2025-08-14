from query_generator import QueryGenerator

# Mock the Ollama client to avoid actual LLM calls
class MockOllamaClient:
    def generate(self, model, prompt, options):
        # Return mock longer queries
        return {
            'response': """Birkbeck Psychological Sciences research
Richard Cooper research focus
Birkbeck University of London psychology
Study here Undergraduate research
Financial support Birkbeck students"""
        }

# Test with actual bookmark data
generator = QueryGenerator()
# Replace with mock client
generator.client = MockOllamaClient()

bookmark = {
    'id': '797',
    'name': 'Birkbeck Psychological Sciences research',
    'url': 'https://birkbeck.ac.uk',
    'content': 'Research at Birkbeck University focusing on psychological sciences and cognitive studies.'
}

print("Testing short query extraction:")
print("-" * 50)

# Test the _extract_short_query method directly
title = bookmark['name']
url = bookmark['url']
short_query = generator._extract_short_query(title, url)
print(f"Title: {title}")
print(f"URL: {url}")
print(f"Extracted short query: '{short_query}'")
print(f"Short query length: {len(short_query.split())} words")

print("\n" + "=" * 50 + "\n")
print("Testing full generation:")
print("-" * 50)

# Test full generation
queries = generator.generate_queries_for_bookmark(bookmark, max_queries=5)
print(f"Generated {len(queries)} queries:")
for i, q in enumerate(queries, 1):
    print(f"{i}. '{q}' ({len(q.split())} words)")

print("\nFIRST QUERY ANALYSIS:")
if queries:
    first = queries[0]
    print(f"First query: '{first}'")
    print(f"Word count: {len(first.split())}")
    print(f"Is short (<=2 words)? {len(first.split()) <= 2}")
    
print("\nDEBUG: What's in all_queries before cleaning?")
# Let's trace what's happening inside the method
import sys
from io import StringIO

# Capture prints by patching the method
original_method = generator.generate_queries_for_bookmark

def debug_method(bookmark, max_queries=5):
    content = bookmark.get('content', '')
    title = bookmark.get('name', '')
    url = bookmark.get('url', '')
    
    print(f"DEBUG: title='{title}'")
    print(f"DEBUG: url='{url}'")
    
    short_query = generator._extract_short_query(title, url)
    print(f"DEBUG: short_query='{short_query}'")
    
    # Call original
    return original_method(bookmark, max_queries)

generator.generate_queries_for_bookmark = debug_method

print("\n" + "=" * 50 + "\n")
print("Testing with debug output:")
print("-" * 50)
queries2 = generator.generate_queries_for_bookmark(bookmark, max_queries=5)
print(f"Final result: {queries2}")