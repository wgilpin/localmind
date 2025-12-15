# Feature Specification: Headless LLM Migration

**Feature Branch**: `005-headless-llms`  
**Created**: 2025-01-21  
**Status**: Draft  
**Input**: User description: "Remove dependency on external LMStudio process. Reduce app scope to pure RAG indexing/search. Eliminate chat completion features."

## Clarifications

### Session 2025-01-27

- Q: How should the system handle existing embeddings that were generated using LMStudio when migrating to the new local embedding model? → A: The same embedding model (text-embedding-embeddinggemma-300m-qat) will be used, just loaded locally instead of via API, so existing embeddings remain fully compatible and no migration is needed.
- Q: How should the system respond when users attempt to access removed chat/generation features? → A: All chat/generation UI elements, API endpoints, and backend chat/completion logic and configuration must be completely removed (not just disabled or returning errors).
- Q: How should the system handle embedding generation requests while the model is still loading? → A: Allow requests but return a "loading" response that clients can retry.
- Q: How should the system handle model download failures during initial setup? → A: Retry with exponential backoff, with clear error messages and manual retry option.
- Q: How should the system handle text inputs that exceed the embedding model's context length limit? → A: The system uses an existing document chunking strategy (default: 500 characters per chunk with 50 character overlap). Documents are chunked before embedding, each chunk is embedded individually and stored, and matched embeddings return the original source document. Individual inputs to the embedding model are already chunk-sized (max 500 chars) and should be well within model limits. If a chunk itself exceeds model limits, the system should handle this gracefully (reject with error or further sub-chunk).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Simplified System Startup (Priority: P1)

Users can start the application without requiring any external processes to be running. The system initializes embedding capabilities automatically using locally-managed models, eliminating the need to manually start LMStudio or verify external service availability.

**Why this priority**: This is the core value proposition - removing external dependencies simplifies deployment and improves reliability. Users no longer need to manage multiple processes or troubleshoot connection issues.

**Independent Test**: Can be fully tested by starting the application and verifying that embedding generation works without any external services running. The system should automatically download and cache the required model on first use, then use it for all embedding operations.

**Acceptance Scenarios**:

1. **Given** the application is not running and no external services are available, **When** a user starts the application, **Then** the system initializes successfully and is ready to generate embeddings without user intervention
2. **Given** the application is starting for the first time, **When** it needs to generate embeddings, **Then** it automatically downloads and caches the required model, then proceeds with embedding generation
3. **Given** the application has previously cached the model, **When** it starts, **Then** it loads the cached model and is immediately ready for embedding operations

---

### User Story 2 - Pure RAG Indexing and Search (Priority: P1)

Users can index documents and search through them using semantic search capabilities. The system focuses exclusively on these core RAG operations, with all chat completion and text generation features removed.

**Why this priority**: This defines the reduced scope of the application. Users should be able to perform document indexing and semantic search, which are the essential RAG operations, without any chat or generation features.

**Independent Test**: Can be fully tested by indexing a set of documents and then performing semantic searches. The system should return relevant document chunks based on query similarity, without any chat or text generation capabilities being available.

**Acceptance Scenarios**:

1. **Given** documents are available for indexing, **When** a user requests document indexing, **Then** the system chunks documents, generates embeddings for each chunk, stores them, and uses matched embeddings to return original source documents
2. **Given** documents have been indexed, **When** a user performs a semantic search query, **Then** the system returns relevant document chunks ranked by similarity
3. **Given** a user attempts to use chat or text generation features, **When** they try to access these features, **Then** the features are completely absent (no UI elements, no API endpoints, no backend logic)

---

### User Story 3 - Local Embedding Generation (Priority: P1)

Users benefit from embedding generation that occurs entirely on their local machine, with no requests to external services. The system automatically manages a local Python embedding server that runs transparently in the background.

**Why this priority**: This is the technical foundation that enables the simplified architecture. Local embedding generation eliminates external service dependencies, network failures to remote services, and manual server management.

**Independent Test**: Can be fully tested by generating embeddings for text inputs and verifying that no network requests are made to external/remote services. The Python server lifecycle (startup, health checks, shutdown) should be completely transparent to the user.

**Acceptance Scenarios**:

1. **Given** the application starts, **When** the startup script executes, **Then** a Python embedding server is automatically started on localhost, becomes ready for requests, and embeddings are produced using local computation only
2. **Given** the Python server is running, **When** embedding generation is requested for a text input, **Then** the Rust application sends the request to localhost:8000, receives the embedding, and the entire operation completes based on local inference speed
3. **Given** the application is closed, **When** the user exits, **Then** the Python server is automatically terminated and all resources are cleaned up

---

### Edge Cases

