# LocalMind Model Setup Guide

This guide helps you set up the required AI models for LocalMind using LM Studio.

## Quick Start

Run the automated startup script:

```bash
./start_lmstudio.sh
```

This script will:
1. Check if LM Studio and the `lms` CLI are installed
2. Start LM Studio if not running
3. Verify required models are downloaded
4. Load models into memory
5. Launch LocalMind

## Required Models

### Embedding Model
**Name:** nomic-embed-text-v1.5
**Repository:** nomic-ai/nomic-embed-text-v1.5-GGUF
**Purpose:** Converts text into vector embeddings for semantic search

### Completion Model (Recommended)
**Name:** Llama 3.1 8B Instruct
**Repository:** lmstudio-community/Meta-Llama-3.1-8B-Instruct-GGUF
**Purpose:** Generates responses to queries using RAG context

## Manual Setup

### 1. Install LM Studio

Download and install from: https://lmstudio.ai/

Run LM Studio at least once to initialize the `lms` CLI tool.

### 2. Download Models

#### Using LM Studio GUI:
1. Open LM Studio
2. Click on "Discover" tab
3. Search for "nomic-embed-text"
4. Download "nomic-ai/nomic-embed-text-v1.5-GGUF"
5. Search for "Llama 3.1 8B Instruct"
6. Download "lmstudio-community/Meta-Llama-3.1-8B-Instruct-GGUF"

#### Using lms CLI:
```bash
# Check downloaded models
lms ls

# Models will show in the list after downloading via GUI
```

### 3. Load Models

#### Using LM Studio GUI:
1. Click on "Local Server" tab
2. Click "Start Server"
3. Select your embedding model
4. Load the model

#### Using lms CLI:
```bash
# Load embedding model with GPU acceleration
lms load nomic-ai/nomic-embed-text-v1.5-GGUF --gpu=max

# Check loaded models
lms ps
```

### 4. Start LocalMind

```bash
cargo tauri dev
```

Or use the automated script:
```bash
./start_lmstudio.sh
```

## Using lms CLI

### List Downloaded Models
```bash
lms ls
```

### List Currently Loaded Models
```bash
lms ps
```

### Load a Model
```bash
lms load <model-name> --gpu=max
```

Options:
- `--gpu=max`: Use maximum GPU acceleration
- `--gpu=auto`: Automatically determine GPU usage
- `--gpu=0.5`: Use 50% GPU (adjust 0.0-1.0)
- `--context-length=N`: Set context window size

### Unload Models
```bash
# Unload specific model
lms unload <model-name>

# Unload all models
lms unload --all
```

## Alternative Models

### Embedding Models
- **nomic-embed-text-v1.5** (Recommended, 137M parameters)
- mxbai-embed-large
- all-MiniLM-L6-v2 (smaller, faster)

### Completion Models
- **Llama 3.1 8B Instruct** (Recommended, good balance)
- Llama 3.2 3B (lighter, faster)
- Mistral 7B Instruct
- Phi-3 Mini (very light)

## Troubleshooting

### "lms command not found"
- Make sure LM Studio is installed
- Run LM Studio at least once to initialize the CLI
- On Windows: Add LM Studio to PATH or use from Git Bash

### "Cannot connect to LM Studio"
- Ensure LM Studio local server is started
- Check server is running on http://localhost:1234
- Verify in LM Studio → Local Server tab

### "Model not found"
- Download the model through LM Studio GUI
- Run `lms ls` to verify it's in your model directory
- Check "My Models" in LM Studio

### Poor Performance
- Use `--gpu=max` when loading models
- Choose smaller models if RAM/VRAM is limited
- Try Llama 3.2 3B or Phi-3 Mini for completion

## Configuration

Edit LocalMind settings to configure:
- Embedding model name
- Completion model name
- LM Studio server URL (default: http://localhost:1234)

## Next Steps

After setup:
1. Add documents to LocalMind
2. Documents will be automatically embedded using your embedding model
3. Query your documents using natural language
4. Get AI-powered answers using your completion model

For more information, see the main README.md
