# Quickstart: Headless LLM Migration

**Feature**: 005-headless-llms  
**Date**: 2025-01-21

## Overview

This guide walks through the setup and first-time use of the new Python embedding server architecture.

## Prerequisites

Before starting, ensure you have:

- ✅ **Python 3.13+** installed ([download](https://www.python.org/downloads/))
- ✅ **Rust 1.75+** and Cargo installed
- ✅ **uv** installed (`pip install uv` or `cargo install uv`)
- ✅ **Internet connection** (for first-time model download only)
- ✅ **~500MB free disk space** (for model cache)
- ✅ **~1.5GB free RAM** (for model + application)
- ✅ **Port 8000 available** (or set `EMBEDDING_SERVER_PORT` environment variable)

## First-Time Setup (Automated)

### Windows

```batch
cd C:\Users\wgilp\projects\localmind\localmind-rs
start_localmind.bat
```

### macOS / Linux

```bash
cd /path/to/localmind/localmind-rs
./start_localmind.sh
```

### What Happens Automatically

1. **Python Version Check**: Script verifies Python 3.13+ is installed
2. **Virtual Environment**: Creates `.venv/` if it doesn't exist
3. **Dependency Installation**: Installs FastAPI, uvicorn, llama-cpp-python, huggingface-hub via `uv`
4. **Model Download**: Downloads embeddinggemma-300m-qat GGUF (~300MB) to Hugging Face cache
5. **Server Startup**: Starts Python embedding server on localhost:8000
6. **Health Check**: Waits for `/health` endpoint to return "ok"
7. **Application Launch**: Starts LocalMind Rust application
8. **Cleanup**: Terminates Python server when you close LocalMind

**First run**: Expect 5-10 minutes for dependency installation and model download  
**Subsequent runs**: ~30 seconds (model loads from cache)

## Manual Setup (For Development)

### 1. Install Python Dependencies

```bash
cd /path/to/localmind

# Create virtual environment
uv venv

# Activate (Windows)
.venv\Scripts\activate.bat

# Activate (macOS/Linux)
source .venv/bin/activate

# Install dependencies
uv pip install -e .
```

### 2. Verify Installation

```bash
# Check Python version
python --version  # Should be 3.11+

# Check installed packages
uv pip list

# Should see:
# fastapi>=0.115.0
# uvicorn[standard]>=0.32.0
# llama-cpp-python>=0.3.0
# huggingface-hub>=0.26.0
```

### 3. Start Embedding Server Manually

```bash
python embedding_server.py
```

Expected output:
```
INFO:     Loading embeddinggemma-300m-qat from Hugging Face...
INFO:     Model cached at: C:\Users\wgilp\.cache\huggingface\hub\...
INFO:     Model loaded successfully (768 dimensions)
INFO:     Started server process [12345]
INFO:     Waiting for application startup.
INFO:     Application startup complete.
INFO:     Uvicorn running on http://localhost:8000
```

### 4. Test Embedding Endpoint

```bash
# Health check
curl http://localhost:8000/health

# Expected:
# {"status":"ok","model_loaded":true}

# Generate embedding
curl -X POST http://localhost:8000/embed \
  -H "Content-Type: application/json" \
  -d '{"text":"test document chunk"}'

# Expected:
# {"embedding":[0.123,-0.456,...],"model":"embeddinggemma-300m-qat","dimension":768}
```

### 5. Run LocalMind Application

In a separate terminal:

```bash
cd localmind-rs
cargo tauri dev --release
```

## Verification Checklist

After setup, verify:

- [ ] Python embedding server starts without errors
- [ ] `/health` endpoint returns `{"status":"ok","model_loaded":true}`
- [ ] `/embed` endpoint generates 768-dimensional vectors
- [ ] LocalMind application starts and connects to embedding server
- [ ] Document indexing works (test by adding a document)
- [ ] Semantic search works (test by searching indexed documents)
- [ ] No external network requests after initial setup (use network monitor)
- [ ] Python server terminates cleanly when LocalMind closes

## Troubleshooting

### Problem: "Python not found"

**Solution**: Install Python 3.13+ from [python.org](https://www.python.org/downloads/)

**Verify**: `python --version` should show 3.11 or higher

---

### Problem: "Port 8000 already in use"

**Solution**: Find and kill the process using port 8000

```bash
# Windows
netstat -ano | findstr :8000
taskkill /PID <PID> /F

# macOS/Linux
lsof -i :8000
kill <PID>
```

**Alternative**: Set custom port via environment variable

```bash
# Windows
set EMBEDDING_SERVER_PORT=8001
start_localmind.bat

# macOS/Linux
export EMBEDDING_SERVER_PORT=8001
./start_localmind.sh
```

---

### Problem: "Model download failed"

**Solution**: Check internet connection and retry

**Manual retry**:
```bash
# Delete corrupted cache
rm -rf ~/.cache/huggingface/hub/models--ggml-org--embeddinggemma-300m-qat-q8_0-GGUF

# Restart server (will re-download)
python embedding_server.py
```

---

### Problem: "llama-cpp-python installation failed"

**Cause**: Pre-built wheel not available for your platform

**Solution**: Check if your platform is supported at [llama-cpp-python PyPI](https://pypi.org/project/llama-cpp-python/)

**Supported platforms**: Windows (x64), macOS (x64, ARM64), Linux (x64, ARM64)

**Manual compilation** (if needed):
```bash
# Install build tools
# Windows: Install Visual Studio Build Tools
# macOS: xcode-select --install
# Linux: sudo apt-get install build-essential

# Install from source
uv pip install llama-cpp-python --no-binary llama-cpp-python
```

---

### Problem: "Embeddings are slow (>1s per chunk)"

**Cause**: Model running on CPU

**Solution**: This is expected for CPU inference. embeddinggemma-300m-qat is CPU-optimized but still takes 50-200ms per chunk.

**Verify performance**:
```bash
# Time a single embedding
time curl -X POST http://localhost:8000/embed \
  -H "Content-Type: application/json" \
  -d '{"text":"test"}'
```

**Expected**: 50-200ms on modern CPUs

---

### Problem: "Rust application says 'Model still loading'"

**Cause**: Model download/loading still in progress

**Solution**: Wait for Python server logs to show "Model loaded successfully"

**Check server logs**: Look for:
```
INFO:     Model loaded successfully (768 dimensions)
INFO:     Application startup complete.
```

**Typical wait time**: 30-60 seconds for first load

---

## Advanced Configuration

### Custom Model Cache Location

```bash
# Set HuggingFace cache directory
export HF_HOME=/custom/path/to/cache
python embedding_server.py
```

### Custom Port

```bash
# Set custom port via environment variable
export EMBEDDING_SERVER_PORT=9000
python embedding_server.py
```

Update Rust client in `localmind-rs/src/local_embedding.rs`:
```rust
const EMBEDDING_SERVER_URL: &str = "http://localhost:9000";
```

### Enable Debug Logging

```bash
# Python server
export LOG_LEVEL=DEBUG
python embedding_server.py

# Rust application
RUST_LOG=debug cargo tauri dev --release
```

## Performance Benchmarks

Expected performance on various hardware:

| Hardware | Model Load Time | Per-Chunk Latency | 1000 Chunks |
|----------|----------------|-------------------|-------------|
| Intel i7 (2020+) | 3-5s | 80-120ms | 2-3 min |
| M1 Mac | 2-3s | 50-80ms | 1-2 min |
| Older CPU (<2015) | 5-10s | 150-300ms | 3-5 min |

All within success criteria (<1s per chunk, <10 min for 1000 chunks).

## Next Steps

1. ✅ Verify setup with checklist above
2. ✅ Index test documents
3. ✅ Perform test searches
4. ✅ Monitor resource usage (Task Manager / Activity Monitor)
5. ✅ Review `embedding_server.py` code for customization

## Getting Help

If you encounter issues not covered above:

1. Check Python server logs in `server.log`
2. Check Rust application logs in terminal
3. Verify all prerequisites are met
4. Try manual setup steps to isolate the issue
5. Check network monitor to confirm no external requests

## Developer Notes

### Running Tests

```bash
# Python type checking
mypy --strict embedding_server.py

# Python linting
ruff check embedding_server.py
ruff format --check embedding_server.py

# Rust tests
cd localmind-rs
cargo test --all
cargo clippy -- -D warnings
cargo fmt -- --check
```

### Profiling Performance

```bash
# Profile embedding generation
python -m cProfile -o profile.stats embedding_server.py
python -m pstats profile.stats

# Analyze with snakeviz
pip install snakeviz
snakeviz profile.stats
```

### Monitoring Resource Usage

```bash
# Monitor Python server memory
# Windows
tasklist /FI "IMAGENAME eq python.exe" /FO TABLE

# macOS/Linux
ps aux | grep python
```

Expected: ~1GB RSS for Python server (model loaded)

---

**Setup Complete!** You're now running LocalMind with the new Python embedding server architecture.