- What happens when the model download fails due to network issues during initial setup? (System retries with exponential backoff, provides clear error messages, and offers manual retry option)
- How does the system handle insufficient memory to load the embedding model?
- What happens when a user attempts to access removed chat/generation features? (All UI elements, API endpoints, and backend logic are completely removed, so access is not possible)
- How does the system behave if the cached model files become corrupted?
- What happens when embedding generation is requested while the model is still loading? (Python server returns a "loading" response that Rust client can retry)
- How does the system handle very large text inputs that exceed model context limits? (Handled by existing document chunking strategy - documents are chunked to ~500 characters before embedding, so individual inputs to the embedding model are already within limits. If a chunk itself exceeds model limits, system should handle gracefully)
- What happens if Python is not installed or version is <3.13? (Startup script checks Python version and provides clear error message with installation instructions)
- What happens if port 8000 is already in use? (Startup script detects port conflict and provides clear error message)
- What happens if the Python server crashes during operation? (Rust client receives connection error and provides actionable error message to user)
- How does cleanup work when the user closes LocalMind? (Startup script tracks Python server process and terminates it on exit)

## Requirements *(mandatory)*

### Functional Requirements

#### Removal of External Dependencies

- **FR-001**: System MUST remove all HTTP client logic that connects to LMStudio API endpoints
- **FR-002**: System MUST remove all chat completion and text generation features, including streaming completion capabilities, API endpoints, backend logic, and configuration
- **FR-003**: System MUST completely remove all UI elements, API endpoints, and backend chat/completion logic and configuration related to chat or generation functionality
- **FR-004**: System MUST remove all process checks, connection tests, and warnings related to LMStudio service availability
- **FR-005**: System MUST remove all data structures and code related to Llama 3.1 completion models and generation logic

#### Python Embedding Server

- **FR-006**: System MUST provide a Python FastAPI server that exposes `/embed` endpoint accepting text and returning 768-dimensional vectors
- **FR-007**: Python server MUST use `llama-cpp-python` to load `ggml-org/embeddinggemma-300m-qat-q8_0-GGUF/embeddinggemma-300m-qat-Q8_0.gguf` model
- **FR-008**: Python server MUST download model automatically via `hf-hub-download` on first run, with retry logic using exponential backoff and clear error messages on failure
- **FR-009**: Python server MUST cache model in Hugging Face cache directory (`~/.cache/huggingface/` or Windows equivalent)
- **FR-010**: Python server MUST respond with health check on `/health` endpoint
- **FR-011**: Python server MUST log startup, model loading, and request handling to console
- **FR-012**: Python server MUST run on localhost:8000 (configurable via `EMBEDDING_SERVER_PORT` environment variable)
- **FR-013**: Python server MUST return a "loading" response for embedding requests received while the model is still loading
- **FR-014**: All Python code MUST follow Constitution Principle VI (TypedDict for structured data, explicit type hints on all function arguments and return values, no `Any` types, validated with `mypy --strict` or `pyright`, formatted with `ruff format`, passing `ruff check`)

#### Rust Client

- **FR-015**: Rust `LocalEmbeddingClient` MUST be refactored to HTTP client calling Python server at localhost:8000
- **FR-016**: Rust client MUST send POST requests to `/embed` endpoint with JSON body containing text
- **FR-017**: Rust client MUST deserialize JSON response to `Vec<f32>` (768-dimensional vector)
- **FR-018**: Rust client MUST handle "loading" responses from server by retrying with backoff
- **FR-019**: Rust client MUST provide clear error messages when Python server is unavailable

#### Startup Script & Lifecycle Management

- **FR-020**: Startup script (`start_localmind.bat`) MUST check for Python 3.13+ installation before proceeding
- **FR-021**: Startup script MUST use `uv` for all Python package management and virtual environment creation
- **FR-022**: Startup script MUST create Python virtual environment if it doesn't exist
- **FR-023**: Startup script MUST install dependencies from `pyproject.toml` using `uv pip install`
- **FR-024**: Startup script MUST start Python server in background and wait for health check on `/health` endpoint
- **FR-025**: Startup script MUST launch Rust application after Python server is ready
- **FR-026**: Startup script MUST track Python server process ID and terminate it when user exits LocalMind
- **FR-027**: Startup script MUST detect and report port conflicts on port 8000
- **FR-028**: Startup script MUST detect and report Python version incompatibilities

#### Core Functionality

