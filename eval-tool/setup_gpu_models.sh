#!/bin/bash

echo "Creating GPU-optimized models for eval-tool..."

# Create GPU-optimized version of qwen3:4b for LLM tasks
echo "Creating qwen3:4b-gpu..."
ollama create qwen3:4b-gpu -f qwen3-4b-gpu.modelfile

# Create GPU-optimized embedding models (if not already done)
echo "Creating qwen3-embedding-gpu..."
ollama create qwen3-embedding-gpu -f qwen3-gpu.modelfile

echo "Creating nomic-embed-text-gpu..."
ollama create nomic-embed-text-gpu -f nomic-gpu.modelfile

echo "Done! GPU models created:"
ollama list | grep -E "(gpu|GPU)"

echo ""
echo "To use GPU models in eval-tool:"
echo "  For LLM tasks: --llm-model qwen3:4b-gpu"
echo "  For embeddings: --embedding-model qwen3-embedding-gpu --ollama"