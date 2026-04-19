#!/usr/bin/env python3
"""Convert chunk_terms.json + quality_chunks.json into a qrels JSON file.

Output format:
{
  "query_001": {
    "text": "adjacency matrix definition",
    "relevant_doc_ids": ["https://example.com/page"]
  },
  ...
}
"""

import json
import click
from pathlib import Path


@click.command()
@click.option('--terms', default='data/chunk_terms.json', help='chunk_terms.json input')
@click.option('--chunks', default='data/quality_chunks.json', help='quality_chunks.json input')
@click.option('--output', default='data/generated_queries.json', help='Output qrels file')
def main(terms, chunks, output):
    with open(terms, encoding='utf-8') as f:
        terms_data = json.load(f)

    with open(chunks, encoding='utf-8') as f:
        chunks_list = json.load(f)

    # Build chunk_id -> document_url lookup
    chunk_to_url = {str(c['chunk_id']): c['document_url'] for c in chunks_list}
    chunk_to_title = {str(c['chunk_id']): c.get('document_title', '') for c in chunks_list}

    qrels = {}
    idx = 1
    for chunk_id, entry in terms_data['terms'].items():
        url = chunk_to_url.get(chunk_id)
        if not url:
            continue
        for term in entry['search_terms']:
            key = f"query_{idx:04d}"
            qrels[key] = {
                "text": term,
                "relevant_doc_ids": [url],
                "chunk_id": entry['chunk_id'],
                "document_title": chunk_to_title.get(chunk_id, ''),
            }
            idx += 1

    Path(output).parent.mkdir(parents=True, exist_ok=True)
    with open(output, 'w', encoding='utf-8') as f:
        json.dump(qrels, f, indent=2, ensure_ascii=False)

    click.echo(f"Written {len(qrels)} queries to {output}")


if __name__ == '__main__':
    main()
