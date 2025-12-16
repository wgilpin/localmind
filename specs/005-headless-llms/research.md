# Research: Headless LLM Migration

**Date**: 2025-01-21  
**Feature**: 005-headless-llms  
**Status**: Complete

## Overview

This document captures research findings for implementing a Python FastAPI embedding server with llama-cpp-python to replace the external LMStudio dependency.

## Key Research Questions

### Q1: How to run GGUF embedding models without C/C++ compilation?

**Decision**: Use `llama-cpp-python` with pre-built wheels

**Rationale**:
- llama.cpp is the reference implementation for GGUF format
- Pre-built wheels available for Windows, macOS, Linux (x86_64, ARM64)
- Actively maintained with excellent performance
- Supports embeddinggemma models out-of-the-box

**Alternatives Considered**:
- **Candle (Rust)**: No native GGUF support for embeddinggemma, would require manual transformer layer implementation
- **ggml (C++)**: Requires compilation, violates SC-012 (zero C/C++ compilation)
- **HuggingFace transformers**: Doesn't support GGUF format, only safetensors/PyTorch

**Evidence**:
- llama-cpp-python PyPI page shows pre-built wheels for all major platforms
- embeddinggemma GGUF models confirmed working in llama.cpp ecosystem
- Zero compilation required when installing from PyPI wheels

---

### Q2: Best architecture for Rust + Python integration?

**Decision**: Localhost HTTP API with FastAPI server

**Rationale**:
- Clean separation of concerns (Rust handles app logic, Python handles embeddings)
- Easy to debug and monitor (HTTP requests visible in logs)
- Startup script can manage lifecycle transparently
- Well-understood patterns (REST API)
- Type-safe with Pydantic/TypedDict validation

**Alternatives Considered**:
- **PyO3 (Python embedded in Rust)**: Complex build, GIL contention, difficult debugging
- **Subprocess with stdin/stdout**: Fragile, difficult error handling, no standard protocol
- **Shared memory/IPC**: Platform-specific, complex error handling

**Best Practices**:
- Use FastAPI for auto-generated OpenAPI docs and type validation
- Use `uvicorn` with `--reload` for development
- Health check endpoint (`/health`) for startup readiness
- Structured logging to console for debugging

---

### Q3: How to manage Python dependencies and virtual environments?

**Decision**: Use `uv` for all package management

**Rationale**:
- 10-100x faster than pip/poetry
- Reproducible installs with lock files
- Simple CLI (`uv venv`, `uv pip install`)
- Complies with Constitution Principle VI

**Alternatives Considered**:
- **pip**: Slow, no lock files, resolver conflicts
- **poetry**: Complex `pyproject.toml`, slow installs, heavy dependency
- **conda**: Large install, overkill for this use case

**Best Practices**:
- Use `pyproject.toml` for dependency declaration
- Use `uv pip install -e .` for local development
- Use `.python-version` file for version pinning (3.11+)
- Virtual environment in `.venv/` directory (gitignored)

---

### Q4: How to ensure type safety in Python code?

**Decision**: Use TypedDict + mypy --strict

**Rationale**:
- TypedDict provides IDE autocomplete and runtime validation
- mypy --strict catches type errors at development time
- Aligns with Constitution Principle VI
- FastAPI integrates seamlessly with type hints

**Best Practices**:
- Define TypedDict for all request/response bodies:
  ```python
  from typing import TypedDict
  
  class EmbeddingRequest(TypedDict):
      text: str
  
  class EmbeddingResponse(TypedDict):
      embedding: list[float]
      model: str
      dimension: int
  ```
- Use explicit type hints on all function arguments and return values
- Avoid `Any` type - use specific types or generics (`TypeVar`)
- Run `mypy --strict` in pre-commit checks

---

### Q5: How to handle model downloading and caching?

**Decision**: Use `huggingface_hub.hf_hub_download()` with automatic caching

**Rationale**:
- Standard HuggingFace cache directory (`~/.cache/huggingface/`)
- Automatic retry on network failures
- Resumable downloads for large files
- Cross-platform (Windows, macOS, Linux)

**Best Practices**:
```python
from huggingface_hub import hf_hub_download

model_path = hf_hub_download(
    repo_id="ggml-org/embeddinggemma-300m-qat-q8_0-GGUF",
    filename="embeddinggemma-300m-qat-Q8_0.gguf",
    cache_dir=None,  # Use default cache
)
```

- Check if model exists in cache before downloading
- Log download progress to console
- Handle network errors with retry + exponential backoff

---

### Q6: How to handle startup script lifecycle management?

**Decision**: Batch script tracks Python server PID and terminates on exit

**Rationale**:
- Simple and reliable for single-user desktop app
- No need for systemd/launchd on desktop platforms
- User can see server logs in same terminal window

