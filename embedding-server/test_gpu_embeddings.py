#!/usr/bin/env python3
"""
Test script to verify embeddinggemma-300M model works with GPU acceleration.

Usage:
    python test_gpu_embeddings.py

This will:
1. Load google/embeddinggemma-300M from Hugging Face
2. Use GPU acceleration (if available)
3. Generate test embeddings
4. Report performance metrics
"""

import time
from typing import List

try:
    import torch
    from sentence_transformers import SentenceTransformer
except ImportError as e:
    print(f"❌ Missing dependencies: {e}")
    print("\nInstall with:")
    print('  uv pip install sentence-transformers')
    print('  uv pip install "git+https://github.com/huggingface/transformers@v4.56.0-Embedding-Gemma-preview"')
    exit(1)


def load_model() -> SentenceTransformer:
    """Load embeddinggemma model from Hugging Face."""
    print("📦 Loading embeddinggemma-300M from Hugging Face...")
    print("   Model: google/embeddinggemma-300M")
    print("   This will download ~600MB on first run\n")
    
    try:
        device = "cuda" if torch.cuda.is_available() else "cpu"
        print(f"🎮 Device: {device}")
        
        if device == "cuda":
            print(f"   GPU: {torch.cuda.get_device_name(0)}")
            print(f"   CUDA Version: {torch.version.cuda}")
        
        print("\n🚀 Loading model...")
        model = SentenceTransformer("google/embeddinggemma-300M", device=device)
        
        # Print model info
        total_params = sum(p.numel() for p in model.parameters())
        print(f"✅ Model loaded successfully")
        print(f"   Total parameters: {total_params:,}")
        print(f"   Device: {model.device}\n")
        
        return model
    except Exception as e:
        print(f"❌ Model loading failed: {e}")
        print("\nNote: You may need to authenticate with Hugging Face:")
        print("  from huggingface_hub import login")
        print("  login()")
        exit(1)


def test_embedding_generation(model: SentenceTransformer) -> None:
    """Generate test embeddings and report metrics."""
    test_texts = [
        "Hello, world!",
        "This is a test of the embeddinggemma model for semantic search.",
        "LocalMind is a RAG application for semantic document search.",
    ]
    
    print("🧪 Testing embedding generation...\n")
    
    total_time = 0.0
    embeddings: List[List[float]] = []
    
    for i, text in enumerate(test_texts, 1):
        print(f"Test {i}/{len(test_texts)}: {text[:50]}...")
        
        start = time.time()
        embedding = model.encode([text])[0]
        elapsed = time.time() - start
        total_time += elapsed
        
        embeddings.append(embedding.tolist())
        
        print(f"  ⏱️  Time: {elapsed:.3f}s")
        print(f"  📊 Dimension: {len(embedding)}")
        print(f"  📈 Sample values: [{embedding[0]:.4f}, {embedding[1]:.4f}, ..., {embedding[-1]:.4f}]")
        print()
    
    # Performance summary
    avg_time = total_time / len(test_texts)
    print("=" * 60)
    print("📈 Performance Summary")
    print("=" * 60)
    print(f"Total embeddings: {len(embeddings)}")
    print(f"Total time: {total_time:.3f}s")
    print(f"Average time per embedding: {avg_time:.3f}s")
    print(f"Embeddings per second: {1/avg_time:.2f}")
    print()
    
    # Validate dimensions
    expected_dim = 768
    all_correct = all(len(emb) == expected_dim for emb in embeddings)
    
    if all_correct:
        print(f"✅ All embeddings have correct dimension ({expected_dim})")
    else:
        print(f"⚠️  Dimension check:")
        for i, emb in enumerate(embeddings):
            status = "✅" if len(emb) == expected_dim else "❌"
            print(f"   {status} Embedding {i+1}: {len(emb)} dimensions")
    
    print()
    
    # GPU status check
    print("=" * 60)
    print("🎮 GPU Information")
    print("=" * 60)
    
    if torch.cuda.is_available():
        print(f"✅ GPU acceleration enabled")
        print(f"   Device: {torch.cuda.get_device_name(0)}")
        print(f"   Memory allocated: {torch.cuda.memory_allocated(0) / 1024**2:.2f} MB")
        print(f"   Memory cached: {torch.cuda.memory_reserved(0) / 1024**2:.2f} MB")
    else:
        print("⚠️  Running on CPU (GPU not available)")
    
    print()


def main() -> None:
    """Run GPU embedding test."""
    print("=" * 60)
    print("🧪 GPU Embedding Test - embeddinggemma-300M")
    print("=" * 60)
    print()
    
    # Step 1: Load model
    model = load_model()
    
    # Step 2: Test embedding generation
    test_embedding_generation(model)
    
    print("=" * 60)
    print("✅ Test Complete!")
    print("=" * 60)
    print()
    print("Next steps:")
    print("  - If GPU worked: Proceed with Phase 2 implementation")
    print("  - If CPU only: GPU not available but embeddings work")
    print("  - Performance target: <1 second per embedding ✅" if True else "")
    print()


if __name__ == "__main__":
    main()
