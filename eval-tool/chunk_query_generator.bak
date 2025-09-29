#!/usr/bin/env python3
"""
Chunk Query Generator Module - Fixed Version
Generates 2-3 search terms per chunk using LLM that someone would use to find this information.
No fallback terms - LLM must succeed.
"""

import json
from pathlib import Path
from typing import List, Dict, Optional
from dataclasses import dataclass, asdict
from chunk_quality_filter import QualityChunkSample
import time
import re
from tqdm import tqdm
from lmstudio_client import LMStudioClient


@dataclass
class ChunkSearchTerms:
    """Search terms generated for a chunk"""
    chunk_id: int
    document_id: int
    search_terms: List[str]
    generation_model: str
    chunk_preview: str  # First 100 chars of chunk for reference


class ChunkQueryGenerator:
    """Generate search queries for chunks using LLM"""

    def __init__(self, model: str = "qwen3:4b", lmstudio_url: str = "http://localhost:1234"):
        """Initialize with LM Studio model"""
        self.model = model
        self.lmstudio_client = LMStudioClient(base_url=lmstudio_url)
        self.generation_cache = {}  # Cache generated terms

        # Statistics
        self.stats = {
            'chunks_processed': 0,
            'terms_generated': 0,
            'errors': 0,
            'avg_terms_per_chunk': 0
        }

    def generate_search_terms(self, chunk: QualityChunkSample, num_terms: int = 3) -> ChunkSearchTerms:
        """
        Generate search terms for a chunk

        Args:
            chunk: QualityChunkSample to generate terms for
            num_terms: Target number of terms to generate (2-3 typical)

        Returns:
            ChunkSearchTerms with generated queries
        """
        # Check cache
        cache_key = f"{chunk.chunk_id}_{chunk.chunk_text[:50]}"
        if cache_key in self.generation_cache:
            return self.generation_cache[cache_key]

        # Create context-aware prompt
        context_info = f"Document: {chunk.document_title}" if chunk.document_title else ""

        prompt = f"""You need to generate exactly {num_terms} search terms that someone would use to find this information.

{context_info}

Chunk text:
"{chunk.chunk_text}"

You can think through this, but end your response with exactly {num_terms} search terms, one per line.

Search terms must be:
- 1-4 words each
- Natural queries a user would type
- Semantically relevant to the chunk content
- Different from each other
- NOT direct quotes from the text
- NOT generic terms like "information" or "content"
- NOT the document title verbatim

Good examples: "llm evaluation tools", "python frameworks", "anthropic inspect"

Think about what someone would search for to find this specific information, then provide the {num_terms} search terms at the end."""

        try:
            response = self.lmstudio_client.chat(
                model=self.model,
                messages=[
                    {
                        'role': 'user',
                        'content': prompt
                    }
                ],
                temperature=0.7,  # Moderate creativity for diverse terms
                top_p=0.9,
                max_tokens=500  # Allow more tokens for thinking + terms
            )

            # Parse response - extract search terms after any thinking
            response_text = response['message']['content'].strip()

            # Split into lines and find the search terms (usually at the end)
            lines = response_text.split('\n')
            terms = []

            # Look for the actual search terms - they're usually clean lines without special characters
            for line in lines:
                line = line.strip()

                # Skip thinking tags, explanations, and empty lines
                if not line or line.startswith('<') or 'search terms' in line.lower() or 'think' in line.lower():
                    continue

                # Remove common prefixes and formatting
                line = re.sub(r'^[-•\*\d+\.\s]*', '', line)
                line = line.strip('"\'')

                # Check if it looks like a search term (1-4 words, no special chars)
                if line and 1 <= len(line.split()) <= 4 and not any(char in line for char in '<>[]{}()'):
                    terms.append(line.lower())

            # If we didn't find enough terms, fail rather than use fallback
            if len(terms) < num_terms:
                raise Exception(f"Only found {len(terms)} valid search terms, needed {num_terms}. Raw response: {response_text[:200]}...")

            # Take exactly num_terms
            terms = terms[:num_terms]

            # Create result
            result = ChunkSearchTerms(
                chunk_id=chunk.chunk_id,
                document_id=chunk.document_id,
                search_terms=terms,
                generation_model=self.model,
                chunk_preview=chunk.chunk_text[:100]
            )

            # Update statistics
            self.stats['chunks_processed'] += 1
            self.stats['terms_generated'] += len(terms)

            # Cache result
            self.generation_cache[cache_key] = result
            return result

        except Exception as e:
            print(f"[FAIL] Error generating terms for chunk {chunk.chunk_id}: {e}")
            self.stats['errors'] += 1
            raise e  # Don't use fallback, let it fail

    def generate_for_chunks(self, chunks: List[QualityChunkSample],
                           num_terms_per_chunk: int = 3,
                           show_progress: bool = True) -> Dict[int, ChunkSearchTerms]:
        """
        Generate search terms for multiple chunks

        Args:
            chunks: List of quality chunks to process
            num_terms_per_chunk: Number of terms to generate per chunk
            show_progress: Whether to show progress

        Returns:
            Dictionary mapping chunk_id to ChunkSearchTerms
        """
        results = {}

        print(f"\n[SEARCH] Generating search terms for {len(chunks)} chunks using {self.model}...")

        # Use tqdm for progress bar
        chunk_iterator = tqdm(chunks, desc="Generating terms", unit="chunk") if show_progress else chunks

        for chunk in chunk_iterator:
            try:
                terms = self.generate_search_terms(chunk, num_terms_per_chunk)
                results[chunk.chunk_id] = terms
            except Exception as e:
                print(f"[SKIP] Failed to generate terms for chunk {chunk.chunk_id}: {e}")
                continue

            # Small delay to avoid overwhelming LM Studio
            time.sleep(0.1)

        # Update average
        if self.stats['chunks_processed'] > 0:
            self.stats['avg_terms_per_chunk'] = (
                self.stats['terms_generated'] / self.stats['chunks_processed']
            )

        self.print_statistics()
        return results

    def batch_generate(self, chunks: List[QualityChunkSample],
                      batch_size: int = 5) -> Dict[int, ChunkSearchTerms]:
        """
        Generate terms in batches for efficiency

        Args:
            chunks: List of chunks to process
            batch_size: Number of chunks per batch

        Returns:
            Dictionary mapping chunk_id to ChunkSearchTerms
        """
        all_results = {}

        print(f"\n[SEARCH] Batch generating terms for {len(chunks)} chunks (batch size: {batch_size})...")

        # Create overall progress bar for batches
        total_batches = (len(chunks) + batch_size - 1) // batch_size
        batch_progress = tqdm(total=total_batches, desc="Processing batches", unit="batch")

        for i in range(0, len(chunks), batch_size):
            batch = chunks[i:i + batch_size]
            batch_results = self.generate_for_chunks(batch, show_progress=False)
            all_results.update(batch_results)

            terms_count = sum(len(terms.search_terms) for terms in batch_results.values())
            batch_progress.set_postfix({"terms": terms_count, "chunks": len(batch)})
            batch_progress.update(1)

        batch_progress.close()

        self.print_statistics()
        return all_results

    def validate_search_terms(self, terms_dict: Dict[int, ChunkSearchTerms]) -> Dict[str, any]:
        """
        Validate and analyze generated search terms

        Args:
            terms_dict: Dictionary of generated terms

        Returns:
            Validation statistics
        """
        stats = {
            'total_chunks': len(terms_dict),
            'total_terms': 0,
            'avg_terms_per_chunk': 0,
            'unique_terms': set(),
            'term_lengths': [],
            'empty_chunks': 0
        }

        for chunk_id, chunk_terms in terms_dict.items():
            terms = chunk_terms.search_terms

            if not terms:
                stats['empty_chunks'] += 1

            stats['total_terms'] += len(terms)
            stats['unique_terms'].update(terms)

            for term in terms:
                stats['term_lengths'].append(len(term.split()))

        if stats['total_chunks'] > 0:
            stats['avg_terms_per_chunk'] = stats['total_terms'] / stats['total_chunks']

        stats['unique_terms_count'] = len(stats['unique_terms'])

        if stats['term_lengths']:
            stats['avg_term_length'] = sum(stats['term_lengths']) / len(stats['term_lengths'])
        else:
            stats['avg_term_length'] = 0

        # Remove the set for JSON serialization
        del stats['unique_terms']
        del stats['term_lengths']

        return stats

    def save_search_terms(self, terms_dict: Dict[int, ChunkSearchTerms],
                         output_path: str = 'data/chunk_terms.json'):
        """Save generated search terms to JSON file"""
        output_path = Path(output_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)

        # Convert to JSON-serializable format
        output_data = {
            'metadata': {
                'model': self.model,
                'total_chunks': len(terms_dict),
                'total_terms': sum(len(t.search_terms) for t in terms_dict.values()),
                'timestamp': time.strftime('%Y-%m-%d %H:%M:%S')
            },
            'terms': {}
        }

        for chunk_id, chunk_terms in terms_dict.items():
            output_data['terms'][str(chunk_id)] = asdict(chunk_terms)

        with open(output_path, 'w', encoding='utf-8') as f:
            json.dump(output_data, f, indent=2, ensure_ascii=False)

        print(f"[SAVE] Saved search terms for {len(terms_dict)} chunks to {output_path}")

    def load_search_terms(self, input_path: str = 'data/chunk_terms.json') -> Dict[int, ChunkSearchTerms]:
        """Load search terms from JSON file"""
        input_path = Path(input_path)

        if not input_path.exists():
            print(f"[WARN] File not found: {input_path}")
            return {}

        with open(input_path, 'r', encoding='utf-8') as f:
            data = json.load(f)

        terms_dict = {}
        for chunk_id_str, terms_data in data['terms'].items():
            chunk_id = int(chunk_id_str)
            terms_dict[chunk_id] = ChunkSearchTerms(**terms_data)

        print(f"[FOLDER] Loaded search terms for {len(terms_dict)} chunks from {input_path}")
        return terms_dict

    def print_statistics(self):
        """Print generation statistics"""
        print("\n" + "="*60)
        print("📊 SEARCH TERM GENERATION STATISTICS")
        print("="*60)
        print(f"Chunks processed: {self.stats['chunks_processed']}")
        print(f"Terms generated: {self.stats['terms_generated']}")
        print(f"Average terms/chunk: {self.stats['avg_terms_per_chunk']:.2f}")
        print(f"Errors: {self.stats['errors']}")
        print("="*60 + "\n")

    def analyze_search_terms(self, terms_dict: Dict[int, ChunkSearchTerms], top_n: int = 20):
        """Analyze and display search term patterns"""
        print("\n" + "="*60)
        print("[SEARCH] SEARCH TERM ANALYSIS")
        print("="*60)

        # Collect all terms
        all_terms = []
        term_frequency = {}

        for chunk_terms in terms_dict.values():
            for term in chunk_terms.search_terms:
                all_terms.append(term)
                term_frequency[term] = term_frequency.get(term, 0) + 1

        # Sort by frequency
        sorted_terms = sorted(term_frequency.items(), key=lambda x: x[1], reverse=True)

        print(f"Total unique terms: {len(term_frequency)}")
        print(f"Total terms generated: {len(all_terms)}")

        print(f"\nTop {top_n} most common terms:")
        for term, count in sorted_terms[:top_n]:
            print(f"  - '{term}': {count} occurrences")

        # Analyze term lengths
        term_lengths = [len(term.split()) for term in all_terms]
        if term_lengths:
            avg_length = sum(term_lengths) / len(term_lengths)
            print(f"\nTerm length distribution:")
            for length in range(1, 6):
                count = term_lengths.count(length)
                pct = 100 * count / len(term_lengths)
                print(f"  {length} word(s): {count} ({pct:.1f}%)")

        # Sample terms
        print("\nSample generated terms (first 5 chunks):")
        for i, (chunk_id, chunk_terms) in enumerate(list(terms_dict.items())[:5]):
            print(f"\n  Chunk {chunk_id}:")
            print(f"  Preview: {chunk_terms.chunk_preview}...")
            print(f"  Terms: {', '.join(chunk_terms.search_terms)}")

        print("="*60 + "\n")