**Best Practices** (Windows .bat):
```batch
REM 1. Check Python version
python --version

REM 2. Create venv if needed
if not exist .venv\ (
    uv venv
)

REM 3. Activate venv
call .venv\Scripts\activate.bat

REM 4. Install dependencies
uv pip install -e .

REM 5. Start server in background, save PID
start /B python embedding_server.py > server.log 2>&1
set SERVER_PID=%ERRORLEVEL%

REM 6. Wait for health check
:wait_loop
curl -s http://localhost:8000/health && goto :server_ready
timeout /t 1 /nobreak > nul
goto :wait_loop

:server_ready
REM 7. Launch Rust app
cargo tauri dev --release

REM 8. Kill server on exit
taskkill /PID %SERVER_PID% /F
```

---

### Q7: How to ensure embedding dimension compatibility (768)?

**Decision**: Verify embedding dimension in Python server startup and include in response

**Rationale**:
- embeddinggemma-300m-qat produces 768-dimensional embeddings
- Existing database schema expects 768 dimensions
- Runtime validation prevents silent failures

**Best Practices**:
```python
# On startup
model = Llama(model_path=model_path, embedding=True)
test_embed = model.create_embedding("test")
assert len(test_embed['embedding']) == 768, f"Expected 768 dims, got {len(test_embed['embedding'])}"

# In response
class EmbeddingResponse(TypedDict):
    embedding: list[float]
    model: str
    dimension: int  # Always 768
```

---

### Q8: How to handle "loading" state during model initialization?

**Decision**: Return HTTP 503 Service Unavailable with Retry-After header

**Rationale**:
- Standard HTTP semantics for temporary unavailability
- Retry-After header tells client when to retry
- Rust client can implement backoff automatically

**Best Practices**:
```python
from fastapi import FastAPI, HTTPException
from fastapi.responses import JSONResponse

app = FastAPI()
model_loaded = False

@app.post("/embed")
async def embed(request: EmbeddingRequest) -> EmbeddingResponse:
    if not model_loaded:
        return JSONResponse(
            status_code=503,
            headers={"Retry-After": "5"},
            content={"error": "Model still loading, please retry"}
        )
    # ... embedding logic
```

---

## Technology Choices Summary

| Component | Technology | Justification |
|-----------|-----------|---------------|
| GGUF Runtime | llama-cpp-python | Pre-built wheels, no compilation, reference implementation |
| HTTP Framework | FastAPI | Type-safe, async, auto-docs, minimal code |
| HTTP Server | uvicorn | Production-ready ASGI server, async support |
| Package Manager | uv | 10-100x faster than pip, reproducible |
| Type Checker | mypy --strict | Strict type safety, Constitution compliant |
| Formatter | ruff format | Fast, Constitution compliant |
| Linter | ruff check | Fast, comprehensive rules |
| Model Cache | HuggingFace Hub | Standard caching, resumable downloads |
| HTTP Client (Rust) | reqwest | Well-tested, async, serde integration |

---

## Performance Estimates

Based on llama-cpp-python benchmarks:

- **Model Loading**: 2-5 seconds (from cache)
- **Embedding Generation** (CPU): 50-200ms per chunk (depending on hardware)
- **HTTP Overhead**: 1-5ms (localhost)
- **Total Latency**: 50-205ms per chunk (within SC-005 requirement of <1s)

**Re-embedding 1000 chunks**:
- Optimistic: 50s (50ms/chunk)
- Realistic: 3-5 minutes (180-300ms/chunk average)
- Well within SC-008 requirement of <10 minutes

---

## Risk Mitigation

### Risk 1: Pre-built wheels not available for user's platform

**Mitigation**: Startup script checks Python version and provides clear error if llama-cpp-python installation fails
**Fallback**: Document manual compilation instructions in error message

### Risk 2: Port 8000 already in use

**Mitigation**: Startup script checks port availability before starting server
**Fallback**: Make port configurable via environment variable `EMBEDDING_SERVER_PORT`

### Risk 3: Python not installed or wrong version

**Mitigation**: Startup script checks for Python 3.13+ before proceeding
**Fallback**: Provide clear error message with download link to python.org

### Risk 4: Model download fails or is corrupted

**Mitigation**: huggingface_hub handles retries and validates checksums
**Fallback**: Manual retry option in error message, delete cache and re-download

---

## Next Steps

Phase 1 will generate:
1. `data-model.md`: TypedDict definitions for request/response bodies
2. `contracts/`: OpenAPI schema for `/embed` and `/health` endpoints
3. `quickstart.md`: Step-by-step guide for first-time setup

Ready to proceed to Phase 1: Design & Contracts.
