# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LocalMind is a privacy-focused knowledge management system that allows users to store and intelligently search notes and bookmarks locally using RAG (Retrieval-Augmented Generation) with Ollama.

## Architecture

### Core Components

1. **Desktop Daemon** (`desktop-daemon/`) - Node.js/TypeScript backend server
   - Express API server handling CRUD operations
   - SQLite database for document storage
   - FAISS vector store for semantic search
   - Ollama integration for embeddings and LLM inference
   - Service-based architecture pattern

2. **Frontend** (`desktop-daemon/frontend/`) - SvelteKit web application
   - Static build served by the daemon
   - Search, note creation, and document management UI
   - Real-time status updates via Server-Sent Events (SSE)

3. **Chrome Extension** (`chrome-extension/`) - Browser integration
   - Bookmark capture and content extraction
   - Communication with desktop daemon API

### Key Services (in `desktop-daemon/src/services/`)

- **OllamaService**: Manages LLM and embedding model interactions
- **VectorStoreService**: Handles FAISS vector indexing and similarity search
- **DatabaseService**: SQLite operations for documents and vector mappings
- **RagService**: Orchestrates RAG pipeline for intelligent search
- **BookmarkMonitor**: Watches Chrome bookmarks for automatic indexing

## Development Commands

### Backend (from `desktop-daemon/`)
```bash
npm run dev      # Start development server with hot reload
npm run build    # Compile TypeScript to JavaScript
npm run test     # Run Jest tests
npm start        # Run production build
```

### Frontend (from `desktop-daemon/frontend/`)
```bash
npm run dev      # Start Svelte dev server
npm run build    # Build static frontend
npm run check    # Type-check Svelte components
```

### Full Application
```bash
./start_dev.sh   # Start Ollama + backend in dev mode (from root)
```

## Testing

Run tests from `desktop-daemon/`:
```bash
npm test                 # Run all tests
npm test -- <filename>   # Run specific test file
```

Test files follow `*.test.ts` naming convention alongside source files.

## Data Storage

- **Database**: `%APPDATA%/LocalMind/localmind.db` (Windows) or equivalent on other platforms
- **Vector Index**: Configured via `OllamaConfig.vectorIndexFile`
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
- Ollama models (embedding and language)
- Server port
- Data directory paths
- Vector index settings

## Dependencies

- **Ollama**: Must be installed and running for LLM functionality
- **Node.js**: v18+ required
- **TypeScript**: Primary language for backend
- **Better-SQLite3**: Database operations
- **Chroma DB**: Vector similarity search
- **LangChain**: Text processing utilities
- Before making major architectural changes, such as swapping one vectorDB provider for another, stop and ask the user to confirm
- the app is started by either of two scripts, @start_dev.sh for dev or @start_all.sh for prod build