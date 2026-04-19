# LocalMind Retrieval Evaluation Tool

Evaluates BM25, vector, and hybrid (RRF) retrieval over LocalMind's production
corpus. Measures MRR, Recall@K, and NDCG@K using chunks sampled from the live
SQLite database.

---

## Setup

```bash
uv sync
source .venv/Scripts/activate   # Windows Git Bash
```

All scripts must run inside this venv. The embedding server (LM Studio) must be
running for any command that embeds queries or generates text.

---

## Evaluation pipeline

There are two corpus sampling strategies. Use the cluster-based one for realistic
hard-negative evaluation; use quality-chunks for a quick sanity check.

### Cluster-based (recommended)

Samples from the largest topic clusters in the production corpus, creating
within-topic hard negatives.

```bash
# 1. Sample corpus from production SQLite embeddings
python cluster_sampler.py \
  --n-clusters 50 --top-clusters 20 --per-cluster 15

# 2. Generate search terms for each eval chunk
python main.py generate-chunk-terms --chunks data/cluster_eval_chunks.json

# 3. Build qrels
python make_qrels.py \
  --terms data/chunk_terms.json \
  --chunks data/cluster_eval_chunks.json \
  --output data/cluster_queries.json

# 4. Run eval (sanity-check first with --sample 20)
python run_retrieval_eval.py evaluate \
  --queries data/cluster_queries.json \
  --chunks data/cluster_corpus.json \
  --lmstudio \
  --embedding-model text-embedding-embeddinggemma-300m-qat \
  --sample 20
```

### Quality-chunks (quick sanity check)

Uses the existing quality-filtered chunk set. Faster but less rigorous — queries
are generated from the same chunks, which inflates vector scores.

```bash
python main.py run-all --sample-size 200
python run_retrieval_eval.py evaluate \
  --queries data/generated_queries.json \
  --chunks data/quality_chunks.json \
  --lmstudio \
  --embedding-model text-embedding-embeddinggemma-300m-qat
```

---

## Commands

### `run_retrieval_eval.py evaluate`

```text
--queries        Path to qrels JSON             (default: data/generated_queries.json)
--chunks         Path to corpus JSON            (default: data/quality_chunks.json)
--embedding-model                               (default: all-MiniLM-L6-v2)
--lmstudio       Use LM Studio for embeddings
--ollama         Use Ollama for embeddings
--lmstudio-url                                  (default: http://localhost:1234)
--top-k                                         (default: 20)
--cache-dir      Embedding cache directory      (default: ./chunk_embeddings_cache)
--output         Results JSON path              (default: results/retrieval_comparison.json)
--sample N       Sanity-check N random queries first; abort if vector MRR < 0.1
--expand N       Generate N LLM query variants per query and fuse with two-stage RRF
--expand-model   Chat model for expansion       (default: gemma-4-e4b)
```

Always use `--sample 20` on first run after any pipeline change to catch
embedding-space mismatches before committing to a full hour-long run.

### `run_retrieval_eval.py paraphrase-queries`

Rewrites queries with natural user phrasing (simulates "half-remembering"
the content) to reduce the semantic echo between generated queries and their
source chunks.

```bash
python run_retrieval_eval.py paraphrase-queries \
  --queries data/generated_queries.json \
  --chunks data/quality_chunks.json \
  --output data/paraphrased_queries.json \
  --model gemma-4-e4b
```

Supports `--resume` (default on) to continue interrupted runs.

### `cluster_sampler.py`

```text
--n-clusters     Total K-means clusters to fit  (default: 50)
--top-clusters   Keep N largest clusters        (default: 20)
--per-cluster    Chunks sampled per cluster     (default: 15)
--corpus-output                                 (default: data/cluster_corpus.json)
--eval-output                                   (default: data/cluster_eval_chunks.json)
--db-path        Path to localmind.db           (default: %APPDATA%/localmind/localmind.db)
--embedding-model                               (default: text-embedding-embeddinggemma-300m-qat)
```

