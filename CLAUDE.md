# CLAUDE.md - LocalMind Project

This file provides guidance for working with the LocalMind project structure.

## Project Overview

LocalMind is a privacy-focused knowledge management system that allows users to store and intelligently search notes and bookmarks locally using RAG (Retrieval-Augmented Generation) with Ollama. All data processing happens locally - no data ever leaves the user's device unencrypted.

## Repository Structure

This repository contains multiple implementations of LocalMind:

```
localmind/
├── desktop-daemon/          # Node.js/TypeScript implementation (legacy)
│   ├── CLAUDE.md           # TypeScript-specific guidance
│   ├── src/                # TypeScript source code
│   ├── frontend/           # SvelteKit web application
│   └── package.json        # Node.js dependencies
├── localmind-go/           # Go implementation (current development)
│   ├── CLAUDE.md           # Go-specific guidance
│   ├── cmd/localmind/      # Main application
│   ├── internal/           # Go packages
│   └── go.mod              # Go dependencies
├── chrome-extension/        # Browser integration (shared)
├── docs/                   # Documentation and planning
│   └── main_plan_go.md     # Go implementation plan
└── CLAUDE.md               # This file - project overview
```

## Implementation Status

### TypeScript Implementation (Legacy) 🟡
- **Status**: Maintenance mode, fully functional
- **Location**: `desktop-daemon/`
- **Technology**: Node.js, TypeScript, ChromaDB, Better-SQLite3
- **Use Case**: Current production version, feature-complete
- **Documentation**: See `desktop-daemon/CLAUDE.md`

### Go Implementation (Active Development) 🟢  
- **Status**: Phase 1 complete, Phase 2 in progress
- **Location**: `localmind-go/`
- **Technology**: Pure Go, modernc.org/sqlite, embedded UI
- **Target**: Single 15MB binary with zero dependencies
- **Documentation**: See `localmind-go/CLAUDE.md`

### Chrome Extension (Shared) 🟢
- **Status**: Works with both implementations
- **Location**: `chrome-extension/`
- **Compatibility**: API-compatible with both versions
- **Port**: Configurable (3001 default)

## Development Workflow

### Working with TypeScript Version
```bash
cd desktop-daemon
npm install
npm run dev                 # Development server
```
See `desktop-daemon/CLAUDE.md` for detailed TypeScript-specific commands.

### Working with Go Version  
```bash
cd localmind-go
go mod tidy
go run ./cmd/localmind      # Development server
go build -o localmind.exe ./cmd/localmind  # Build binary
```
See `localmind-go/CLAUDE.md` for detailed Go-specific commands.

### Working with Chrome Extension
```bash
cd chrome-extension
# Load unpacked extension in Chrome developer mode
# Point to either localhost:3001 (default) or custom port
```

## Migration Path

The Go implementation is designed to be a drop-in replacement:

1. **API Compatibility**: 100% compatible REST API
2. **Data Migration**: Tools provided to migrate from SQLite + ChromaDB to pure SQLite
3. **Chrome Extension**: No changes required
4. **Configuration**: Similar config structure with Go-specific optimizations

## Which Implementation to Use?

### Use TypeScript Version When:
- Need immediate production stability
- Working with existing ChromaDB setup  
- Contributing to legacy features
- Familiar with Node.js ecosystem

### Use Go Version When:
- Want single binary deployment
- Need better performance/memory usage
- Contributing to future development
- Want zero external dependencies

## Configuration

Both implementations share the same data directory and basic configuration:

- **Data Directory**: `~/.localmind/`
- **Default Port**: `3001`
- **Ollama URL**: `http://localhost:11434`
- **Chrome Extension**: Compatible with both versions

## Development Commands (Project Root)

### Start Development Environment
```bash
# TypeScript version
./start_dev.sh              # Starts Ollama + ChromaDB + TypeScript backend

# Go version
cd localmind-go && go run ./cmd/localmind
```

### Build Production Versions
```bash
# TypeScript version  
./start_all.sh              # Build frontend + start production server

# Go version
cd localmind-go && go build -o localmind.exe ./cmd/localmind
```

### Run Tests
```bash
# TypeScript version
cd desktop-daemon && npm test

# Go version  
cd localmind-go && go test ./...
```

## Important Notes

- **Parallel Development**: Both versions can run simultaneously on different ports
- **Data Compatibility**: Migration tools handle schema differences
- **Chrome Extension**: Works with both versions via port configuration
- **Documentation**: Each implementation has its own detailed CLAUDE.md file
- **Future**: Go version is the target for new development and distribution

For implementation-specific details, see the respective CLAUDE.md files in each subdirectory.