- **FR-029**: System MUST generate embeddings by processing text chunks (from existing chunking strategy, ~500 characters per chunk) through the Python server and receiving normalized vector representations
- **FR-030**: System MUST NOT exceed 1.2GB total memory usage (1GB for embedding model + 200MB overhead)
- **FR-031**: System MUST eliminate all network requests to external/remote services for embedding generation, with latency bounded only by local inference time and localhost communication
- **FR-032**: System MUST preserve all existing RAG indexing and search functionality
- **FR-033**: System MUST maintain compatibility with existing indexed documents and vector stores (768-dimensional embeddings)

### Key Entities *(include if feature involves data)*

- **Embedding Model**: The GGUF-format neural network model (embeddinggemma-300m-qat) used for generating vector embeddings from text. The same model previously used via LMStudio API will now be loaded by the Python server using llama-cpp-python, stored in Hugging Face cache, and executed locally.
- **Embedding Vector**: A 768-dimensional normalized vector representation of text content, used for semantic similarity calculations in search operations.
- **Model Cache**: Local storage location for downloaded model files in Hugging Face cache directory (`~/.cache/huggingface/` or Windows equivalent).
- **EmbeddingRequest**: TypedDict structure with `text: str` field, sent from Rust client to Python server.
- **EmbeddingResponse**: TypedDict structure with `embedding: list[float]`, `model: str`, and `dimension: int` fields, returned from Python server to Rust client.
- **HealthResponse**: TypedDict structure with `status: str` and `model_loaded: bool` fields, used for server health checks.
- **Python Embedding Server**: FastAPI application running on localhost:8000, managing model lifecycle and serving embedding requests.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Application starts successfully without requiring any manually-managed external services, with 100% of startup attempts completing without external dependency errors (Python server lifecycle is managed automatically by startup script)
- **SC-002**: Embedding generation completes with zero network requests to external/remote services, with all requests staying on localhost, verified through network monitoring
- **SC-003**: All chat completion and generation features are completely removed, with zero references to these capabilities in user-facing interfaces, APIs, backend code, configuration, or documentation
- **SC-004**: Model download and caching completes successfully on first use (within 10 minutes on typical broadband), with cached models loading on subsequent starts within 5 seconds
- **SC-005**: Embedding generation completes in <1 second per chunk (90th percentile), with latency bounded by local inference time and minimal localhost communication overhead
- **SC-006**: Memory usage remains within specified bounds (max 1.2GB: 1GB model + 200MB overhead) during normal operation
- **SC-007**: All existing RAG indexing and search operations continue to function correctly, with 100% compatibility with previously indexed documents (768-dimensional embeddings)
- **SC-008**: Re-embedding 1000 chunks completes in <10 minutes
- **SC-009**: System handles model loading errors gracefully, retrying downloads with exponential backoff, providing clear error messages, and offering manual retry options when model download or loading fails
- **SC-010**: Python server startup completes successfully within 30 seconds after venv/dependencies are installed
- **SC-011**: Python code passes `ruff check`, `ruff format --check`, and `mypy --strict` with zero errors
- **SC-012**: Zero C/C++ compilation required (llama-cpp-python uses pre-built wheels)

## Assumptions

- Users have Python 3.13+ installed or can install it following provided instructions
- Users have sufficient disk space for model caching (~300MB for embeddinggemma-300m-qat GGUF model)
- Users have sufficient RAM to load embedding model into memory (approximately 1GB for the model)
- Internet connectivity is available for initial model download and Python package installation, but not required for subsequent operation
- The target embedding model (embeddinggemma-300m-qat GGUF) is available from Hugging Face Hub and produces 768-dimensional embeddings compatible with existing indexed documents
- Users do not require chat completion or text generation features for their use cases
- The application will focus exclusively on RAG indexing and search operations going forward
- Port 8000 is available for the Python embedding server (or can be made available by stopping conflicting processes)
- Pre-built `llama-cpp-python` wheels are available for the user's platform, avoiding C/C++ compilation

## Dependencies

### Removed
- LMStudio HTTP client dependencies
- `embed_anything` Rust crate and all Candle ML dependencies
- All completion/generation code paths

### Added
- Python 3.13+ runtime environment
- `uv` for Python package management
- Python FastAPI server with dependencies:
  - `fastapi>=0.115.0`
  - `uvicorn[standard]>=0.32.0`
  - `llama-cpp-python>=0.3.0` (pre-built wheels, no C/C++ compilation)
  - `huggingface-hub>=0.26.0`
- Rust HTTP client (`reqwest`) for calling Python embedding server

## Out of Scope

- Chat completion or text generation features (explicitly removed)
- Support for external/remote embedding services or APIs
- Model fine-tuning or training capabilities
- Support for multiple embedding models simultaneously
- Model version management or automatic updates
- GPU acceleration (future enhancement)
- Batch embedding endpoint (future enhancement)
- Distributed deployment of Python server (localhost only)
- Python server authentication or multi-user support (single-user localhost only)