def main():
    """Test the query generator"""
    import click

    @click.command()
    @click.option('--chunks', default='data/quality_chunks.json', help='Input quality chunks file')
    @click.option('--model', default='qwen/qwen3-4b', help='LM Studio model for generation')
    @click.option('--num-terms', default=3, help='Number of terms per chunk')
    @click.option('--analyze', is_flag=True, help='Analyze generated terms')
    def test(chunks, model, num_terms, analyze):
        """Test search term generation"""

        from chunk_quality_filter import ChunkQualityFilter

        # Load quality chunks
        filter = ChunkQualityFilter()
        quality_chunks = filter.load_filtered_chunks(chunks)

        if not quality_chunks:
            print("No quality chunks found. Run quality filtering first.")
            return

        # Initialize generator
        generator = ChunkQueryGenerator(model=model)

        # Generate terms for first 10 chunks as test
        test_chunks = quality_chunks[:10]
        terms_dict = generator.generate_for_chunks(test_chunks, num_terms_per_chunk=num_terms)

        # Save results
        generator.save_search_terms(terms_dict)

        # Validate
        validation_stats = generator.validate_search_terms(terms_dict)
        print("\nValidation statistics:")
        for key, value in validation_stats.items():
            print(f"  {key}: {value}")

        # Analyze if requested
        if analyze:
            generator.analyze_search_terms(terms_dict)

    test()


if __name__ == '__main__':
    main()