# Feature Specification: Python Embedding Server

**Feature Branch**: `006-python-embedding-server`  
**Created**: 2025-01-21  
**Status**: Draft  
**Input**: Replace embed_anything Rust implementation with Python FastAPI server using llama-cpp-python for embeddinggemma-300m-qat GGUF model

## User Scenarios & Testing

### User Story 1 - Automatic Server Lifecycle (Priority: P1)

User runs `start_localmind.bat` and the Python embedding server starts automatically in the background, downloads the model on first run, and provides embeddings to the Rust app without user intervention.

**Why this priority**: Core functionality - embeddings are required for all document indexing and search.

**Independent Test**: Run `start_localmind.bat`, add a document, verify it gets embedded and indexed. Server lifecycle is transparent to user.

**Acceptance Scenarios**:

1. **Given** fresh install, **When** user runs `start_localmind.bat`, **Then** Python venv is created, dependencies installed, embeddinggemma model downloaded (~300MB), server starts on localhost:8000
2. **Given** subsequent startups, **When** user runs `start_localmind.bat`, **Then** server starts immediately using cached model (<5 seconds)
3. **Given** server crashes, **When** Rust app calls embedding endpoint, **Then** clear error message returned to user with actionable fix

---

### User Story 2 - Document Embedding Performance (Priority: P1)

Documents are embedded with comparable or better performance than previous Qwen3 CPU implementation, ideally <1 second per chunk.

**Why this priority**: Slow embeddings block document ingestion and re-embedding workflows.

**Independent Test**: Add 10 documents (~50 chunks), measure embedding time. Should complete in <30 seconds total.

**Acceptance Scenarios**:

1. **Given** server is running, **When** Rust app sends text for embedding, **Then** response returned in <1 second per chunk
2. **Given** batch of 50 chunks, **When** re-embedding script runs, **Then** all embeddings complete in <30 seconds

---

### Edge Cases

- What happens when embedding server port 8000 is already in use?
- How does system handle embeddinggemma model download failures?
- What if Python is not installed or version <3.11?
- How does cleanup work when user closes LocalMind?

## Requirements

### Functional Requirements

- **FR-001**: System MUST provide a Python FastAPI server that exposes `/embed` endpoint accepting text and returning 768-dimensional vectors
- **FR-002**: System MUST use `llama-cpp-python` to load `ggml-org/embeddinggemma-300m-qat-q8_0-GGUF/embeddinggemma-300m-qat-Q8_0.gguf` model
- **FR-003**: System MUST use `uv` for Python package management and virtual environment creation
- **FR-004**: System MUST download model automatically via `hf-hub-download` on first run
- **FR-005**: System MUST cache model in Hugging Face cache directory (`~/.cache/huggingface/`)
- **FR-006**: Rust `LocalEmbeddingClient` MUST be refactored to HTTP client calling Python server
- **FR-007**: `start_localmind.bat` MUST manage Python server lifecycle (create venv, install deps, start server, launch Rust app, cleanup on exit)
- **FR-008**: Server MUST respond with health check on `/health` endpoint
- **FR-009**: Server MUST log startup, model loading, and request handling to console
- **FR-010**: Server MUST run on localhost:8000 (configurable via environment variable)
- **FR-011**: All Python code MUST follow Principle VI (TypedDict, type hints, no `Any`, ruff/mypy validation)
- **FR-012**: Dependencies MUST be declared in `pyproject.toml` with locked versions

### Key Entities

- **EmbeddingRequest**: Text input to be embedded (TypedDict with `text: str` field)
- **EmbeddingResponse**: 768-dimensional float vector output (TypedDict with `embedding: list[float]`, `model: str`, `dimension: int`)
- **HealthResponse**: Server health status (TypedDict with `status: str`, `model_loaded: bool`)

## Success Criteria

### Measurable Outcomes

- **SC-001**: User can add and search documents without noticing embedding implementation changed (transparent migration)
- **SC-002**: Embedding generation completes in <1 second per chunk (90th percentile)
- **SC-003**: First-run setup (venv + model download) completes in <10 minutes on typical broadband
- **SC-004**: Zero C/C++ compilation required (llama-cpp-python uses pre-built wheels)
- **SC-005**: Re-embedding 1000 chunks completes in <10 minutes
- **SC-006**: Python code passes `ruff check`, `ruff format --check`, and `mypy --strict` with zero errors

## Technical Approach

### Python Server (`embedding_server.py`)

```python
from typing import TypedDict
from fastapi import FastAPI
from llama_cpp import Llama
from huggingface_hub import hf_hub_download
import uvicorn

class EmbeddingRequest(TypedDict):
    text: str

class EmbeddingResponse(TypedDict):
    embedding: list[float]
    model: str
    dimension: int

# Download and load model on startup
# Server runs on localhost:8000
# /embed endpoint accepts POST with JSON body
# Returns 768-dim vector
```

### Rust Client (`localmind-rs/src/local_embedding.rs`)

```rust
// Replace embed_anything with reqwest HTTP client
// POST to http://localhost:8000/embed
// Deserialize JSON response to Vec<f32>
```

### Startup Script (`localmind-rs/start_localmind.bat`)

```batch
REM 1. Check Python 3.11+ installed
REM 2. Create venv if not exists: uv venv
REM 3. Activate venv
REM 4. Install dependencies: uv pip install -r requirements.txt
REM 5. Start server in background: python embedding_server.py
REM 6. Wait for health check on localhost:8000/health
REM 7. Launch Rust app: cargo tauri dev --release
REM 8. On exit: kill Python server process
```

### Dependencies (`pyproject.toml`)

```toml
[project]
dependencies = [
    "fastapi>=0.115.0",
    "uvicorn[standard]>=0.32.0",
    "llama-cpp-python>=0.3.0",
    "huggingface-hub>=0.26.0",
]
```

## Migration Notes

- **Breaking Change**: `embed_anything` dependency removed from `Cargo.toml`
- **Data Migration**: None - embeddings table schema unchanged (still 768-dim vectors)
- **User Impact**: None if migration successful; must re-embed documents if embedding dimension changed
- **Rollback**: Keep `embed_anything` implementation in git history for emergency rollback

## Non-Goals

- GPU acceleration (future enhancement)
- Batch embedding endpoint (future enhancement)
- Multiple model support (single model: embeddinggemma-300m-qat)
- Distributed deployment (localhost only)
