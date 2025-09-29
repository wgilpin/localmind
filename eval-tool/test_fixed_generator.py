#!/usr/bin/env python3
"""
Test the fixed query generator
"""

from chunk_query_generator_fixed import ChunkQueryGenerator
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

def test_fixed_generator():
    """Test using the fixed ChunkQueryGenerator"""
    print("="*60)
    print("TESTING FIXED ChunkQueryGenerator")
    print("="*60)

    generator = ChunkQueryGenerator(model="qwen/qwen3-4b")

    try:
        result = generator.generate_search_terms(chunk, num_terms=3)
        print(f"✅ SUCCESS!")
        print(f"Generated terms: {result.search_terms}")
        print(f"Generation model: {result.generation_model}")
        return result.search_terms
    except Exception as e:
        print(f"❌ FAILED: {e}")
        return []

if __name__ == "__main__":
    print("TESTING FIXED QUERY GENERATOR FOR CHUNK 6509")
    print("Chunk text:", chunk_text[:100] + "...")
    print()

    # Test fixed generator
    fixed_terms = test_fixed_generator()

    print("\n" + "="*60)
    print("EXPECTED vs ACTUAL")
    print("="*60)
    print("Expected (manual test): ['llm evaluation tools', 'anthropic eval framework', 'python eval features']")
    print(f"Actual (fixed generator): {fixed_terms}")

    if fixed_terms and len(fixed_terms) == 3:
        print("\n✅ SUCCESS: Generated 3 proper search terms!")
        for i, term in enumerate(fixed_terms, 1):
            print(f"  {i}. '{term}'")
    else:
        print(f"\n❌ FAILED: Expected 3 terms, got {len(fixed_terms) if fixed_terms else 0}")