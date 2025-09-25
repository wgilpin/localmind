# Technical Design Document - Go Single Binary

## 1. Overview

LocalMind will be refactored from the current multi-process Node.js/ChromaDB architecture to a single Go binary. This eliminates the complexity of managing separate processes while maintaining all core functionality.

## 2. Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│ LocalMind Go Binary (~15MB)                                 │
│                                                             │
│ ┌─────────────────┐  ┌──────────────────┐  ┌─────────────┐ │
│ │ Embedded Web UI │  │ HTTP API Server  │  │ Core Engine │ │
│ │ (Static Assets) │  │ (Gin/Chi)       │  │             │ │
│ └─────────────────┘  └──────────────────┘  └─────────────┘ │
│                                                             │
│ ┌─────────────────┐  ┌──────────────────┐  ┌─────────────┐ │
│ │ SQLite Database │  │ Vector Index     │  │ Ollama      │ │
│ │ + FTS5         │  │ (Pure Go)        │  │ Client      │ │
│ │                 │  │                  │  │             │ │
│ └─────────────────┘  └──────────────────┘  └─────────────┘ │
│                                                             │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ File System Watcher (Chrome Bookmarks)                 │ │
│ └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## 3. Technology Stack

### Core

- **Language**: Go 1.21+
- **Web Framework**: Chi (lightweight router)
- **Database**: SQLite via modernc.org/sqlite (pure Go, no CGO)
- **Vector Search**: Pure Go implementation (no extensions needed)
- **Embedding**: HTTP client to Ollama API
- **Build**: Single static binary via `go build` - no CGO required

### Vector Search Approach

- **In-memory index** for active vectors (fast search)
- **SQLite BLOBs** for persistent storage (msgpack encoded)
- **Pure Go cosine similarity** computation
- **Scale target**: 10k-100k documents (plenty for local use)

### Frontend

- **Existing Svelte UI**: Embedded as static assets using `embed.FS`
- **API**: RESTful JSON API (same endpoints as current implementation)
- **Serving**: Go's built-in HTTP server

## 4. Project Structure

```text
localmind-go/
├── main.go                 # Entry point
├── cmd/
│   └── server.go          # Server startup logic
├── internal/
│   ├── api/               # HTTP handlers
│   │   ├── handlers.go
│   │   ├── middleware.go
│   │   └── routes.go
│   ├── core/              # Business logic
│   │   ├── documents.go   # Document management
│   │   ├── search.go      # RAG pipeline
│   │   ├── embeddings.go  # Ollama integration
│   │   └── bookmarks.go   # Chrome bookmark monitoring
│   ├── storage/           # Data layer
│   │   ├── sqlite.go      # SQLite operations
│   │   ├── vector.go      # Vector operations
│   │   └── migrations.go  # Schema management
│   └── config/
│       └── config.go      # Configuration
├── web/                   # Embedded frontend assets
│   ├── static/
│   └── templates/
├── pkg/                   # Shared utilities
│   ├── ollama/           # Ollama client
│   ├── textprocessor/    # Text chunking, cleaning
│   └── watcher/          # File system monitoring
└── scripts/
    ├── build.sh          # Cross-platform builds
    └── package.sh        # Distribution packaging
```

## 5. Data Storage

### SQLite Schema

```sql
-- Documents table
CREATE TABLE documents (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    url TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Vector embeddings table
CREATE TABLE embeddings (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    chunk_text TEXT NOT NULL,
    embedding BLOB NOT NULL,  -- Msgpack encoded []float32
    chunk_index INTEGER NOT NULL,
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

-- Create index for faster document lookups
CREATE INDEX idx_embeddings_document_id ON embeddings(document_id);

-- Full-text search index
CREATE VIRTUAL TABLE documents_fts USING fts5(
    title, content, content='documents', content_rowid='rowid'
);
```

### Configuration

```yaml
# ~/.localmind/config.yaml
server:
  port: 3001
  host: localhost

ollama:
  api_url: "http://localhost:11434"
  embedding_model: "qwen3-embedding:0.6b"
  completion_model: "qwen2.5:0.5b"

storage:
  data_dir: "~/.localmind"
  db_file: "localmind.db"

search:
  chunk_size: 500
  chunk_overlap: 50
  max_results: 20
```

## 6. Core Components

### 6.1 Document Manager

```go
type DocumentManager struct {
    db     *sql.DB
    vector *VectorStore
    ollama *OllamaClient
}

func (dm *DocumentManager) AddDocument(doc Document) error {
    // 1. Store document in SQLite
    // 2. Chunk text content
    // 3. Generate embeddings via Ollama
    // 4. Store vectors in vector index
    // 5. Update FTS index
}
```

### 6.2 RAG Engine

