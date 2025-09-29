#!/usr/bin/env python3
"""Debug the raw LLM response"""

from chunk_query_generator import ChunkQueryGenerator
from chunk_quality_filter import ChunkQualityFilter

# Load test chunk
filter = ChunkQualityFilter()
quality_chunks = filter.load_filtered_chunks('data/quality_chunks.json')
chunk_6509 = next((c for c in quality_chunks if c.chunk_id == 6509), None)

if not chunk_6509:
    print("Chunk 6509 not found")
    exit(1)

# Initialize generator
generator = ChunkQueryGenerator(model="qwen/qwen3-4b")

# Get raw response
context_info = f"Document: {chunk_6509.document_title}" if chunk_6509.document_title else ""

prompt = f"""You need to generate exactly 3 search terms that someone would use to find this information.

{context_info}

Chunk text:
"{chunk_6509.chunk_text}"

You can think through this, but end your response with exactly 3 search terms, one per line.

Search terms must be:
- 1-4 words each
- Natural queries a user would type
- Semantically relevant to the chunk content
- Different from each other
- NOT direct quotes from the text
- NOT generic terms like "information" or "content"
- NOT the document title verbatim

Good examples: "llm evaluation tools", "python frameworks", "anthropic inspect"

Think about what someone would search for to find this specific information, then provide the 3 search terms at the end."""

try:
    response = generator.lmstudio_client.chat(
        model="qwen/qwen3-4b",
        messages=[{'role': 'user', 'content': prompt}],
        temperature=0.7,
        top_p=0.9,
        max_tokens=2000
    )

    print("=== RAW RESPONSE ===")
    print(response['message']['content'])
    print("\n=== RESPONSE LENGTH ===")
    print(f"Characters: {len(response['message']['content'])}")
    print(f"Tokens (approx): {len(response['message']['content'].split())}")
    print(f"Finish reason: {response.get('finish_reason', 'unknown')}")

    # Try parsing
    response_text = response['message']['content'].strip()
    lines = response_text.split('\n')

    print("\n=== PARSED LINES ===")
    for i, line in enumerate(lines):
        print(f"{i}: '{line.strip()}'")

except Exception as e:
    print(f"Error: {e}")