Reads chunk embeddings directly from the production SQLite database (stored as
bincode `Vec<f32>` blobs). Filters stub bookmarks (< 30 words, or starting with
`Bookmark:`/`URL:`). Chunk IDs are stable SQLite embedding IDs so corpus and
qrels files stay consistent regardless of sort order.

### `make_qrels.py`

```text
--terms   chunk_terms.json input               (default: data/chunk_terms.json)
--chunks  Chunk JSON for URL/title lookup      (default: data/quality_chunks.json)
--output  Output qrels file                    (default: data/generated_queries.json)
```

### `main.py` (legacy pipeline)

Covers chunk quality filtering, search-term generation, and the original
bookmark-based embedding evaluation. Run `python main.py --help` for full
command list. LLM defaults to `gemma-4-e4b` via LM Studio.

---

## Metrics

| Metric       | Description                                                                    |
| ------------ | ------------------------------------------------------------------------------ |
| **MRR**      | Mean Reciprocal Rank — primary metric                                          |
| **Recall@K** | Fraction of queries where the relevant chunk appears in top K (K = 5, 10, 20) |
| **NDCG@K**   | Normalised Discounted Cumulative Gain at K                                     |

Each query has exactly one relevant chunk (the one its search terms were
generated from). IDCG = 1 for all queries, so NDCG = 1/log₂(rank+1).

---

## Multi-query expansion (`--expand N`)

Generates N alternative phrasings per query via LM Studio, runs all variants
against both indices, and fuses results with two-stage RRF:

1. Per-variant: BM25 + vector → local hybrid
2. Across variants: RRF of all local hybrids → final ranking

Reports three additional columns: `MQ-BM25`, `MQ-Vector`, `MQ-Hybrid`.

**Note**: Expansion did not improve MRR when queries were generated from chunks
(original query is already well-matched). Re-evaluate once real user queries are
available from the shadow log.

---

## Embedding cache

The evaluator caches corpus embeddings as `{model}_chunks.pkl` in
`./chunk_embeddings_cache/`. If vector MRR is near-random, delete the stale pkl
and re-run — the evaluator will re-embed from the corpus JSON.

The cluster sampler no longer writes to the cache. The evaluator always
re-embeds the cluster corpus using its own prefix conventions
(`title: {title} | text: {text}` for documents, `task: search result | query: {query}` for queries),
which must match for cosine similarity to be meaningful.

---

## Data files

| File                                  | Description                                                        |
| ------------------------------------- | ------------------------------------------------------------------ |
| `data/cluster_corpus.json`            | Retrieval index (300 chunks, 20 topic clusters)                    |
| `data/cluster_eval_chunks.json`       | Query generation source (same chunks, sorted by centroid distance) |
| `data/cluster_queries.json`           | Qrels for cluster eval                                             |
| `data/chunk_terms.json`               | LLM-generated search terms per chunk                              |
| `data/quality_chunks.json`            | Quality-filtered chunks from bookmark sampler                      |
| `data/generated_queries.json`         | Qrels for quality-chunk eval                                       |
| `data/paraphrased_queries.json`       | Rewritten queries (reduced embedding bias)                         |
| `results/retrieval_comparison.json`   | Last eval results                                                  |
| `chunk_embeddings_cache/*.pkl`        | Cached corpus embeddings (delete to force re-embed)                |

---

## Shadow query log

The production app writes real user searches to
`%APPDATA%/localmind/query_log.jsonl`. Each line is a JSON object:

```json
{
  "timestamp": 1745123456,
  "query": "how does attention work",
  "results": [{"rank": 1, "doc_id": 42, "title": "...", "score": 0.87}],
  "outcome": "clicked",
  "clicked_doc_id": 42
}
```

Outcomes: `clicked`, `abandoned`, `new_search`. Use this log to build a
ground-truth eval set of real user queries for unbiased evaluation of retrieval
and multi-query expansion.
