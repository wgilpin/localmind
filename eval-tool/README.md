# ChromaDB Bookmark Evaluation Tool

A command-line tool for evaluating ChromaDB search performance on Chrome bookmarks.

## Features

- Samples bookmarks from Chrome's bookmarks file
- **Automatic exclusion filtering**: Respects LocalMind exclude list for consistent filtering
- Fetches actual page content for bookmarks
- Generates search queries using Ollama LLM (qwen3:4b)
- Creates a separate ChromaDB instance for evaluation
- Measures search performance metrics (recall, MRR, rank distribution)
- Incremental processing with automatic resume capability
- Windows-compatible file operations

## Installation

```bash
uv pip install -r requirements.txt
```

## Quick Start

```bash
# Run the complete evaluation pipeline (200 samples)
python main.py run-all --sample-size 200

# Start fresh (delete all existing data)
python main.py run-all --sample-size 200 --reset

# Use qwen3:4b model
python main.py run-all --sample-size 200 --model qwen3:4b
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
python main.py evaluate [--samples data/sampled_bookmarks.json] [--queries data/generated_queries.json] [--top-k 20]
```

## Key Features

### Incremental Processing
- Automatically resumes from where it left off if interrupted
- Saves data after each bookmark is processed
- Only processes what's needed to reach target sample size

### Smart Sample Management
- If you have 150 samples and want 200, it adds exactly 50 more
- Skips already processed bookmarks
- Handles failed web requests gracefully

### Windows Compatible
- Handles Windows file permission issues
- Uses safe atomic file operations with fallback

## Output Files

- `data/sampled_bookmarks.json` - Sampled bookmarks with content
- `data/generated_queries.json` - Generated search queries  
- `results/evaluation_results.json` - Detailed evaluation results
- `results/evaluation_summary.csv` - CSV summary of results
- `chroma_db_eval/` - ChromaDB database directory

## Metrics

- **Recall@K**: Percentage of queries that found their target bookmark in top K results
- **Mean Reciprocal Rank (MRR)**: Average of 1/rank for each query
- **Mean/Median Rank**: Average and median position of correct results
- **Distance**: Cosine distance between query and document embeddings

## Configuration

- **Embedding Model**: `all-MiniLM-L6-v2` (default)
- **Query Generation Models**: `qwen3:4b` default
- **Sample Size**: 200 bookmarks (default)  
- **Top-K Results**: 20 (default)
- **Exclude List**: Automatically loaded from LocalMind config (`~/.localmind/config.json`) or uses defaults

### Exclude List

The tool automatically filters out bookmarks that match patterns in the LocalMind exclude list:
- Default excludes: `node_modules`, `.git`, `build`, `dist`, `coverage`, `tmp`, etc.
- Loads from LocalMind configuration if available
- Filters based on both URL paths and bookmark titles
- Provides clear logging of excluded bookmarks

## Examples

```bash
# Start with 50 samples
python main.py run-all --sample-size 50

# Later, expand to 200 (adds 150 more)
python main.py run-all --sample-size 200

# Reset and start over with different model
python main.py run-all --sample-size 100 --model qwen3:4b --reset

# Just generate more queries for existing samples
python main.py sample-and-generate --sample-size 300
```