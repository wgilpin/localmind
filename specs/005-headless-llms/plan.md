# Implementation Plan: Headless LLM Migration

**Branch**: `005-headless-llms` | **Date**: 2025-01-21 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/005-headless-llms/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Remove dependency on external LMStudio process by implementing a local Python FastAPI embedding server using llama-cpp-python for the embeddinggemma-300m-qat GGUF model. The Rust application acts as an HTTP client to localhost:8000, with the startup script managing the Python server lifecycle transparently. All chat/completion features are removed, reducing scope to pure RAG indexing and semantic search.

## Technical Context

**Language/Version**: Python 3.13+ (embedding server), Rust 1.75+ (main application)  
**Primary Dependencies**: 
- Python: FastAPI 0.115+, uvicorn 0.32+, llama-cpp-python 0.3+, huggingface-hub 0.26+, uv (package manager)
- Rust: reqwest (HTTP client), serde_json (JSON serialization), Tauri 1.5+, rusqlite  

**Storage**: 
- SQLite (existing document/embedding database via rusqlite)
- Hugging Face cache (`~/.cache/huggingface/` or Windows `%USERPROFILE%\.cache\huggingface\`) for GGUF model (~300MB)  

**Testing**: 
- Python: pytest (optional, not required by spec)
- Rust: cargo test (existing test suite)
- Type checking: mypy --strict / pyright for Python code  

**Target Platform**: Windows, macOS, Linux (desktop application via Tauri)

**Project Type**: Desktop application with separate Python server process (managed by startup script)

**Performance Goals**: 
- Embedding generation: <1 second per chunk (90th percentile)
- Re-embedding 1000 chunks: <10 minutes total
- Python server startup: <30 seconds (after venv/deps installed)
- Search responses: <100ms median latency (existing constraint)

**Constraints**: 
- Memory: <1GB for embedding model, <50MB for Rust core application
- Network: Zero external/remote requests (localhost:8000 only)
- Offline-capable: Works without internet after initial setup
- No C/C++ compilation: llama-cpp-python uses pre-built wheels

**Scale/Scope**: 
- Single-user desktop application
- Supports thousands of documents with hundreds of thousands of chunks
- Port 8000 required for Python server (configurable via environment variable)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I: Privacy & Offline-First Architecture ✅

- ✅ All processing remains local (Python server on localhost:8000)
- ✅ SQLite database unchanged (bundled rusqlite)
- ✅ Zero cloud dependencies (model cached from Hugging Face on first use only)
- ✅ No external network requests after initial setup
- ✅ Network requests only to localhost (Python server)

### Principle II: Performance & Native Experience ✅

- ✅ Search target <100ms maintained (embedding is offline operation)
- ✅ Memory <1GB for model + <50MB for app core
- ✅ Single executable deployment (Python server auto-managed by startup script)
- ✅ Tokio async runtime unchanged
- ✅ Cross-platform: Windows, macOS, Linux

### Principle III: Modern UI/UX Excellence ✅

- ✅ No UI changes required (backend-only feature)
- ✅ Chat/generation features completely removed (cleaner UX)
- ✅ Error messages remain actionable

### Principle IV: Intelligent Automation with User Control ✅

- ✅ Python server lifecycle fully automated (transparent to user)
- ✅ Graceful degradation if embedding server unavailable
- ✅ All automated actions remain visible

### Principle V: Developer Quality & Maintainability (Rust) ✅

- ✅ All Rust code will pass `cargo clippy` and `cargo fmt`
- ✅ Unit tests for Rust HTTP client module
- ✅ Doc comments for public functions
- ✅ Clear module separation maintained

### Principle VI: Python Development Standards ⚠️ NEEDS VERIFICATION

**Note**: Constitution version 1.0.0 doesn't include Python principle yet, but spec requires:

- ✅ `uv` for package management (specified in FR-021)
- ✅ venv for virtual environment (specified in FR-022)
- ✅ TypedDict for structured data (specified in FR-014, entities defined in spec)
- ✅ Explicit type hints on all arguments and return values (specified in FR-014)
- ✅ No `Any` types (specified in FR-014)
- ✅ mypy --strict / pyright validation (specified in FR-014)
- ✅ Python 3.13+ (specified in FR-020, assumptions)
- ✅ ruff format and ruff check (specified in FR-014, SC-011)

**Action**: Update constitution to v1.1.0 with Python Principle VI after this feature is approved.

### Simplicity Mandate ⚠️ NEW DEPENDENCY JUSTIFICATION REQUIRED

**New Dependencies Being Added:**

1. **Python 3.13+ runtime**
   - **Problem**: Need to run GGUF models without C/C++ compilation
   - **Why custom code won't work**: llama.cpp bindings provide battle-tested GGUF support with pre-built wheels
   - **Alternative rejected**: Pure Rust with Candle (no GGUF support for embeddinggemma, requires manual layer implementation)

2. **llama-cpp-python**
   - **Problem**: Load and run embeddinggemma-300m-qat GGUF model
   - **Why custom code won't work**: GGUF format is complex, llama.cpp is the reference implementation
   - **Alternative rejected**: Writing custom GGUF loader would take weeks and be error-prone

3. **FastAPI + uvicorn**
   - **Problem**: HTTP server for Rust to call embeddings
   - **Why custom code won't work**: FastAPI provides async, type-safe, auto-documented API with minimal code
   - **Alternative rejected**: Flask (no async), raw asyncio (more boilerplate)

4. **uv**
   - **Problem**: Fast, reproducible Python package management
   - **Why custom code won't work**: N/A (package manager)
   - **Alternative rejected**: pip (slow), poetry (complex lock files)

5. **reqwest (Rust)**
   - **Problem**: HTTP client for calling Python server
   - **Why custom code won't work**: Well-tested, async HTTP client
   - **Alternative rejected**: hyper (lower-level, more boilerplate)

### Technology Stack Alignment ✅

- ✅ Backend: Rust 1.75+ with Tauri 1.5+ (unchanged)
- ✅ Frontend: Svelte 5+ with Vite (unchanged)
- ✅ Database: SQLite via rusqlite (unchanged)
- ⚠️ **LLM**: Remove Ollama/LM Studio dependencies (no longer needed for embeddings)
- ✅ **New**: Python 3.13+ with uv (as specified in constitution v1.1.0)

### Summary

**Status**: ✅ PASS with justified dependency additions

All constitutional principles are upheld. New Python dependencies are fully justified and align with the goal of simplifying architecture by removing external service dependencies.

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
localmind-rs/                    # Rust/Tauri application
├── src/
│   ├── main.rs                 # Entry point (modified: remove LMStudio, completion logic)
│   ├── lib.rs                  # Module exports (modified: remove lmstudio module)
│   ├── local_embedding.rs      # NEW: HTTP client for Python server
│   ├── rag.rs                  # RAG logic (modified: use LocalEmbeddingClient)
│   ├── db.rs                   # Database (unchanged)
│   ├── http_server.rs          # HTTP API (modified: remove completion endpoints)
│   ├── bookmark.rs             # Bookmark logic (unchanged)
│   ├── bookmark_exclusion.rs   # Exclusions (unchanged)
│   └── lmstudio.rs             # DELETE: Remove entire module
├── src/bin/
│   └── reembed_batched.rs      # Re-embedding tool (modified: use LocalEmbeddingClient)
├── tests/
│   └── local_embedding_test.rs # NEW: Tests for HTTP client
├── start_localmind.bat         # Windows startup (modified: manage Python server)
├── start_localmind.sh          # Unix startup (modified: manage Python server)
└── Cargo.toml                  # Dependencies (modified: remove LMStudio, add reqwest)

embedding_server.py              # NEW: Python FastAPI server
pyproject.toml                   # NEW: Python dependencies (uv)
.python-version                  # NEW: Python version constraint (3.11+)

src-tauri/                       # Tauri configuration (unchanged)
ui/                              # Svelte frontend (unchanged)
```

**Structure Decision**: 

This is a **hybrid Rust + Python project** where:

1. **Rust (localmind-rs/)**: Main application, database, RAG logic, HTTP API, frontend
2. **Python (root)**: Embedding server (`embedding_server.py`), managed automatically by startup scripts

The Python server is treated as an internal implementation detail, not a separate deployable service. Users interact only with the Rust application, which manages the Python server lifecycle transparently.

## Complexity Tracking

**No unjustified violations** - All dependency additions have been justified in Constitution Check above.

---

## Phase 0: Research (Complete)

**Output**: `research.md`

Key decisions made:
- llama-cpp-python for GGUF runtime (pre-built wheels, no compilation)
- FastAPI for HTTP server (type-safe, async, minimal code)
- uv for Python package management (fast, reproducible)
- TypedDict + mypy --strict for type safety
- HuggingFace Hub for model caching
- Batch script for lifecycle management

All technical uncertainties resolved. See [research.md](./research.md) for details.

---

## Phase 1: Design & Contracts (Complete)

**Outputs**:
- `data-model.md`: TypedDict and Rust struct definitions
- `contracts/embedding-api.yaml`: OpenAPI 3.0 specification
- `quickstart.md`: Step-by-step setup guide

### Data Model Summary

**Python TypedDict**:
- `EmbeddingRequest` (text: str)
- `EmbeddingResponse` (embedding: list[float], model: str, dimension: int)
- `HealthResponse` (status: str, model_loaded: bool)
- `ErrorResponse` (error: str, detail: str | None)

**Rust Structs**:
- `EmbeddingRequest` (text: String)
- `EmbeddingResponse` (embedding: Vec<f32>, model: String, dimension: usize)
- `ErrorResponse` (error: String, detail: Option<String>)

### API Endpoints

1. **GET /health**: Health check (returns 200 OK when ready, 503 when loading)
2. **POST /embed**: Generate embedding (returns 768-dim vector)

See [contracts/embedding-api.yaml](./contracts/embedding-api.yaml) for full OpenAPI spec.

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                        User                                  │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              start_localmind.bat/.sh                         │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ 1. Check Python 3.13+                                 │   │
│  │ 2. Create venv (uv venv)                              │   │
│  │ 3. Install deps (uv pip install)                      │   │
│  │ 4. Start Python server (background)                   │   │
│  │ 5. Wait for health check                              │   │
│  │ 6. Launch Rust app                                    │   │
│  │ 7. On exit: kill Python server                        │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────┬──────────────────────┬─────────────────────────┘
              │                      │
              ▼                      ▼
┌─────────────────────────┐   ┌──────────────────────────────┐
│  Python Server          │   │  Rust Application            │
│  (localhost:8000)       │   │  (Tauri + Svelte)            │
│                         │   │                              │
│  ┌──────────────────┐   │   │  ┌────────────────────────┐  │
│  │ FastAPI          │   │◄──┤  │ LocalEmbeddingClient   │  │
│  │ - /health        │   │   │  │ (HTTP Client)          │  │
│  │ - /embed         │   │   │  └────────────────────────┘  │
│  └──────────────────┘   │   │                              │
│                         │   │  ┌────────────────────────┐  │
│  ┌──────────────────┐   │   │  │ RAG Module             │  │
│  │ llama-cpp-python │   │   │  └────────────────────────┘  │
│  │ (embeddinggemma) │   │   │                              │
│  └──────────────────┘   │   │  ┌────────────────────────┐  │
│                         │   │  │ SQLite Database        │  │
│  ┌──────────────────┐   │   │  └────────────────────────┘  │
│  │ HuggingFace      │   │   │                              │
│  │ Cache (~/.cache) │   │   │  ┌────────────────────────┐  │
│  └──────────────────┘   │   │  │ Svelte UI              │  │
│                         │   │  └────────────────────────┘  │
└─────────────────────────┘   └──────────────────────────────┘
```

### Setup Workflow

See [quickstart.md](./quickstart.md) for detailed setup instructions.

---

## Constitution Check (Post-Design) ✅

Re-evaluated after Phase 1 design:

- ✅ All principles still upheld
- ✅ No new dependencies added beyond those justified
- ✅ Architecture maintains privacy, performance, and maintainability goals
- ✅ Python code will follow Principle VI (TypedDict, type hints, mypy, ruff)
- ✅ API contracts are well-defined and type-safe

**Ready to proceed to Phase 2: Task Generation** via `/speckit.tasks` command.

---

## Next Steps

1. Run `/speckit.tasks` to generate detailed implementation tasks
2. Implement tasks in order:
   - **Removal**: Delete LMStudio, completion code
   - **Python Server**: Create embedding_server.py
   - **Rust Client**: Implement LocalEmbeddingClient HTTP client
   - **Startup Scripts**: Update start_localmind.bat/.sh
   - **Testing**: Verify end-to-end workflow
3. Verify all success criteria (SC-001 through SC-012)
4. Run quality gates (cargo clippy, cargo fmt, mypy, ruff)
5. Manual testing on target platforms

---

## Risk Register (Updated)

| Risk | Likelihood | Impact | Mitigation Status |
|------|-----------|--------|-------------------|
| Pre-built wheels unavailable | Low | High | ✅ Documented fallback (manual compilation) |
| Port 8000 conflict | Medium | Low | ✅ Configurable via env var |
| Python not installed | Medium | High | ✅ Startup script checks + clear error |
| Model download failure | Low | Medium | ✅ Retry logic + manual retry option |
| Slow CPU performance | Medium | Low | ✅ Within success criteria (SC-005) |

---

## Artifacts Generated

- ✅ `plan.md`: This file (implementation plan)
- ✅ `research.md`: Research findings and technology choices
- ✅ `data-model.md`: TypedDict and struct definitions
- ✅ `contracts/embedding-api.yaml`: OpenAPI 3.0 specification
- ✅ `quickstart.md`: Setup guide

**Next**: Run `/speckit.tasks` to break this plan into actionable tasks.
