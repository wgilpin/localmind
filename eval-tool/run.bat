@echo off

REM Force Ollama to use GPU
set OLLAMA_NUM_GPU=999
set CUDA_VISIBLE_DEVICES=0

REM Check GPU availability
echo Checking GPU status...
nvidia-smi

echo.
echo Starting pipeline with GPU acceleration...
python main.py run-chunk-pipeline --sample-size 20 --llm-model qwen3:4b-gpu --embedding-model all-MiniLM-L6-v2 --top-k 5 --reset