#!/bin/bash
cd eval-tool
uv pip install -r requirements.txt
uv run python main.py run-all --sample-size 200 --model qwen3:4b