```go
type RAGEngine struct {
    docMgr     *DocumentManager
    vectorStore *VectorStore
    ollama     *OllamaClient
}

func (re *RAGEngine) Search(query string) (*SearchResult, error) {
    // 1. Generate query embedding
    // 2. Perform vector similarity search
    // 3. Retrieve relevant documents
    // 4. Build context prompt
    // 5. Stream completion from Ollama
    // 6. Return results with citations
}
```

### 6.3 Vector Store

```go
type VectorStore struct {
    db       *sql.DB
    index    *VectorIndex  // In-memory for fast search
    mu       sync.RWMutex
}

type VectorIndex struct {
    vectors  map[string][]float32  // docID -> embedding (dimension varies by model)
    metadata map[string]ChunkMeta  // docID -> chunk info
    dimension int                  // embedding dimension (set at runtime)
}

func (vs *VectorStore) AddVector(docID string, chunk string, vector []float32) error {
    // 1. Store in SQLite (persistent)
    // 2. Add to in-memory index (fast search)
}

func (vs *VectorStore) Search(queryVector []float32, limit int) ([]VectorResult, error) {
    // Pure Go cosine similarity search
    results := make([]VectorResult, 0)
    for id, vec := range vs.index.vectors {
        score := cosineSimilarity(queryVector, vec)
        results = append(results, VectorResult{ID: id, Score: score})
    }
    // Sort by score and return top-k
}

func (vs *VectorStore) LoadIndex() error {
    // Load all vectors from SQLite into memory on startup
}
```

### 6.4 Ollama Client

```go
type OllamaClient struct {
    baseURL string
    client  *http.Client
}

func (oc *OllamaClient) GenerateEmbedding(text string) ([]float32, error)
func (oc *OllamaClient) StreamCompletion(prompt string) (<-chan string, error)
func (oc *OllamaClient) ListModels() ([]Model, error)
```

## 7. API Endpoints

Maintain compatibility with existing Chrome extension:

```
GET  /                          # Serve web UI
POST /api/documents             # Add document/note
GET  /api/documents/:id         # Get specific document
PUT  /api/documents/:id         # Update document
DELETE /api/documents/:id       # Delete document
GET  /api/search-stream/:query  # Streaming search (SSE)
POST /api/log-result-click      # Analytics logging
GET  /api/models               # List Ollama models
POST /api/models               # Set completion model
GET  /api/recent-notes         # Recent documents
```

## 8. Build & Distribution

### Build Script

```bash
#!/bin/bash
# scripts/build.sh

# Embed frontend assets
go generate ./...

# Cross-platform builds
GOOS=windows GOARCH=amd64 go build -ldflags="-s -w" -o dist/localmind-windows.exe
GOOS=darwin GOARCH=amd64 go build -ldflags="-s -w" -o dist/localmind-darwin-amd64
GOOS=darwin GOARCH=arm64 go build -ldflags="-s -w" -o dist/localmind-darwin-arm64
GOOS=linux GOARCH=amd64 go build -ldflags="-s -w" -o dist/localmind-linux
```

### Asset Embedding

```go
//go:embed web/static/*
var staticFiles embed.FS

//go:embed web/templates/*
var templates embed.FS
```

## 9. Migration Strategy

### Phase 1: Core Implementation

1. Set up Go project structure
2. Implement SQLite storage layer
3. Build Ollama client
4. Create basic HTTP API
5. Embed existing Svelte UI

### Phase 2: Feature Parity

1. Port RAG pipeline logic
2. Implement vector search
3. Add bookmark monitoring
4. Test with existing Chrome extension

### Phase 3: Optimization

1. Performance tuning
2. Memory optimization
3. Cross-platform testing
4. Distribution packaging

## 10. Advantages of This Architecture

### Simplicity

- Single binary deployment
- No process management
- No port conflicts
- One configuration file

### Performance

- Native Go performance
- SQLite for fast local queries
- Embedded assets (no file I/O for UI)
- Efficient vector operations

### Reliability

- No inter-service communication failures
- Atomic transactions
- Simple error handling
- Easy debugging

### Distribution

- ~15MB single executable
- No runtime dependencies
- Works offline immediately
- Simple installation

## 11. Dependencies

### Core Dependencies

```go
// Database (pure Go, no CGO)
modernc.org/sqlite

// Web framework
github.com/go-chi/chi/v5

// Configuration
gopkg.in/yaml.v3

// File watching
github.com/fsnotify/fsnotify

// Serialization for vectors
github.com/vmihailenco/msgpack/v5

// Text processing
github.com/kljensen/snowball
```

### Build Dependencies

- Go 1.21+ with embed support
- No CGO required
- No external C libraries
- Cross-compilation works out of the box

## 12. Future Enhancements

- **Mobile sync**: Add simple HTTP endpoints for mobile apps
- **Encryption**: Add AES-256 encryption for sensitive documents
- **Plugin system**: Go plugins for custom processors
- **Advanced search**: Hybrid vector + keyword search
- **Export/import**: Backup and restore functionality

This architecture maintains all existing functionality while dramatically simplifying deployment and operation.
