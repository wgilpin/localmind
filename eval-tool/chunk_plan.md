# Chunk-based Evaluation Plan for eval-tool

## Overview

Modify the eval-tool to work with actual 500-character chunks from the localmind-rs database instead of entire documents, incorporating LLM-based quality assessment and search term generation.

## Current State

- **eval-tool**: Works with entire bookmark documents from Chrome bookmarks
- **localmind-rs**: Chunks documents into ~500 character pieces stored in `embeddings` table
- **Gap**: Evaluation doesn't reflect actual search behavior (chunks vs full docs)

## Proposed Changes

### 1. Database Integration

#### 1.1 Chunk Sampler Module (`chunk_sampler.py`)
- **Purpose**: Extract chunks from localmind-rs SQLite database
- **Location**: `%APPDATA%/localmind/localmind.db` (Windows)
- **Target table**: `embeddings` table with columns:
  - `id`, `document_id`, `chunk_index`, `chunk_start`, `chunk_end`, `embedding`
- **Functionality**:
  - Connect to localmind-rs database
  - Sample N random chunks (configurable, default 200)
  - Extract chunk text using `chunk_start`/`chunk_end` from parent document
  - Return chunk data with metadata (document context, position, etc.)

#### 1.2 Data Structure
```python
@dataclass
class ChunkSample:
    chunk_id: int
    document_id: int
    chunk_index: int
    chunk_text: str
    document_title: str
    document_url: Optional[str]
    chunk_start: int
    chunk_end: int
    parent_content: str  # Full document for context
```

### 2. LLM Quality Assessment

#### 2.1 Quality Filter Module (`chunk_quality_filter.py`)
- **Purpose**: Use LLM to assess if chunks contain quality English text
- **Model**: Configurable Ollama model (default: `qwen3:4b`)
- **Prompt**:
```
Analyze this text chunk and determine if it contains good quality English text suitable for search evaluation.

Chunk: "{chunk_text}"

Respond with only "SUITABLE" or "UNSUITABLE" followed by a brief reason.

SUITABLE chunks have:
- Clear, meaningful English text
- Complete sentences or coherent fragments
- Searchable content (facts, concepts, descriptions)

UNSUITABLE chunks have:
- Code/programming content
- Random characters or formatting
- Non-English text
- Too short or meaningless content
```

#### 2.2 Implementation
- Filter chunks before query generation
- Log rejection reasons for analysis
- Configurable quality threshold (% of suitable chunks needed)

### 3. Search Term Generation

#### 3.1 Enhanced Query Generator (`chunk_query_generator.py`)
- **Purpose**: Generate 2-3 search terms per suitable chunk
- **Model**: Same as quality assessment
- **Prompt**:
```
Given this text chunk from a larger document, generate 2-3 search terms that someone would likely use to find this specific information.

Chunk: "{chunk_text}"
Document Context: "{document_title}"

Generate search terms that are:
- 1-3 words each
- Natural user queries
- Likely to match this chunk content
- Varied in specificity (broad to narrow)

Examples: "italian recipes", "rust debugging", "first aid", "python error handling"

Return only the search terms, one per line.
```

#### 3.2 Output Format
```python
{
    "chunk_id": 123,
    "search_terms": [
        "machine learning algorithms",
        "neural networks",
        "tensorflow tutorial"
    ],
    "quality_score": "SUITABLE",
    "quality_reason": "Clear technical content with searchable concepts"
}
```

### 4. Modified Evaluation Pipeline

#### 4.1 Updated Vector Evaluator (`chunk_vector_evaluator.py`)
- **Index chunks**: Use actual chunk text instead of full documents
- **Embedding storage**: Store chunk embeddings with proper IDs
- **Search results**: Return chunk-level matches
- **Metrics**: Measure if correct chunk is found, not just document

#### 4.2 New Evaluation Metrics
- **Chunk Hit Rate**: % of queries that find the target chunk in top-k
- **Chunk Position**: Average rank of target chunk in results
- **Document Hit Rate**: % of queries that find the parent document
- **False Positive Rate**: % of irrelevant chunks in top results

### 5. New CLI Commands

#### 5.1 Modified Commands
```bash
# Sample chunks from localmind-rs database
python main.py sample-chunks --sample-size 200 --db-path %APPDATA%/localmind/localmind.db

# Quality filter chunks using LLM
python main.py filter-quality --chunks data/sampled_chunks.json --model qwen3:4b

# Generate search terms for suitable chunks
python main.py generate-terms --chunks data/quality_chunks.json --model qwen3:4b

# Evaluate chunk-based search
python main.py evaluate-chunks --chunks data/quality_chunks.json --terms data/chunk_terms.json

# Full pipeline
python main.py run-chunk-pipeline --sample-size 200 --model qwen3:4b --embedding-model nomic-embed-text
```

## Implementation Plan

### Phase 1: Database Integration
1. Create `chunk_sampler.py` module
2. Implement SQLite connection to localmind-rs database
3. Add chunk sampling with proper text extraction
4. Test with small sample size

### Phase 2: Quality Assessment
1. Create `chunk_quality_filter.py` module
2. Implement LLM-based quality assessment
3. Add configurable quality thresholds
4. Test quality filtering accuracy

### Phase 3: Search Term Generation
1. Extend query generator for chunk-specific terms
2. Implement 2-3 term generation per chunk
3. Add term variety and quality controls
4. Test generated terms relevance

### Phase 4: Evaluation Updates
1. Modify vector evaluator for chunk-level evaluation
2. Implement new chunk-specific metrics
3. Update result analysis and reporting
4. Test evaluation accuracy

### Phase 5: Integration & CLI
1. Add new CLI commands to `main.py`
2. Create integrated pipeline command
3. Update documentation and help text
4. Performance testing and optimization

## Benefits

1. **Realistic Evaluation**: Tests actual search behavior (chunks vs documents)
2. **Quality Control**: LLM filtering ensures meaningful test data
3. **Diverse Search Terms**: Multiple terms per chunk increase coverage
4. **Better Metrics**: Chunk-level precision measures real user experience
5. **Automated**: Minimal manual intervention required

## Dependencies

- **New**: `sqlite3` (built-in Python), path handling for Windows AppData
- **Existing**: `ollama` client for LLM calls, embedding models
- **Database**: Read-only access to localmind-rs SQLite database

## File Structure

```
eval-tool/
├── chunk_sampler.py           # NEW: Sample chunks from localmind-rs DB
├── chunk_quality_filter.py    # NEW: LLM quality assessment
├── chunk_query_generator.py   # NEW: Generate search terms for chunks
├── chunk_vector_evaluator.py  # NEW: Chunk-level evaluation
├── main.py                    # MODIFIED: Add chunk pipeline commands
├── data/
│   ├── sampled_chunks.json    # NEW: Raw chunk samples
│   ├── quality_chunks.json    # NEW: Quality-filtered chunks
│   └── chunk_terms.json       # NEW: Generated search terms
└── results/
    └── chunk_evaluation_results.json  # NEW: Chunk-level results
```

This plan transforms the eval-tool from document-based to chunk-based evaluation, providing a more accurate assessment of the actual localmind-rs search experience.