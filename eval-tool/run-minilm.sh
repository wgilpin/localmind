#!/bin/bash

echo "Testing LM Studio connection..."
python test_lmstudio.py

echo ""
echo "Starting pipeline with LM Studio..."
python main.py run-chunk-pipeline \
    --sample-size 200 \
    --llm-model qwen3:4b \
    --embedding-model all-MiniLM-L6-v2 \
    --sentence-transformers \
    --top-k 5