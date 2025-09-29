#!/usr/bin/env python3
"""Quick script to save embeddings that were already generated"""

import sys
import traceback
from vector_evaluator import VectorEvaluator

def main():
    try:
        # The embeddings should still be in the VectorEvaluator instance
        # Let's check if there are any existing embeddings we can recover
        evaluator = VectorEvaluator(
            persist_directory="./vector_store_eval",
            embedding_model="qwen3-embedding:0.6b",
            use_ollama=True
        )

        # Try to load existing vectors first
        if evaluator.load_vectors():
            print(f"Found existing embeddings: {len(evaluator.vectors_df)} documents")
            print("Embeddings are already saved!")
        else:
            print("No existing embeddings found in vector_store_eval directory")
            print("The embeddings from your interrupted run may have been lost")

    except Exception as e:
        print(f"Error: {e}")
        traceback.print_exc()

if __name__ == '__main__':
    main()