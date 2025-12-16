# LocalMind Embedding Server Setup Guide

This guide documents the Python embedding server architecture used by LocalMind.

## Architecture Overview

LocalMind uses a **local Python embedding server** to generate vector embeddings for semantic search. The server runs alongside the Rust application and provides embeddings without requiring external LLM services.

### Components

1. **Python Embedding Server** (`embedding-server/embedding_server.py`)
   - FastAPI-based HTTP server
   - Uses `sentence-transformers` with `google/embeddinggemma-300M` model
   - Runs on `localhost:8000` (configurable via `EMBEDDING_SERVER_PORT`)

2. **Rust HTTP Client** (`localmind-rs/src/local_embedding.rs`)
   - Communicates with Python server via HTTP
   - Handles retry logic and error recovery
   - Validates embedding dimensions (768)

## Quick Start

### Automated Startup

Use the startup script in the repository root:

**Windows:**
```batch
start_localmind.bat
```

**Unix/Linux/macOS:**
```bash
./start_localmind.sh
```

The script will:
1. Check for Python 3.11+ installation
2. Install `uv` if needed
3. Create virtual environment in `embedding-server/.venv`
4. Install dependencies
5. Start Python embedding server in background
6. Wait for server to be ready
7. Launch LocalMind application

### Manual Startup

If you prefer to start components manually:

#### 1. Start Python Embedding Server

```bash
cd embedding-server
python -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate.bat
uv pip install -e .
python embedding_server.py
```

The server will start on `http://localhost:8000` by default.

#### 2. Start LocalMind Application

In a separate terminal:

```bash
cd localmind-rs
cargo tauri dev --release
```

## Model Information

### Embedding Model

**Model:** `google/embeddinggemma-300M`  
**Format:** PyTorch (via Hugging Face)  
**Dimensions:** 768  
**Size:** ~600MB (downloads automatically on first run)  
**Provider:** Google (via Hugging Face)

### Model Download

The model is automatically downloaded from Hugging Face on first run. You may need to authenticate:

```python
from huggingface_hub import login
login()
```

Enter your Hugging Face token when prompted.

## Configuration

### Environment Variables

- `EMBEDDING_SERVER_PORT`: Port for the embedding server (default: 8000)

Example:
```bash
export EMBEDDING_SERVER_PORT=9000
./start_localmind.sh
```

### Server Endpoints

- `GET /health`: Health check endpoint
  - Returns: `{"status": "ready", "model_loaded": true}`
  
- `POST /embed`: Generate embedding
  - Request: `{"text": "your text here"}`
  - Response: `{"embedding": [0.1, 0.2, ...], "model": "google/embeddinggemma-300M", "dimension": 768}`

## Dependencies

### Python Dependencies

Managed via `embedding-server/pyproject.toml`:
- `fastapi>=0.115.0`: Web framework
- `uvicorn[standard]>=0.32.0`: ASGI server
- `sentence-transformers>=3.3.0`: Embedding library
- `torch>=2.0.0`: PyTorch (for GPU acceleration)
- `transformers @ git+https://github.com/huggingface/transformers@v4.56.0-Embedding-Gemma-preview`: Gemma support

### Development Tools

- `mypy>=1.8.0`: Type checking
- `ruff>=0.6.0`: Linting and formatting

## GPU Acceleration

The embedding server automatically detects and uses GPU if available:

- **CUDA**: Automatically used if `torch.cuda.is_available()` returns `true`
- **CPU**: Falls back to CPU if no GPU is available

GPU acceleration significantly improves embedding generation speed.

## Troubleshooting

### "Failed to connect to embedding server"

- Ensure the Python server is running: `cd embedding-server && python embedding_server.py`
- Check the server logs: `embedding-server/embedding_server.log`
- Verify the port: `curl http://localhost:8000/health`

### "Model loading failed" / "401 Client Error"

- Authenticate with Hugging Face:
  ```python
  from huggingface_hub import login
  login()
  ```
- Ensure you have access to `google/embeddinggemma-300M` on Hugging Face

### "Out of memory" errors

- The model requires ~1GB RAM
- Close other applications
- Ensure you have at least 1.2GB free memory

### Server takes too long to start

- First run: Model download can take several minutes (~600MB)
- Subsequent runs: Model loads from cache (typically <30 seconds)
- Check logs: `embedding-server/embedding_server.log`

### Port already in use

- Change the port: `export EMBEDDING_SERVER_PORT=9000`
- Or stop the existing server on port 8000

## Development

### Running Tests

```bash
cd embedding-server
source .venv/bin/activate
python test_gpu_embeddings.py  # Test GPU embedding generation
```

### Code Quality

```bash
# Type checking
mypy --strict embedding_server.py

# Linting
ruff check embedding_server.py

# Formatting
ruff format embedding_server.py
```

## Architecture Benefits

1. **No External Dependencies**: Everything runs locally
2. **No C/C++ Compilation**: Pure Python/PyTorch, uses pre-built wheels
3. **GPU Acceleration**: Automatic detection and usage
4. **Simple Deployment**: Single Python server process
5. **Isolated Environment**: Virtual environment keeps dependencies clean

## Next Steps

After setup:
1. Add documents to LocalMind
2. Documents are automatically embedded using the local server
3. Perform semantic searches across your knowledge base
4. No external services required!

For more information, see the main README.md
