#!/usr/bin/env python3
"""
Debug script to test query generation for chunk 6509
"""

from lmstudio_client import LMStudioClient
from chunk_quality_filter import QualityChunkSample

# The problematic chunk
chunk_text = "it's like a very sort of advanced eval platform. It is everything is programmed in Python. It's extremely flexible. It has all kinds of things for doing like fancy scoring and parallel operations and agents. You might see some of this and say why do I like I'm just at first base or second base on evals. I don't need all this fancy stuff. You might at some point. So I would say some of you this might resonate and go wow I could really use a framework like that."

chunk = QualityChunkSample(
    chunk_id=6509,
    document_id=75,
    chunk_index=0,
    chunk_text=chunk_text,
    document_title="Inspect - A LLM Eval Framework Used by Anthropic, DeepMind, Grok and More. - YouTube",
    document_url=None,
    chunk_start=0,
    chunk_end=len(chunk_text),
    parent_content=chunk_text,
    embedding_id=1,
    quality_status="accepted",
    quality_reason="test",
    confidence_score=1.0
)

def test_direct_llm():
    """Test direct LLM call"""
    print("="*60)
    print("TESTING DIRECT LLM CALL")
    print("="*60)

    client = LMStudioClient()

    num_terms = 3
    context_info = f"Document: {chunk.document_title}"

    prompt = f"""Given this text chunk from a larger document, generate exactly {num_terms} search terms that someone would likely use to find this specific information.

{context_info}

Chunk text:
"{chunk.chunk_text}"

Generate search terms that are:
- 1-4 words each
- Natural queries a user would type
- Diverse in specificity (mix of broad and specific)
- Likely to match this chunk's content
- Different from each other (not variations of the same term)
- not direct quotes from the text, but semantically relevant
- not common stop words like "the", "and", "of", "their", etc.

DO NOT include:
- Generic terms like "information" or "content"
- The document title verbatim
- Programming-specific terms unless the chunk is about programming
- Terms longer than 4 words

Examples of good search terms:
- "italian pasta recipes"
- "machine learning basics"
- "python error handling"
- "climate change effects"
- "workout routines"
- "investment strategies"

Return ONLY the search terms, one per line, no numbering or bullets."""

    print("PROMPT:")
    print(prompt)
    print("\n" + "="*60)

    try:
        response = client.chat(
            model="qwen/qwen3-4b",
            messages=[
                {
                    'role': 'user',
                    'content': prompt
                }
            ],
            temperature=0.7,
            top_p=0.9,
            max_tokens=100
        )

        response_text = response['message']['content'].strip()
        print("RAW LLM RESPONSE:")
        print(repr(response_text))
        print("\nFORMATTED RESPONSE:")
        print(response_text)

        # Test the parsing logic
        print("\n" + "="*60)
        print("TESTING PARSING LOGIC")
        print("="*60)

        import re
        terms = []
        for line in response_text.split('\n'):
            print(f"Processing line: {repr(line)}")
            line = line.strip()
            print(f"After strip: {repr(line)}")

            # Remove common prefixes
            line = re.sub(r'^[-•\*\d+\.]\s*', '', line)
            print(f"After prefix removal: {repr(line)}")

            # Remove quotes
            line = line.strip('"\'')
            print(f"After quote removal: {repr(line)}")

            if line and len(line.split()) <= 4:
                terms.append(line.lower())
                print(f"Added term: {repr(line.lower())}")
            else:
                print(f"Rejected (empty or too long): {len(line.split()) if line else 0} words")
            print()

        print(f"FINAL PARSED TERMS: {terms}")
        return terms

    except Exception as e:
        print(f"ERROR: {e}")
        return []

def test_query_generator():
    """Test using the actual ChunkQueryGenerator"""
    print("\n" + "="*60)
    print("TESTING ChunkQueryGenerator")
    print("="*60)

    from chunk_query_generator import ChunkQueryGenerator

    generator = ChunkQueryGenerator(model="qwen/qwen3-4b")
    result = generator.generate_search_terms(chunk, num_terms=3)

    print(f"Generated terms: {result.search_terms}")
    print(f"Generation model: {result.generation_model}")

    return result.search_terms

if __name__ == "__main__":
    print("DEBUGGING QUERY GENERATION FOR CHUNK 6509")
    print("Chunk text:", chunk_text[:100] + "...")
    print()

    # Test direct LLM
    direct_terms = test_direct_llm()

    # Test query generator
    generator_terms = test_query_generator()

    print("\n" + "="*60)
    print("COMPARISON")
    print("="*60)
    print(f"Direct LLM terms: {direct_terms}")
    print(f"Generator terms:  {generator_terms}")

    if direct_terms != generator_terms:
        print("\n❌ MISMATCH! The generator is not using LLM results properly.")
    else:
        print("\n✅ Results match!")