#!/usr/bin/env python3
"""
Simple test of the fixed generator
"""

from chunk_query_generator_fixed import ChunkQueryGenerator
from chunk_quality_filter import QualityChunkSample

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

print("Testing Fixed Query Generator...")
generator = ChunkQueryGenerator(model="qwen/qwen3-4b")

try:
    result = generator.generate_search_terms(chunk, num_terms=3)
    print("SUCCESS!")
    print("Generated terms:", result.search_terms)
    print("Model:", result.generation_model)
except Exception as e:
    print("FAILED:", str(e))