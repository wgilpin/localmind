#!/usr/bin/env python3
"""Standalone CLI for BM25 / vector / hybrid retrieval evaluation."""

import json
import random
import re
import sys
import click
from pathlib import Path
from tqdm import tqdm
from retrieval_evaluator import RetrievalEvaluator, print_results
from chunk_quality_filter import ChunkQualityFilter
from lmstudio_client import LMStudioClient


EXPAND_SYSTEM = (
    "Generate {n} alternative search queries for the same information need. "
    "Use different vocabulary, phrasing styles, and levels of specificity. "
    "Keep domain terms. Return one query per line, no numbering or explanation."
)


def generate_variants(query: str, n: int, client, model: str) -> list:
    prompt = EXPAND_SYSTEM.format(n=n) + f"\n\nOriginal query: {query}"
    try:
        response = client.chat(
            model=model,
            messages=[{"role": "user", "content": prompt}],
            temperature=0.8,
            max_tokens=256,
        )
        raw = response['message']['content']
        raw = re.sub(r'<think>.*?</think>', '', raw, flags=re.DOTALL)
        variants = [line.strip().strip('"').strip() for line in raw.splitlines()]
        variants = [v for v in variants if v and v.lower() != query.lower()]
        return variants[:n]
    except Exception as e:
        print(f"  [expand] Error for '{query[:40]}': {e}")
        return []


PARAPHRASE_SYSTEM = (
    "You are a search query rewriter simulating a real user. "
    "Rewrite the query as a person would naturally phrase it when they "
    "half-remember reading something on this topic — they know roughly what "
    "they want but may use different words, a broader framing, or ask it as "
    "a question. Keep domain terms where natural (e.g. 'larb', 'transformer'). "
    "Do not copy phrases verbatim from the original query. "
    "Return only the rewritten query, no explanation."
)

PARAPHRASE_USER = """\
Topic context (what the content is about):
\"\"\"
{chunk_text}
\"\"\"

Original query: {query}

Rewritten query (natural user phrasing, same intent):"""


@click.group()
def cli():
    pass


