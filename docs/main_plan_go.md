# LocalMind Go Implementation Plan

## Overview

Rebuild LocalMind as a single Go binary with embedded UI, pure Go SQLite, and in-memory vector search. Target: 10-15MB executable with zero runtime dependencies.

## Phase 1: Core Foundation

### 1.1 Project Setup

- Initialize Go module `github.com/wgilp/localmind-go`
- Set up project structure per TDD
- Configure modernc.org/sqlite (pure Go, no CGO)
- Set up Chi router and basic HTTP server

### 1.2 Database Layer

- Implement SQLite schema (documents, embeddings, FTS5)
- Create migration system for schema updates
- Build database service with CRUD operations
- Add msgpack serialization for vector storage

### 1.3 Ollama Client

- HTTP client for Ollama API
- Embedding generation with `qwen2-embedding:0.6b` (lower dimensionality for RAM efficiency)
- Streaming completion with `qwen2.5:0.5b`
- Model listing and configuration

## Phase 2: Vector Search & RAG

### 2.1 Vector Store

- Pure Go cosine similarity implementation
- In-memory index structure (map-based for simplicity)
- Load/save vectors from SQLite BLOBs
- Benchmark: Ensure <50ms search for 10k vectors

### 2.2 RAG Pipeline

- Text chunking (500 chars with 50 char overlap)
- Document ingestion pipeline
- Query embedding and similarity search
- Context assembly and prompt formatting
- Streaming response generation

### 2.3 Testing

- Unit tests for vector operations
- Integration tests for RAG pipeline
- Performance benchmarks

## Phase 3: API & Frontend

### 3.1 REST API

- Port existing API endpoints from Node.js
- SSE for streaming search results
- Maintain Chrome extension compatibility
- Add request logging and error handling

### 3.2 Frontend Integration

- Copy Svelte build output to `web/static`
- Embed using `embed.FS`
- Serve at root path
- Test all UI functionality

### 3.3 Chrome Integration

- Test existing Chrome extension
- Update manifest for port 3001
- Verify bookmark monitoring
- YouTube transcript extraction

## Phase 4: Features & Polish

### 4.1 Bookmark Monitor

- File system watcher for Chrome bookmarks
- Auto-indexing new bookmarks
- Duplicate detection

### 4.2 Configuration

- YAML config file support
- Environment variable overrides
- First-run setup wizard

### 4.3 Distribution

- Cross-platform build scripts
- Windows: `.exe` with installer
- macOS: Universal binary (Intel + ARM)
- Linux: Static binary + AppImage

## Success Metrics

### Performance

- Startup time: <2 seconds
- Search latency: <100ms for 10k documents
- Memory usage: <500MB with 10k documents
- Binary size: <15MB

### Functionality

- Feature parity with current Node.js version
- All tests passing
- Chrome extension fully functional
- No CGO dependencies

## Risk Mitigation

### Vector Search Performance

**Risk**: Pure Go might be too slow for large document sets
**Mitigation**:

- Start with simple map-based index
- Profile and optimize hot paths
- Consider HNSW library if needed (still pure Go)

### Embedding Model Size

**Risk**: `qwen2-embedding:0.6b` produces large embeddings
**Mitigation**:

- Measure actual dimension on first run
- Use dimension-aware serialization
- Consider quantization if memory becomes issue

### Cross-Platform Issues

**Risk**: File paths, Chrome bookmark locations vary by OS
**Mitigation**:

- Use filepath.Join for all paths
- OS-specific bookmark detection
- Extensive testing on all platforms

## Migration Path

1. **Data Migration Tool**
   - Export from ChromaDB to JSON
   - Import into new SQLite schema
   - Re-generate embeddings if model changed

2. **Parallel Running**
   - Run on different port initially (3002)
   - Test thoroughly before switching
   - Keep old system as backup

3. **Cutover**
   - Stop old Node.js daemon
   - Update Chrome extension settings
   - Start new Go binary on port 3001

## Development Priorities

**Must Have**:

- Core RAG functionality
- Chrome extension support
- Existing UI working
- Cross-platform builds

**Nice to Have**:

- Progress bars for indexing
- Backup/restore functionality
- Multiple collection support
- Advanced search operators

**Future**:

- Mobile sync capability
- Plugin system
- Multi-user support
- Cloud backup option

## Estimated Timeline

- **Week 1**: Core foundation complete, basic CRUD working
- **Week 2**: RAG pipeline functional, search working
- **Week 3**: Full API compatibility, UI integrated
- **Week 4**: Polish, testing, distribution ready

Total: **4 weeks to production-ready single binary**
