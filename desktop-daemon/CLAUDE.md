# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LocalMind is a privacy-focused knowledge management system that allows users to store and intelligently search notes and bookmarks locally using RAG (Retrieval-Augmented Generation) with Ollama. All data processing happens locally - no data ever leaves the user's device unencrypted.

## Architecture

### Core Components

1. **Desktop Daemon** (`desktop-daemon/`) - Node.js/TypeScript backend server
   - Express API server handling CRUD operations
   - Better-SQLite3 database for document storage
   - ChromaDB vector store for semantic search (replacing FAISS)
   - Ollama integration for embeddings and LLM inference
   - Service-based architecture pattern

2. **Frontend** (`desktop-daemon/frontend/`) - SvelteKit web application
   - Static build served by the daemon at localhost:3001
   - Search, note creation, and document management UI
   - Real-time status updates via Server-Sent Events (SSE)

3. **Chrome Extension** (`chrome-extension/`) - Browser integration
   - One-click bookmark capture with full page content extraction
   - Auto-extraction of metadata (title, description, OpenGraph tags)
   - Communication with desktop daemon API

### Key Services (in `desktop-daemon/src/services/`)

- **ollama.ts**: Manages LLM and embedding model interactions with Ollama
- **chromaStore.ts**: Handles ChromaDB vector indexing and similarity search
- **database.ts**: Better-SQLite3 operations for documents and metadata
- **rag.ts**: Orchestrates RAG pipeline for intelligent search
- **bookmarkMonitor.ts**: Watches Chrome bookmarks for automatic indexing

### Fixed Model Configuration

- **Embedding Model**: `all-MiniLM-L6-v2` (384 dimensions) - NOT user-configurable to avoid expensive re-embedding
- **LLM Model**: `qwen3:0.6b` (configurable for completion)

## Development Commands

### Full Application (Recommended)
```bash
./start_dev.sh   # Start Ollama + ChromaDB + backend in dev mode (from root)
./start_all.sh   # Build and start production version (from root)
```

The `start_dev.sh` script handles:
- Ollama service startup with optimized environment variables
- ChromaDB server startup at localhost:8000 with persistent storage
- Frontend build and backend development server with hot reload

### Backend Only (from `desktop-daemon/`)
```bash
npm run dev      # Start development server with hot reload (requires ChromaDB running)
npm run build    # Compile TypeScript to JavaScript  
npm run test     # Run Jest tests
npm start        # Run production build
```

### Frontend Only (from `desktop-daemon/frontend/`)
```bash
npm run dev      # Start Svelte dev server (for frontend-only development)
npm run build    # Build static frontend
npm run check    # Type-check Svelte components
```

## Testing

Run tests from `desktop-daemon/`:
```bash
npm test                 # Run all tests
npm test -- <filename>   # Run specific test file
```

Test files follow `*.test.ts` naming convention alongside source files.

## Data Storage

- **Database**: `~/.localmind/localmind.db` (cross-platform via Better-SQLite3)
- **Vector Index**: ChromaDB at `~/.localmind/chromadb` (persistent storage)
- **Configuration**: `~/.localmind/config.json` (optional, uses defaults if not present)
- **Chrome Bookmarks**: Auto-monitored from default Chrome profile location

## API Endpoints

Key endpoints served on `http://localhost:3001`:
- `POST /api/documents` - Create/update documents  
- `GET /api/documents` - List all documents
- `POST /api/search` - Semantic search with RAG
- `GET /api/bookmarks` - Get Chrome bookmarks
- `GET /status-stream` - SSE endpoint for real-time status

## Configuration

Configuration loaded from `desktop-daemon/src/config.ts`:
- Ollama API URL (default: http://localhost:11434)
- Embedding model: `mahonzhan/all-MiniLM-L6-v2` (fixed, not user-configurable)  
- Completion model: `qwen3:0.6b` (configurable)
- Server port (default: 3000, can be overridden via PORT env var)
- Data directory paths (`~/.localmind/`)

## Dependencies & Environment

- **Ollama**: Must be installed and running for LLM functionality
- **ChromaDB**: Vector database server (auto-started by start_dev.sh)
- **Node.js**: v18+ required
- **Python**: Required for ChromaDB (pip install chromadb)
- **TypeScript**: Primary language for backend
- **Better-SQLite3**: Document and metadata storage
- **SvelteKit**: Frontend framework

## Important Development Notes

- Before making major architectural changes (e.g., swapping vector DB providers), confirm with user first
- The embedding model is intentionally fixed to avoid expensive re-embedding of the entire corpus
- Ollama environment is optimized with `OLLAMA_NUM_PARALLEL=2`, `OLLAMA_KEEP_ALIVE=-1`, `OLLAMA_MAX_LOADED_MODELS=2`
- ChromaDB runs as a separate server process at localhost:8000
- Use the startup scripts (`start_dev.sh` or `start_all.sh`) rather than running components individually