@cli.command()
@click.option('--queries', default='data/generated_queries.json', help='generated_queries.json path')
@click.option('--chunks', default='data/quality_chunks.json', help='quality_chunks.json path')
@click.option('--embedding-model', default='all-MiniLM-L6-v2', help='Embedding model name')
@click.option('--lmstudio', is_flag=True, help='Use LM Studio for query embeddings')
@click.option('--ollama', is_flag=True, help='Use Ollama for query embeddings')
@click.option('--lmstudio-url', default='http://localhost:1234', help='LM Studio API URL')
@click.option('--ollama-url', default='http://localhost:11434', help='Ollama API URL')
@click.option('--top-k', default=20, help='Results to retrieve per query')
@click.option('--cache-dir', default='./chunk_embeddings_cache', help='Embedding cache directory')
@click.option('--output', default='results/retrieval_comparison.json', help='Output JSON path')
@click.option('--sample', default=0, help='Run a sanity check on N random queries first; abort if vector MRR < 0.1')
@click.option('--expand', default=0, help='Generate N query variants per query via LLM (0 = off)')
@click.option('--expand-model', default='gemma-4-e4b', help='LM Studio chat model for query expansion')
def evaluate(queries, chunks, embedding_model, lmstudio, ollama, lmstudio_url, ollama_url, top_k, cache_dir, output, sample, expand, expand_model):
    """Evaluate BM25, vector, and hybrid retrieval. Metrics: MRR, Recall@K, NDCG@K."""

    with open(queries, encoding='utf-8') as f:
        queries_data = json.load(f)

    filter_ = ChunkQualityFilter()
    chunk_list = filter_.load_filtered_chunks(chunks)
    print(f"Loaded {len(chunk_list)} chunks from {chunks}")

    evaluator = RetrievalEvaluator(
        embedding_model=embedding_model,
        cache_dir=cache_dir,
        use_ollama=ollama,
        use_lmstudio=lmstudio,
        ollama_url=ollama_url,
        lmstudio_url=lmstudio_url,
    )

    evaluator.load_index(chunk_list)

    if sample > 0:
        sample_keys = random.sample(list(queries_data.keys()), min(sample, len(queries_data)))
        sample_queries = {k: queries_data[k] for k in sample_keys}
        print(f"\n--- Sanity check: {len(sample_queries)} random queries ---")
        sample_results = evaluator.evaluate(sample_queries, top_k=top_k)
        print_results(sample_results)
        vector_mrr = sample_results["vector"]["mrr"]
        if vector_mrr < 0.1:
            click.echo(f"\nVector MRR={vector_mrr:.4f} is near-random. Aborting before full run.")
            click.echo("Check your embedding cache (delete the .pkl and retry).")
            sys.exit(1)
        click.echo(f"\nSanity check passed (vector MRR={vector_mrr:.4f}). Continuing with full eval...\n")

    if expand > 0:
        expand_client = LMStudioClient(base_url=lmstudio_url)
        query_variants: dict = {}
        print(f"Generating {expand} variants per query via {expand_model}...")
        for qid, q in tqdm(queries_data.items()):
            query_variants[qid] = generate_variants(q["text"], expand, expand_client, expand_model)
        results = evaluator.evaluate_multi_query(queries_data, query_variants, top_k=top_k)
    else:
        results = evaluator.evaluate(queries_data, top_k=top_k)

    print_results(results)

    output_path = Path(output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(results, f, indent=2)
    click.echo(f"\nResults saved to {output_path}")


@cli.command()
@click.option('--queries', default='data/generated_queries.json', help='Input queries file')
@click.option('--chunks', default='data/quality_chunks.json', help='quality_chunks.json for chunk text lookup')
@click.option('--output', default='data/paraphrased_queries.json', help='Output paraphrased queries file')
@click.option('--model', default='gemma-4-e4b', help='LM Studio chat model for paraphrasing')
@click.option('--lmstudio-url', default='http://localhost:1234', help='LM Studio API URL')
@click.option('--resume/--no-resume', default=True, help='Resume from existing output file')
def paraphrase_queries(queries, chunks, output, model, lmstudio_url, resume):
    """Rewrite queries using different vocabulary to reduce embedding bias."""

    with open(queries, encoding='utf-8') as f:
        queries_data = json.load(f)

    # Build chunk_id -> chunk_text lookup
    filter_ = ChunkQualityFilter()
    chunk_list = filter_.load_filtered_chunks(chunks)
    chunk_text_map = {c.chunk_id: c.chunk_text for c in chunk_list}
    print(f"Loaded {len(chunk_list)} chunks for text lookup")

    # Load existing output to resume
    output_path = Path(output)
    existing: dict = {}
    if resume and output_path.exists():
        with open(output_path, encoding='utf-8') as f:
            existing = json.load(f)
        print(f"Resuming: {len(existing)} queries already paraphrased")

    client = LMStudioClient(base_url=lmstudio_url)

    todo = {qid: q for qid, q in queries_data.items() if qid not in existing}
    skipped_no_chunk = 0
    print(f"Paraphrasing {len(todo)} queries...")

    for qid, q in tqdm(todo.items()):
        chunk_id = q.get('chunk_id')
        chunk_text = chunk_text_map.get(chunk_id, '')

        if not chunk_text:
            # No chunk text available — keep original query unchanged
            existing[qid] = q
            skipped_no_chunk += 1
            continue

        prompt = PARAPHRASE_USER.format(
            chunk_text=chunk_text[:800],
            query=q['text'],
        )

        try:
            response = client.chat(
                model=model,
                messages=[
                    {"role": "system", "content": PARAPHRASE_SYSTEM},
                    {"role": "user", "content": prompt},
                ],
                temperature=0.7,
                max_tokens=1024,
            )
            raw = response['message']['content']
            # Strip thinking tokens emitted by reasoning models
            raw = re.sub(r'<think>.*?</think>', '', raw, flags=re.DOTALL)
            rewritten = raw.strip().strip('"').strip()
            if not rewritten:
                print(f"\n  [EMPTY] {qid} original='{q['text']}'")
                print(f"  [EMPTY] raw model output: {repr(response['message']['content'][:200])}")
                rewritten = q['text']  # fall back to original
        except Exception as e:
            print(f"  Error on {qid}: {e} — keeping original")
            rewritten = q['text']

        existing[qid] = {
            "text": rewritten,
            "original_text": q['text'],
            "relevant_doc_ids": q['relevant_doc_ids'],
            "chunk_id": q['chunk_id'],
            "document_title": q.get('document_title', ''),
        }

        # Save incrementally every 10 queries
        if len(existing) % 10 == 0:
            output_path.parent.mkdir(parents=True, exist_ok=True)
            with open(output_path, 'w', encoding='utf-8') as f:
                json.dump(existing, f, indent=2, ensure_ascii=False)

    # Final save
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(existing, f, indent=2, ensure_ascii=False)

    if skipped_no_chunk:
        print(f"  {skipped_no_chunk} queries kept original (chunk not in quality_chunks.json)")
    click.echo(f"Saved {len(existing)} paraphrased queries to {output_path}")


if __name__ == '__main__':
    cli()
