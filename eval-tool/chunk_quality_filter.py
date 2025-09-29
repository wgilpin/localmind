#!/usr/bin/env python3
"""
Chunk Quality Filter Module
Uses LLM to assess if chunks contain good quality English text suitable for evaluation.
"""

import json
from pathlib import Path
from typing import List, Dict, Optional, Tuple
from dataclasses import dataclass, asdict
from chunk_sampler import ChunkSample
import time
from tqdm import tqdm
from lmstudio_client import LMStudioClient


@dataclass
class QualityAssessment:
    """Quality assessment result for a chunk"""
    chunk_id: int
    quality_status: str  # SUITABLE or UNSUITABLE
    quality_reason: str
    confidence_score: float  # 0-1 confidence in assessment


@dataclass
class QualityChunkSample(ChunkSample):
    """Chunk sample with quality assessment"""
    quality_status: str
    quality_reason: str
    confidence_score: float


class ChunkQualityFilter:
    """Filter chunks based on LLM quality assessment"""

    def __init__(self, model: str = "qwen3:4b", lmstudio_url: str = "http://localhost:1234"):
        """Initialize with LM Studio model"""
        self.model = model
        self.lmstudio_client = LMStudioClient(base_url=lmstudio_url)
        self.assessment_cache = {}  # Cache assessments to avoid re-processing

        # Statistics tracking
        self.stats = {
            'total_assessed': 0,
            'suitable': 0,
            'unsuitable': 0,
            'errors': 0
        }

        # Track rejection reasons
        self.rejection_counts = {}

    def assess_chunk_quality(self, chunk: ChunkSample) -> QualityAssessment:
        """
        Assess if a chunk contains quality English text

        Args:
            chunk: ChunkSample to assess

        Returns:
            QualityAssessment with status and reasoning
        """
        # Check cache first
        cache_key = f"{chunk.chunk_id}_{chunk.chunk_text[:50]}"
        if cache_key in self.assessment_cache:
            return self.assessment_cache[cache_key]

        prompt = f"""Analyze this text chunk and determine if it contains good quality English text suitable for search evaluation.

Chunk: "{chunk.chunk_text}"

Respond with only "SUITABLE" or "UNSUITABLE"

SUITABLE chunks have:
- Clear, meaningful English text
- Complete sentences or coherent fragments
- Searchable content (facts, concepts, descriptions)
- At least some informational value

UNSUITABLE chunks have:
- Primarily code or programming content
- Random characters, URLs, or excessive formatting
- Non-English text (except for occasional terms)
- No meaningful searchable content
- Truncated mid-word or nonsensical

Respond with only "SUITABLE" or "UNSUITABLE"
"""

        def try_llm_assessment(attempt=1):
            """Try LLM assessment with retry logic"""
            response = self.lmstudio_client.chat(
                model=self.model,
                messages=[
                    {
                        'role': 'user',
                        'content': prompt
                    }
                ],
                temperature=0.3,  # Lower temperature for consistent classification
                top_p=0.9,
                max_tokens=100  # Limit response length
            )

            # Parse response
            response_text = response['message']['content'].strip().upper()

            # Simple parsing - just look for SUITABLE or UNSUITABLE
            if 'SUITABLE' in response_text and 'UNSUITABLE' not in response_text:
                return 'SUITABLE', "LLM assessed as suitable", 0.9
            elif 'UNSUITABLE' in response_text:
                return 'UNSUITABLE', "LLM assessed as unsuitable", 0.9
            else:
                # Response is unclear
                if attempt == 1:
                    print(f"   Unclear response (attempt {attempt}): '{response_text[:30]}...', retrying...")
                    return try_llm_assessment(attempt=2)
                else:
                    # After retry, still unclear - mark as unsuitable
                    return 'UNSUITABLE', f"Unclear after retry: {response_text[:50]}", 0.3

        try:
            status, reason, confidence = try_llm_assessment()

            assessment = QualityAssessment(
                chunk_id=chunk.chunk_id,
                quality_status=status,
                quality_reason=reason,
                confidence_score=confidence
            )

            # Update stats
            self.stats['total_assessed'] += 1
            if status == 'SUITABLE':
                self.stats['suitable'] += 1
            else:
                self.stats['unsuitable'] += 1

            # Cache result
            self.assessment_cache[cache_key] = assessment
            return assessment

        except Exception as e:
            print(f" Error assessing chunk {chunk.chunk_id}: {e}")
            self.stats['errors'] += 1

            # Return unsuitable on error
            return QualityAssessment(
                chunk_id=chunk.chunk_id,
                quality_status='UNSUITABLE',
                quality_reason=f"Assessment error: {str(e)}",
                confidence_score=0.0
            )

    def filter_chunks(self, chunks: List[ChunkSample],
                     min_confidence: float = 0.6,
                     show_progress: bool = True) -> Tuple[List[QualityChunkSample], List[QualityChunkSample]]:
        """
        Filter chunks based on quality assessment

        Args:
            chunks: List of chunks to filter
            min_confidence: Minimum confidence score to accept assessment
            show_progress: Whether to show progress

        Returns:
            Tuple of (suitable_chunks, unsuitable_chunks)
        """
        suitable = []
        unsuitable = []

        print(f"\n Assessing quality of {len(chunks)} chunks using {self.model}...")

        # Use tqdm for progress bar
        chunk_iterator = tqdm(chunks, desc="Assessing quality", unit="chunk") if show_progress else chunks

        for chunk in chunk_iterator:
            assessment = self.assess_chunk_quality(chunk)

            # Create quality chunk sample
            quality_chunk = QualityChunkSample(
                chunk_id=chunk.chunk_id,
                document_id=chunk.document_id,
                chunk_index=chunk.chunk_index,
                chunk_text=chunk.chunk_text,
                document_title=chunk.document_title,
                document_url=chunk.document_url,
                chunk_start=chunk.chunk_start,
                chunk_end=chunk.chunk_end,
                parent_content=chunk.parent_content,
                embedding_id=chunk.embedding_id,
                quality_status=assessment.quality_status,
                quality_reason=assessment.quality_reason,
                confidence_score=assessment.confidence_score
            )

            if assessment.quality_status == 'SUITABLE' and assessment.confidence_score >= min_confidence:
                suitable.append(quality_chunk)
            else:
                unsuitable.append(quality_chunk)

                # Track LLM rejection reason
                if "Unclear after retry" in assessment.quality_reason:
                    self.rejection_counts["Unclear LLM response after retry"] = self.rejection_counts.get("Unclear LLM response after retry", 0) + 1
                else:
                    self.rejection_counts["LLM assessed as unsuitable"] = self.rejection_counts.get("LLM assessed as unsuitable", 0) + 1

                # Log LLM rejections
                preview = chunk.chunk_text[:200].replace('\n', '\\n').replace('\r', '\\r')
                tqdm.write(f" LLM REJECTED Chunk {chunk.chunk_id}: {assessment.quality_reason}")
                tqdm.write(f"   Text: {preview}...")

            # Small delay to avoid overwhelming LM Studio
            time.sleep(0.1)

        self.print_statistics()
        return suitable, unsuitable

    def batch_filter(self, chunks: List[ChunkSample],
                    batch_size: int = 5) -> Tuple[List[QualityChunkSample], List[QualityChunkSample]]:
        """
        Filter chunks in batches for efficiency

        Args:
            chunks: List of chunks to filter
            batch_size: Number of chunks to assess together

        Returns:
            Tuple of (suitable_chunks, unsuitable_chunks)
        """
        suitable = []
        unsuitable = []

        print(f"\n Batch assessing {len(chunks)} chunks (batch size: {batch_size})...")

        for i in range(0, len(chunks), batch_size):
            batch = chunks[i:i + batch_size]
            batch_suitable, batch_unsuitable = self.filter_chunks(batch, show_progress=False)
            suitable.extend(batch_suitable)
            unsuitable.extend(batch_unsuitable)

            print(f"  Batch {i//batch_size + 1}: {len(batch_suitable)} suitable, {len(batch_unsuitable)} unsuitable")

        self.print_statistics()
        return suitable, unsuitable

    def apply_heuristic_filters(self, chunk: ChunkSample) -> Optional[str]:
        """
        Apply quick heuristic filters before LLM assessment

        Args:
            chunk: ChunkSample to check

        Returns:
            Rejection reason if chunk fails heuristics, None if passes
        """
        text = chunk.chunk_text

        # Check minimum length
        if len(text) < 50:
            return f"Text too short ({len(text)} < 50 chars)"

        # Check word count
        words = text.split()
        if len(words) < 10:
            return f"Too few words ({len(words)} < 10)"

        # Check for excessive code indicators
        code_indicators = ['function', 'const', 'var', 'import', 'export', 'class', 'def', 'return']
        code_count = sum(1 for indicator in code_indicators if indicator in text.lower())
        code_ratio = code_count / len(words) if words else 0
        if code_ratio > 0.2:  # More than 20% code keywords
            return f"High code content ratio ({code_ratio:.1%} > 20%)"

        # Check for excessive special characters
        special_chars = sum(1 for c in text if c in '{}[]()<>;:=')
        special_ratio = special_chars / len(text) if text else 0
        if special_ratio > 0.2:  # More than 20% special chars
            return f"Excessive special characters ({special_ratio:.1%} > 20%)"

        # Check for URL/path dominance
        slash_count = text.count('/')
        http_count = text.count('http')
        if slash_count > 10 or http_count > 3:
            return f"URL/path heavy content ({slash_count} slashes, {http_count} http)"

        return None  # Passes all heuristics

    def quick_filter(self, chunks: List[ChunkSample]) -> Tuple[List[ChunkSample], List[Tuple[ChunkSample, str]]]:
        """
        Quick heuristic filtering before LLM assessment

        Args:
            chunks: List of chunks to filter

        Returns:
            Tuple of (chunks_for_llm, rejected_chunks_with_reasons)
        """
        chunks_for_llm = []
        rejected = []

        print(f"\n Applying heuristic filters to {len(chunks)} chunks...")

        for chunk in chunks:
            rejection_reason = self.apply_heuristic_filters(chunk)
            if rejection_reason:
                rejected.append((chunk, rejection_reason))

                # Track rejection reason
                reason_key = rejection_reason.split('(')[0].strip()
                self.rejection_counts[reason_key] = self.rejection_counts.get(reason_key, 0) + 1

                # Log rejected chunk details
                preview = chunk.chunk_text[:100].replace('\n', '\\n').replace('\r', '\\r')
                print(f" REJECTED Chunk {chunk.chunk_id}: {rejection_reason}")
                print(f"   Text: {preview}...")
                print(f"   Length: {len(chunk.chunk_text)} chars, Words: {len(chunk.chunk_text.split())}")
                print()
            else:
                chunks_for_llm.append(chunk)
                # Log accepted chunk summary
                preview = chunk.chunk_text[:60].replace('\n', '\\n').replace('\r', '\\r')
                print(f" ACCEPTED Chunk {chunk.chunk_id}: {preview}...")

        print(f"\n Heuristic Filter Results:")
        print(f"   Passed: {len(chunks_for_llm)}")
        print(f"   Rejected: {len(rejected)}")

        # Show rejection reason summary
        if rejected:
            reason_counts = {}
            for _, reason in rejected:
                key = reason.split('(')[0].strip()  # Get reason without details
                reason_counts[key] = reason_counts.get(key, 0) + 1

            print(f"\n   Rejection reasons:")
            for reason, count in sorted(reason_counts.items(), key=lambda x: x[1], reverse=True):
                print(f"     - {reason}: {count} chunks")

        return chunks_for_llm, rejected

    def save_filtered_chunks(self, suitable: List[QualityChunkSample],
                            unsuitable: List[QualityChunkSample],
                            output_dir: str = 'data'):
        """Save filtered chunks to JSON files"""
        output_dir = Path(output_dir)
        output_dir.mkdir(parents=True, exist_ok=True)

        # Save suitable chunks
        suitable_path = output_dir / 'quality_chunks.json'
        suitable_dict = [asdict(chunk) for chunk in suitable]
        with open(suitable_path, 'w', encoding='utf-8') as f:
            json.dump(suitable_dict, f, indent=2, ensure_ascii=False)
        print(f" Saved {len(suitable)} suitable chunks to {suitable_path}")

        # Save unsuitable chunks for analysis
        unsuitable_path = output_dir / 'rejected_chunks.json'
        unsuitable_dict = [asdict(chunk) for chunk in unsuitable]
        with open(unsuitable_path, 'w', encoding='utf-8') as f:
            json.dump(unsuitable_dict, f, indent=2, ensure_ascii=False)
        print(f" Saved {len(unsuitable)} unsuitable chunks to {unsuitable_path}")

    def load_filtered_chunks(self, input_path: str = 'data/quality_chunks.json') -> List[QualityChunkSample]:
        """Load filtered chunks from JSON file"""
        input_path = Path(input_path)

        if not input_path.exists():
            print(f" File not found: {input_path}")
            return []

        with open(input_path, 'r', encoding='utf-8') as f:
            chunks_dict = json.load(f)

        chunks = [QualityChunkSample(**chunk) for chunk in chunks_dict]
        print(f" Loaded {len(chunks)} quality chunks from {input_path}")
        return chunks

    def print_statistics(self):
        """Print assessment statistics"""
        total = self.stats['total_assessed']
        if total == 0:
            return

        print("\n" + "="*60)
        print(" QUALITY ASSESSMENT STATISTICS")
        print("="*60)
        print(f"Total assessed: {total}")
        print(f"Suitable: {self.stats['suitable']} ({100*self.stats['suitable']/total:.1f}%)")
        print(f"Unsuitable: {self.stats['unsuitable']} ({100*self.stats['unsuitable']/total:.1f}%)")
        print(f"Errors: {self.stats['errors']}")
        print("="*60 + "\n")

    def print_rejection_summary(self, total_chunks: int):
        """Print summary of all rejection reasons"""
        if not self.rejection_counts:
            return

        print("\n" + "="*60)
        print(" REJECTION REASONS SUMMARY")
        print("="*60)
        print(f"Total chunks processed: {total_chunks}")
        print(f"Total rejected: {sum(self.rejection_counts.values())}")
        print(f"Total accepted: {total_chunks - sum(self.rejection_counts.values())}")
        print()
        print("Rejection breakdown:")

        # Sort by count (highest first)
        sorted_reasons = sorted(self.rejection_counts.items(), key=lambda x: x[1], reverse=True)

        for reason, count in sorted_reasons:
            percentage = (count / total_chunks) * 100
            print(f"  - {reason}: {count} chunks ({percentage:.1f}%)")

        print("="*60 + "\n")

    def analyze_rejections(self, unsuitable: List[QualityChunkSample], top_n: int = 10):
        """Analyze and display rejection reasons"""
        print("\n" + "="*60)
        print(" REJECTION ANALYSIS")
        print("="*60)

        # Count rejection reasons
        reason_counts = {}
        for chunk in unsuitable:
            reason_key = chunk.quality_reason.split(':')[0]  # Get main reason
            reason_counts[reason_key] = reason_counts.get(reason_key, 0) + 1

        # Sort by count
        sorted_reasons = sorted(reason_counts.items(), key=lambda x: x[1], reverse=True)

        print("Top rejection reasons:")
        for reason, count in sorted_reasons[:top_n]:
            print(f"  - {reason}: {count} chunks")

        # Show sample rejected chunks
        print(f"\nSample rejected chunks (first 3):")
        for chunk in unsuitable[:3]:
            print(f"\n  Chunk {chunk.chunk_id}:")
            print(f"  Text: {chunk.chunk_text[:100]}...")
            print(f"  Reason: {chunk.quality_reason}")

        print("="*60 + "\n")


