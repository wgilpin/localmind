#!/bin/bash

echo "Testing LM Studio connection..."
python test_lmstudio.py

echo ""
echo "Starting pipeline with LM Studio..."
python main.py run-chunk-pipeline \
    --sample-size 200 \
    --llm-model qwen3:4b \
    --embedding-model text-embedding-qwen3-embedding-0.6b \
    --lmstudio \
    --top-k 5 