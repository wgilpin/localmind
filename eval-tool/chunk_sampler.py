#!/usr/bin/env python3
"""
Chunk Sampler Module
Samples text chunks from the localmind-rs SQLite database for evaluation.
"""

import sqlite3
import json
import random
from pathlib import Path
from typing import List, Dict, Optional, Tuple
from dataclasses import dataclass, asdict
import os
import platform


@dataclass
class ChunkSample:
    """Represents a sampled chunk with its metadata"""
    chunk_id: int
    document_id: int
    chunk_index: int
    chunk_text: str
    document_title: str
    document_url: Optional[str]
    chunk_start: int
    chunk_end: int
    parent_content: str  # Full document for context
    embedding_id: int  # ID from embeddings table


class ChunkSampler:
    """Samples chunks from localmind-rs database for evaluation"""

    def __init__(self, db_path: Optional[str] = None):
        """Initialize the sampler with database path"""
        if db_path:
            self.db_path = Path(db_path)
        else:
            # Default path based on OS
            if platform.system() == 'Windows':
                app_data = os.environ.get('APPDATA', '')
                self.db_path = Path(app_data) / 'localmind' / 'localmind.db'
            else:
                # Linux/macOS
                home = Path.home()
                self.db_path = home / '.localmind' / 'localmind.db'

        if not self.db_path.exists():
            raise FileNotFoundError(f"LocalMind database not found at {self.db_path}")

        print(f"Using LocalMind database: {self.db_path}")

    def get_connection(self) -> sqlite3.Connection:
        """Create a database connection"""
        return sqlite3.connect(self.db_path)

    def get_total_chunks(self) -> int:
        """Get total number of chunks in the database"""
        with self.get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT COUNT(*) FROM embeddings")
            return cursor.fetchone()[0]

    def get_chunk_by_id(self, embedding_id: int) -> Optional[ChunkSample]:
        """Retrieve a specific chunk by its embedding ID"""
        with self.get_connection() as conn:
            cursor = conn.cursor()

            # Get chunk metadata from embeddings table
            cursor.execute("""
                SELECT e.id, e.document_id, e.chunk_index, e.chunk_start, e.chunk_end,
                       d.title, d.content, d.url
                FROM embeddings e
                JOIN documents d ON e.document_id = d.id
                WHERE e.id = ?
            """, (embedding_id,))

            row = cursor.fetchone()
            if not row:
                return None

            (chunk_id, doc_id, chunk_index, chunk_start, chunk_end,
             doc_title, doc_content, doc_url) = row

            # Extract chunk text from document content
            chunk_text = doc_content[chunk_start:chunk_end] if doc_content else ""

            return ChunkSample(
                chunk_id=chunk_id,
                document_id=doc_id,
                chunk_index=chunk_index,
                chunk_text=chunk_text,
                document_title=doc_title or "",
                document_url=doc_url,
                chunk_start=chunk_start,
                chunk_end=chunk_end,
                parent_content=doc_content or "",
                embedding_id=chunk_id
            )

    def sample_chunks(self, sample_size: int = 200,
                     exclude_dead: bool = True,
                     min_chunk_length: int = 50) -> List[ChunkSample]:
        """
        Sample random chunks from the database

        Args:
            sample_size: Number of chunks to sample
            exclude_dead: Whether to exclude documents marked as dead
            min_chunk_length: Minimum chunk text length to consider

        Returns:
            List of ChunkSample objects
        """
        with self.get_connection() as conn:
            cursor = conn.cursor()

            # Build query based on filters
            query = """
                SELECT e.id, e.document_id, e.chunk_index, e.chunk_start, e.chunk_end,
                       d.title, d.content, d.url
                FROM embeddings e
                JOIN documents d ON e.document_id = d.id
                WHERE 1=1
            """

            params = []
            if exclude_dead:
                query += " AND (d.is_dead IS NULL OR d.is_dead = 0)"

            # Get all valid chunk IDs first
            # Build a simpler query for just IDs
            id_query = """
                SELECT e.id
                FROM embeddings e
                JOIN documents d ON e.document_id = d.id
                WHERE 1=1
            """
            if exclude_dead:
                id_query += " AND (d.is_dead IS NULL OR d.is_dead = 0)"

            cursor.execute(id_query, params)
            all_chunk_ids = [row[0] for row in cursor.fetchall()]

            if not all_chunk_ids:
                print("Warning: No chunks found in database")
                return []

            # Sample random chunk IDs
            sample_count = min(sample_size, len(all_chunk_ids))
            sampled_ids = random.sample(all_chunk_ids, sample_count)

            # Fetch full data for sampled chunks
            samples = []
            for chunk_id in sampled_ids:
                chunk = self.get_chunk_by_id(chunk_id)
                if chunk and len(chunk.chunk_text) >= min_chunk_length:
                    samples.append(chunk)

            print(f"Sampled {len(samples)} chunks from {len(all_chunk_ids)} total chunks")
            return samples

    def sample_chunks_by_quality(self, sample_size: int = 200,
                                 min_words: int = 20,
                                 max_code_ratio: float = 0.3) -> List[ChunkSample]:
        """
        Sample chunks with basic quality filtering

        Args:
            sample_size: Number of chunks to sample
            min_words: Minimum word count in chunk
            max_code_ratio: Maximum ratio of code-like characters

        Returns:
            List of quality-filtered ChunkSample objects
        """
        # Start with larger sample to account for filtering
        initial_sample = self.sample_chunks(sample_size * 2)

        filtered_samples = []
        for chunk in initial_sample:
            text = chunk.chunk_text

            # Basic quality checks
            word_count = len(text.split())
            if word_count < min_words:
                continue

            # Check for code-like content (brackets, semicolons, etc.)
            code_chars = sum(1 for c in text if c in '{}[]();=<>')
            code_ratio = code_chars / len(text) if text else 1
            if code_ratio > max_code_ratio:
                continue

            filtered_samples.append(chunk)

            if len(filtered_samples) >= sample_size:
                break

        print(f"Quality-filtered to {len(filtered_samples)} chunks")
        return filtered_samples

    def get_chunks_by_document(self, document_id: int) -> List[ChunkSample]:
        """Get all chunks for a specific document"""
        with self.get_connection() as conn:
            cursor = conn.cursor()

            cursor.execute("""
                SELECT e.id, e.document_id, e.chunk_index, e.chunk_start, e.chunk_end,
                       d.title, d.content, d.url
                FROM embeddings e
                JOIN documents d ON e.document_id = d.id
                WHERE e.document_id = ?
                ORDER BY e.chunk_index
            """, (document_id,))

            chunks = []
            for row in cursor.fetchall():
                (chunk_id, doc_id, chunk_index, chunk_start, chunk_end,
                 doc_title, doc_content, doc_url) = row

                chunk_text = doc_content[chunk_start:chunk_end] if doc_content else ""

                chunks.append(ChunkSample(
                    chunk_id=chunk_id,
                    document_id=doc_id,
                    chunk_index=chunk_index,
                    chunk_text=chunk_text,
                    document_title=doc_title or "",
                    document_url=doc_url,
                    chunk_start=chunk_start,
                    chunk_end=chunk_end,
                    parent_content=doc_content or "",
                    embedding_id=chunk_id
                ))

            return chunks

    def save_samples(self, samples: List[ChunkSample], output_path: str = 'data/sampled_chunks.json'):
        """Save samples to JSON file"""
        output_path = Path(output_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)

        # Convert to dict for JSON serialization
        samples_dict = [asdict(sample) for sample in samples]

        with open(output_path, 'w', encoding='utf-8') as f:
            json.dump(samples_dict, f, indent=2, ensure_ascii=False)

        print(f"Saved {len(samples)} chunks to {output_path}")

    def load_samples(self, input_path: str = 'data/sampled_chunks.json') -> List[ChunkSample]:
        """Load samples from JSON file"""
        input_path = Path(input_path)

        if not input_path.exists():
            print(f"Warning: File not found: {input_path}")
            return []

        with open(input_path, 'r', encoding='utf-8') as f:
            samples_dict = json.load(f)

        # Convert dict back to ChunkSample objects
        samples = [ChunkSample(**sample) for sample in samples_dict]

        print(f"Loaded {len(samples)} chunks from {input_path}")
        return samples

    def get_statistics(self) -> Dict:
        """Get statistics about the chunks in the database"""
        with self.get_connection() as conn:
            cursor = conn.cursor()

            stats = {}

            # Total chunks
            cursor.execute("SELECT COUNT(*) FROM embeddings")
            stats['total_chunks'] = cursor.fetchone()[0]

            # Total documents
            cursor.execute("SELECT COUNT(DISTINCT document_id) FROM embeddings")
            stats['total_documents'] = cursor.fetchone()[0]

            # Average chunks per document
            cursor.execute("""
                SELECT AVG(chunk_count) FROM (
                    SELECT COUNT(*) as chunk_count
                    FROM embeddings
                    GROUP BY document_id
                )
            """)
            stats['avg_chunks_per_doc'] = cursor.fetchone()[0] or 0

            # Chunk size statistics
            cursor.execute("""
                SELECT
                    AVG(chunk_end - chunk_start) as avg_size,
                    MIN(chunk_end - chunk_start) as min_size,
                    MAX(chunk_end - chunk_start) as max_size
                FROM embeddings
            """)
            size_stats = cursor.fetchone()
            stats['avg_chunk_size'] = size_stats[0] or 0
            stats['min_chunk_size'] = size_stats[1] or 0
            stats['max_chunk_size'] = size_stats[2] or 0

            return stats

    def print_statistics(self):
        """Print database statistics"""
        stats = self.get_statistics()

        print("\n" + "="*60)
        print("LOCALMIND DATABASE STATISTICS")
        print("="*60)
        print(f"Total chunks: {stats['total_chunks']:,}")
        print(f"Total documents: {stats['total_documents']:,}")
        print(f"Avg chunks per document: {stats['avg_chunks_per_doc']:.1f}")
        print(f"Avg chunk size: {stats['avg_chunk_size']:.0f} chars")
        print(f"Min chunk size: {stats['min_chunk_size']} chars")
        print(f"Max chunk size: {stats['max_chunk_size']} chars")
        print("="*60 + "\n")


def main():
    """Test the chunk sampler"""
    import click

    @click.command()
    @click.option('--db-path', help='Path to localmind.db')
    @click.option('--sample-size', default=10, help='Number of chunks to sample')
    @click.option('--stats', is_flag=True, help='Show database statistics')
    def test(db_path, sample_size, stats):
        """Test chunk sampling functionality"""

        sampler = ChunkSampler(db_path)

        if stats:
            sampler.print_statistics()

        # Sample some chunks
        chunks = sampler.sample_chunks(sample_size=sample_size)

        # Display sample chunks
        for i, chunk in enumerate(chunks[:5], 1):
            print(f"\n--- Chunk {i} ---")
            print(f"Document: {chunk.document_title[:50]}...")
            print(f"Chunk {chunk.chunk_index}: chars {chunk.chunk_start}-{chunk.chunk_end}")
            print(f"Text: {chunk.chunk_text[:200]}...")

        # Save samples
        sampler.save_samples(chunks)

    test()


if __name__ == '__main__':
    main()