def main():
    """Test the quality filter"""
    import click

    @click.command()
    @click.option('--chunks', default='data/sampled_chunks.json', help='Input chunks file')
    @click.option('--model', default='qwen3:4b', help='Ollama model for assessment')
    @click.option('--quick', is_flag=True, help='Use quick heuristic filtering first')
    @click.option('--analyze', is_flag=True, help='Analyze rejection reasons')
    def test(chunks, model, quick, analyze):
        """Test chunk quality filtering"""

        from chunk_sampler import ChunkSampler

        # Load chunks
        sampler = ChunkSampler()
        chunk_samples = sampler.load_samples(chunks)

        if not chunk_samples:
            print("No chunks to filter. Run chunk sampling first.")
            return

        # Initialize filter
        filter = ChunkQualityFilter(model=model)

        # Apply quick filter if requested
        if quick:
            chunk_samples, quick_rejected = filter.quick_filter(chunk_samples)
            print(f"Quick filter rejected {len(quick_rejected)} chunks")

        # Filter chunks
        suitable, unsuitable = filter.filter_chunks(chunk_samples[:20])  # Test with first 20

        # Save results
        filter.save_filtered_chunks(suitable, unsuitable)

        # Analyze if requested
        if analyze:
            filter.analyze_rejections(unsuitable)

    test()


if __name__ == '__main__':
    main()