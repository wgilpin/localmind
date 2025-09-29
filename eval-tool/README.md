# Embedding Model Evaluation Tool

A command-line tool for evaluating and comparing different embedding models (both Ollama and SentenceTransformers) for semantic search on Chrome bookmarks.

## Features

- Samples bookmarks from Chrome's bookmarks file
- **Automatic exclusion filtering**: Respects LocalMind exclude list for consistent filtering
- Fetches actual page content for bookmarks
- Generates search queries using Ollama LLM (qwen3:4b)
- **Supports multiple embedding models**: Both Ollama and SentenceTransformers models
- **Model comparison**: Automatically appends results to comparison CSV
- **Incremental saves**: Saves embeddings every 10 documents to prevent data loss
- **Resume capability**: Automatically resumes from where it left off if interrupted
- Measures search performance metrics (recall, MRR, rank distribution)
- Windows-compatible file operations

## Installation

```bash
uv pip install -r requirements.txt
```

## Quick Start

```bash
# Run the complete evaluation pipeline (200 samples)
python main.py run-all --sample-size 200

# Evaluate with Ollama embedding model
python main.py evaluate --embedding-model nomic-embed-text --ollama

# Evaluate with SentenceTransformers model
python main.py evaluate --embedding-model all-MiniLM-L6-v2

# Compare multiple models
python main.py evaluate --embedding-model qwen3-embedding:0.6b --ollama
python main.py evaluate --embedding-model mxbai-embed-large --ollama
python main.py evaluate --embedding-model all-mpnet-base-v2
# Results automatically append to results/model_comparison.csv
```

## Usage

### Main Commands

**Complete Pipeline:**
```bash
python main.py run-all --sample-size 200 [--model qwen3:4b] [--reset]
```

**Sample and Generate Only:**
```bash
python main.py sample-and-generate --sample-size 200 [--model qwen3:4b] [--reset]
```

**Evaluate with Different Embedding Models:**
```bash
# Ollama models (requires --ollama flag)
python main.py evaluate --embedding-model nomic-embed-text --ollama
python main.py evaluate --embedding-model mxbai-embed-large --ollama
python main.py evaluate --embedding-model qwen3-embedding:0.6b --ollama

# SentenceTransformers models (no --ollama flag)
python main.py evaluate --embedding-model all-MiniLM-L6-v2
python main.py evaluate --embedding-model all-mpnet-base-v2
python main.py evaluate --embedding-model sentence-transformers/all-MiniLM-L12-v2
```

**Reset All Data:**
```bash
python main.py reset
```

**Analyze Results:**
```bash
python main.py analyze [--results results/evaluation_results.json]
```

### Advanced Commands

**Sample bookmarks only:**
```bash
python main.py sample --sample-size 200 [--fetch-content/--no-fetch-content]
```

**Generate queries only:**
```bash
python main.py generate [--samples data/sampled_bookmarks.json] [--model qwen3:4b]
```

**Evaluate only:**
```bash
python main.py evaluate [--samples data/sampled_bookmarks.json] [--queries data/generated_queries.json] [--embedding-model MODEL] [--ollama] [--top-k 20]
```

## Key Features

### Model Comparison
- Results from all models append to `results/model_comparison.csv`
- Single CSV row per model evaluation for easy comparison
- Includes all metrics: recall@k, MRR, mean rank, similarity scores
- Compare Ollama vs SentenceTransformers models directly

### Incremental Processing & Recovery
- **Incremental saves**: Embeddings saved every 10 documents during generation
- **Automatic resume**: If interrupted, automatically skips already processed bookmarks
- **Timeout handling**: 120-second timeout for slow Ollama models
- **Fallback storage**: Uses Pickle if Parquet (pyarrow) unavailable

### Smart Sample Management
- If you have 150 samples and want 200, it adds exactly 50 more
- Skips already processed bookmarks
- Handles failed web requests gracefully

### Windows Compatible
- Handles Windows file permission issues
- Uses safe atomic file operations with fallback
- Supports both forward and backslash paths

## Output Files

- `data/sampled_bookmarks.json` - Sampled bookmarks with content
- `data/generated_queries.json` - Generated search queries
- `results/evaluation_results.json` - Detailed evaluation results
- `results/evaluation_summary.csv` - CSV summary of individual query results
- `results/model_comparison.csv` - **Single-row-per-model comparison CSV**
- `vector_store_eval/` - Vector storage directory (Parquet or Pickle files)

## Metrics

- **Recall@K**: Percentage of queries that found their target bookmark in top K results
- **Mean Reciprocal Rank (MRR)**: Average of 1/rank for each query
- **Mean/Median Rank**: Average and median position of correct results
- **Distance**: Cosine distance between query and document embeddings
- **Similarity**: Cosine similarity scores

## Configuration

- **Default Embedding Model**: `all-MiniLM-L6-v2` (SentenceTransformers)
- **Ollama Embedding Models**: Any model available in Ollama (use `--ollama` flag)
- **Query Generation Model**: `qwen3:4b` (via Ollama)
- **Sample Size**: 200 bookmarks (default)
- **Top-K Results**: 20 (default)
- **Ollama Timeout**: 120 seconds per embedding
- **Incremental Save Frequency**: Every 10 embeddings
- **Exclude List**: Automatically loaded from LocalMind config (`~/.localmind/config.json`) or uses defaults

### Exclude List

The tool automatically filters out bookmarks that match patterns in the LocalMind exclude list:
- Default excludes: `node_modules`, `.git`, `build`, `dist`, `coverage`, `tmp`, etc.
- Loads from LocalMind configuration if available
- Filters based on both URL paths and bookmark titles
- Provides clear logging of excluded bookmarks

## Examples

### Compare Multiple Embedding Models
```bash
# Test Ollama models
python main.py evaluate --embedding-model nomic-embed-text --ollama
python main.py evaluate --embedding-model mxbai-embed-large --ollama
python main.py evaluate --embedding-model qwen3-embedding:0.6b --ollama

# Test SentenceTransformers models
python main.py evaluate --embedding-model all-MiniLM-L6-v2
python main.py evaluate --embedding-model all-mpnet-base-v2

# View comparison results
cat results/model_comparison.csv
```

### Incremental Workflow
```bash
# Start with 50 samples
python main.py run-all --sample-size 50

# Later, expand to 200 (adds 150 more)
python main.py run-all --sample-size 200

# If interrupted during evaluation, just re-run (auto-resumes)
python main.py evaluate --embedding-model nomic-embed-text --ollama
```

### Reset and Start Fresh
```bash
# Clear all data and start over
python main.py reset
python main.py run-all --sample-size 100 --model qwen3:4b
```

## Troubleshooting

### Ollama Timeout Errors
- Default timeout is 120 seconds per embedding
- For very slow models, modify timeout in `ollama_embedding.py`

### Missing Dependencies
- If Parquet fails, install: `pip install pyarrow`
- Tool automatically falls back to Pickle format if needed

### Resume After Interruption
- Just re-run the same command - it automatically resumes
- Check `vector_store_eval/` directory for saved embeddings

## Architecture Changes

### Previous (ChromaDB-based)
- Used ChromaDB for vector storage
- No incremental saves
- Lost data on interruption

### Current (NumPy/Pandas-based)
- Pure NumPy arrays for embeddings
- Pandas DataFrames for metadata
- Scikit-learn for cosine similarity
- Parquet/Pickle for persistence
- Incremental saves prevent data loss
- Model comparison CSV for analysis