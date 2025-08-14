from query_generator import QueryGenerator

# Test with actual bookmark data
generator = QueryGenerator()

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