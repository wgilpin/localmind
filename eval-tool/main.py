#!/usr/bin/env python3
import click
import json
import shutil
from pathlib import Path
from bookmark_sampler import BookmarkSampler
from query_generator import QueryGenerator
from vector_evaluator import VectorEvaluator
from integrated_sampler import IntegratedSampler
from chunk_sampler import ChunkSampler
from chunk_quality_filter import ChunkQualityFilter
from chunk_query_generator import ChunkQueryGenerator
from chunk_vector_evaluator import ChunkVectorEvaluator

def reset_data():
    """Reset/delete all existing data"""
    directories_to_remove = ['data', 'vector_store_eval', 'results', 'chunk_embeddings_cache']

    for dir_name in directories_to_remove:
        dir_path = Path(dir_name)
        if dir_path.exists():
            click.echo(f"Removing {dir_path}...")
            shutil.rmtree(dir_path)

    click.echo("All data has been reset!")

@click.group()
def cli():
    """Vector Bookmark Search Evaluation Tool"""
    pass

@cli.command()
@click.option('--sample-size', default=200, help='Number of bookmarks to sample')
@click.option('--fetch-content/--no-fetch-content', default=True, help='Fetch actual page content')
@click.option('--output', default='data/sampled_bookmarks.json', help='Output file for samples')
def sample(sample_size, fetch_content, output):
    """Sample bookmarks from Chrome bookmarks file"""

    sampler = BookmarkSampler()

    # Sample bookmarks
    samples = sampler.sample_bookmarks_with_content(
        sample_size=sample_size,
        fetch_content=fetch_content
    )

    # Save samples
    sampler.save_samples(samples, output)

    click.echo(f"Sampled {len(samples)} bookmarks and saved to {output}")

@cli.command()
@click.option('--samples', default='data/sampled_bookmarks.json', help='Input file with sampled bookmarks')
@click.option('--model', default='qwen3:4b', help='Ollama model to use for query generation')
@click.option('--output', default='data/generated_queries.json', help='Output file for queries')
def generate(samples, model, output):
    """Generate search queries for sampled bookmarks"""

    # Load samples
    sampler = BookmarkSampler()
    bookmarks = sampler.load_samples(samples)

    # Generate queries (with incremental saving)
    generator = QueryGenerator(model=model)
    queries_map = generator.generate_queries_for_samples(bookmarks, output_path=output)

    total_queries = sum(len(queries) for queries in queries_map.values())
    click.echo(f"Generated {total_queries} queries for {len(queries_map)} bookmarks")

@cli.command()
@click.option('--samples', default='data/sampled_bookmarks.json', help='Input file with sampled bookmarks')
@click.option('--queries', default='data/generated_queries.json', help='Input file with generated queries')
@click.option('--top-k', default=20, help='Number of top results to retrieve')
@click.option('--embedding-model', default='all-MiniLM-L6-v2', help='Embedding model name')
@click.option('--ollama', is_flag=True, help='Use Ollama for embeddings instead of SentenceTransformers')
@click.option('--ollama-url', default='http://localhost:11434', help='Ollama API URL')
@click.option('--output', default='results/evaluation_results.json', help='Output file for results')
@click.option('--reset', is_flag=True, help='Clear existing embeddings and regenerate')
def evaluate(samples, queries, top_k, embedding_model, ollama, ollama_url, output, reset):
    """Evaluate search performance"""

    # Load samples and queries
    sampler = BookmarkSampler()
    bookmarks = sampler.load_samples(samples)

    generator = QueryGenerator()
    queries_map = generator.load_queries(queries)

    # Initialize evaluator with model-specific directory
    # Clean model name for directory
    safe_model_name = embedding_model.replace('/', '_').replace(':', '_')
    persist_dir = f"./vector_store_eval_{safe_model_name}"

    # Clear existing embeddings if reset flag is set
    if reset:
        import shutil
        from pathlib import Path
        if Path(persist_dir).exists():
            shutil.rmtree(persist_dir)
            click.echo(f"Cleared existing embeddings in {persist_dir}")

    evaluator = VectorEvaluator(
        persist_directory=persist_dir,
        embedding_model=embedding_model,
        use_ollama=ollama,
        ollama_url=ollama_url
    )

    # Index bookmarks
    evaluator.index_bookmarks(bookmarks)

    # Evaluate queries
    results = evaluator.evaluate_queries(queries_map, top_k=top_k)

    # Save results
    evaluator.save_results(results, output)

    # Print summary
    evaluator.print_metrics_summary(results)

@cli.command()
@click.option('--sample-size', default=200, help='Number of bookmarks to sample')
@click.option('--model', default='qwen3:4b', help='Ollama model for query generation')
@click.option('--reset', is_flag=True, help='Reset/delete all existing data and start fresh')
def sample_and_generate(sample_size, model, reset):
    """Sample bookmarks and generate queries incrementally (automatically resumes)"""

    click.echo("="*60)
    click.echo("INTEGRATED SAMPLING AND QUERY GENERATION")
    click.echo("="*60)

    if reset:
        reset_data()

    sampler = IntegratedSampler(model=model)
    samples, queries = sampler.sample_and_generate(sample_size=sample_size)

    click.echo(f"\nCompleted: {len(samples)} samples with {sum(len(q) for q in queries.values())} total queries")

@cli.command()
@click.option('--sample-size', default=200, help='Number of bookmarks to sample')
@click.option('--model', default='qwen3:4b', help='Ollama model for query generation')
@click.option('--top-k', default=20, help='Number of top results to retrieve')
@click.option('--embedding-model', default='all-MiniLM-L6-v2', help='Embedding model name')
@click.option('--ollama', is_flag=True, help='Use Ollama for embeddings instead of SentenceTransformers')
@click.option('--ollama-url', default='http://localhost:11434', help='Ollama API URL')
@click.option('--reset', is_flag=True, help='Reset/delete all existing data and start fresh')
def run_all(sample_size, model, top_k, embedding_model, ollama, ollama_url, reset):
    """Run the complete evaluation pipeline"""

    click.echo("="*60)
    click.echo("VECTOR BOOKMARK EVALUATION PIPELINE")
    click.echo("="*60)

    if reset:
        reset_data()

    # Step 1 & 2: Sample bookmarks and generate queries (integrated)
    click.echo(f"\n[1/3] Checking/generating {sample_size} samples with queries using {model}...")
    sampler = IntegratedSampler(model=model)
    samples, queries_map = sampler.sample_and_generate(sample_size=sample_size)

    # Step 3: Index bookmarks
    click.echo("\n[2/3] Indexing bookmarks in vector store...")
    evaluator = VectorEvaluator(
        persist_directory="./vector_store_eval",
        embedding_model=embedding_model,
        use_ollama=ollama,
        ollama_url=ollama_url
    )
    evaluator.index_bookmarks(samples)

    # Step 4: Evaluate
    click.echo("\n[3/3] Evaluating search performance...")
    results = evaluator.evaluate_queries(queries_map, top_k=top_k)
    evaluator.save_results(results)

    # Print summary
    evaluator.print_metrics_summary(results)

    click.echo("\nEvaluation complete! Results saved to 'results/' directory.")

@cli.command()
def reset():
    """Reset/delete all existing data and start fresh"""
    click.echo("="*60)
    click.echo("RESETTING ALL DATA")
    click.echo("="*60)

    if click.confirm("This will delete all samples, queries, vector store data, and results. Continue?"):
        reset_data()
    else:
        click.echo("Reset cancelled.")

@cli.command()
@click.option('--results', default='results/evaluation_results.json', help='Results file to analyze')
def analyze(results):
    """Analyze and display evaluation results"""

    with open(results, 'r') as f:
        data = json.load(f)

    evaluator = VectorEvaluator()
    evaluator.print_metrics_summary(data)

    # Additional analysis
    click.echo("\nPer-Query Analysis:")
    click.echo("-" * 40)

    # Find worst performing queries
    worst_queries = []
    for eval in data['evaluations']:
        for query_result in eval['queries']:
            if not query_result['found']:
                worst_queries.append({
                    'bookmark_id': eval['bookmark_id'],
                    'query': query_result['query']
                })

    if worst_queries:
        click.echo(f"\nQueries that failed to find their bookmark ({len(worst_queries)} total):")
        for i, q in enumerate(worst_queries[:10], 1):
            click.echo(f"{i}. [{q['bookmark_id']}] {q['query']}")

        if len(worst_queries) > 10:
            click.echo(f"... and {len(worst_queries) - 10} more")

# ============================================
# CHUNK-BASED EVALUATION COMMANDS
# ============================================

@cli.command()
@click.option('--sample-size', default=200, help='Number of chunks to sample')
@click.option('--db-path', help='Path to localmind.db (auto-detected if not provided)')
@click.option('--min-chunk-length', default=50, help='Minimum chunk text length')
@click.option('--output', default='data/sampled_chunks.json', help='Output file')
@click.option('--stats', is_flag=True, help='Show database statistics')
def sample_chunks(sample_size, db_path, min_chunk_length, output, stats):
    """Sample chunks from localmind-rs database"""

    sampler = ChunkSampler(db_path)

    if stats:
        sampler.print_statistics()

    # Sample chunks
    chunks = sampler.sample_chunks(
        sample_size=sample_size,
        min_chunk_length=min_chunk_length
    )

    # Save samples
    sampler.save_samples(chunks, output)

    click.echo(f"[OK] Sampled {len(chunks)} chunks and saved to {output}")

@cli.command()
@click.option('--chunks', default='data/sampled_chunks.json', help='Input chunks file')
@click.option('--model', default='qwen3:4b', help='LM Studio model for quality assessment')
@click.option('--min-confidence', default=0.6, help='Minimum confidence score')
@click.option('--quick', is_flag=True, help='Apply quick heuristic filters first')
def filter_chunks(chunks, model, min_confidence, quick):
    """Filter chunks for quality using LLM"""

    # Load chunks
    sampler = ChunkSampler()
    chunk_samples = sampler.load_samples(chunks)

    if not chunk_samples:
        click.echo("No chunks to filter. Run chunk sampling first.")
        return

    # Initialize filter
    filter = ChunkQualityFilter(model=model)

    # Apply quick filter if requested
    if quick:
        chunk_samples, quick_rejected = filter.quick_filter(chunk_samples)
        click.echo(f"⚡ Quick filter rejected {len(quick_rejected)} chunks")

    # Filter chunks
    suitable, unsuitable = filter.filter_chunks(chunk_samples, min_confidence=min_confidence)

    # Save results
    filter.save_filtered_chunks(suitable, unsuitable)

    # Analyze rejections
    filter.analyze_rejections(unsuitable, top_n=5)

    click.echo(f"[OK] Filtered to {len(suitable)} suitable chunks")

@cli.command()
@click.option('--chunks', default='data/quality_chunks.json', help='Quality chunks file')
@click.option('--model', default='qwen3:4b', help='LM Studio model for term generation')
@click.option('--num-terms', default=3, help='Number of search terms per chunk')
@click.option('--batch-size', default=5, help='Batch size for processing')
def generate_chunk_terms(chunks, model, num_terms, batch_size):
    """Generate search terms for chunks"""

    # Load quality chunks
    filter = ChunkQualityFilter()
    quality_chunks = filter.load_filtered_chunks(chunks)

    if not quality_chunks:
        click.echo("No quality chunks found. Run quality filtering first.")
        return

    # Initialize generator
    generator = ChunkQueryGenerator(model=model)

    # Generate terms
    if batch_size > 1:
        terms_dict = generator.batch_generate(quality_chunks, batch_size=batch_size)
    else:
        terms_dict = generator.generate_for_chunks(quality_chunks, num_terms_per_chunk=num_terms)

    # Save results
    generator.save_search_terms(terms_dict)

    # Analyze terms
    generator.analyze_search_terms(terms_dict, top_n=10)

    click.echo(f"[OK] Generated search terms for {len(terms_dict)} chunks")

@cli.command()
@click.option('--chunks', default='data/quality_chunks.json', help='Quality chunks file')
@click.option('--terms', default='data/chunk_terms.json', help='Search terms file')
@click.option('--embedding-model', default='all-MiniLM-L6-v2', help='Embedding model')
@click.option('--ollama', is_flag=True, help='Use Ollama for embeddings')
@click.option('--top-k', default=10, help='Top-K for evaluation')
@click.option('--output', default='results/chunk_evaluation.json', help='Output file')
def evaluate_chunks(chunks, terms, embedding_model, ollama, top_k, output):
    """Evaluate chunk-based search performance"""

    # Load data
    filter = ChunkQualityFilter()
    quality_chunks = filter.load_filtered_chunks(chunks)

    generator = ChunkQueryGenerator()
    terms_dict = generator.load_search_terms(terms)

    if not quality_chunks or not terms_dict:
        click.echo("Missing data. Run quality filtering and term generation first.")
        return

    # Initialize evaluator
    evaluator = ChunkVectorEvaluator(
        embedding_model=embedding_model,
        use_ollama=ollama
    )

    # Index chunks
    evaluator.index_chunks(quality_chunks)

    # Evaluate
    report = evaluator.evaluate_all_chunks(quality_chunks, terms_dict, top_k=top_k)

    # Save report
    evaluator.save_report(report, output)

    # Save CSV for model comparison
    evaluator.save_csv_summary(report)
    evaluator.save_detailed_csv(report)

    # Analyze failures
    evaluator.analyze_failures(report, top_n=5)

    click.echo(f"[OK] Evaluation complete! Results saved to {output}")
    click.echo("[OK] CSV files saved for model comparison")

@cli.command()
@click.option('--sample-size', default=200, help='Number of chunks to sample')
@click.option('--db-path', help='Path to localmind.db')
@click.option('--llm-model', default='qwen3:4b', help='LM Studio model for quality/terms')
@click.option('--embedding-model', default='all-MiniLM-L6-v2', help='Embedding model')
@click.option('--ollama', is_flag=True, help='Use Ollama for embeddings')
@click.option('--lmstudio', is_flag=True, help='Use LM Studio for embeddings')
@click.option('--sentence-transformers', is_flag=True, help='Use SentenceTransformers for embeddings (default)')
@click.option('--top-k', default=10, help='Top-K for evaluation')
@click.option('--reset', is_flag=True, help='Reset all data first')
@click.option('--skip-quality', is_flag=True, help='Skip LLM quality filtering, use heuristics only')
def run_chunk_pipeline(sample_size, db_path, llm_model, embedding_model, ollama, lmstudio, sentence_transformers, top_k, reset, skip_quality):
    """Run complete chunk-based evaluation pipeline"""

    # Validate embedding backend options
    embedding_backends = [ollama, lmstudio, sentence_transformers]
    if sum(embedding_backends) > 1:
        click.echo("Error: Only one embedding backend can be specified (--ollama, --lmstudio, or --sentence-transformers)")
        return

    # Default to SentenceTransformers if no backend specified
    if not any(embedding_backends):
        sentence_transformers = True

    click.echo("="*60)
    click.echo("CHUNK-BASED EVALUATION PIPELINE")
    click.echo("="*60)

    # Show which embedding backend is being used
    if ollama:
        click.echo(f"Using Ollama embeddings with model: {embedding_model}")
    elif lmstudio:
        click.echo(f"Using LM Studio embeddings with model: {embedding_model}")
    else:
        click.echo(f"Using SentenceTransformers with model: {embedding_model}")

    if reset:
        reset_data()

    # Granular cache check - check each component separately
    chunks_file = Path('data/sampled_chunks.json')
    quality_file = Path('data/quality_chunks.json')
    terms_file = Path('data/chunk_terms.json')

    # Check cached components
    cached_chunks = None
    cached_quality = None
    cached_terms = None

    if not reset:
        # Check sampled chunks
        if chunks_file.exists():
            sampler = ChunkSampler()
            existing_chunks = sampler.load_samples()
            if len(existing_chunks) >= sample_size:
                cached_chunks = existing_chunks
                click.echo(f"✅ Found {len(existing_chunks)} cached chunks (need {sample_size})")
            else:
                click.echo(f"⚠️  Cached chunks insufficient: {len(existing_chunks)} < {sample_size}")

        # Check quality chunks
        if quality_file.exists():
            filter = ChunkQualityFilter(model=llm_model)
            suitable = filter.load_filtered_chunks()
            if len(suitable) > 0:
                cached_quality = suitable
                click.echo(f"✅ Found {len(suitable)} cached quality chunks")
            else:
                click.echo(f"⚠️  No cached quality chunks found")

        # Check search terms
        if terms_file.exists():
            generator = ChunkQueryGenerator(model=llm_model)
            terms_dict = generator.load_search_terms()
            # Terms should match quality chunks count (or at least be substantial)
            if len(terms_dict) > 0 and cached_quality and len(terms_dict) >= len(cached_quality) * 0.9:
                cached_terms = terms_dict
                click.echo(f"✅ Found {len(terms_dict)} cached search term sets")
            else:
                click.echo(f"⚠️  Search terms insufficient: {len(terms_dict)} terms vs {len(cached_quality) if cached_quality else 0} quality chunks")

    # Check if we can skip directly to embeddings
    if cached_chunks and cached_quality and cached_terms:
        click.echo(f"\n🎯 All cached data valid - jumping to embeddings/evaluation!")
        chunks = cached_chunks
        suitable = cached_quality
        terms_dict = cached_terms
    else:
        click.echo(f"\n🔄 Need to regenerate some components...")
        suitable = cached_quality  # Keep if valid
        terms_dict = cached_terms  # Keep if valid

    # Run only the preprocessing steps that need regeneration

    # Step 1: Handle chunks (use cached if available)
    if cached_chunks:
        click.echo(f"\n[1/5] ✅ Using cached {len(cached_chunks)} chunks")
        chunks = cached_chunks
    else:
        if chunks_file.exists() and not reset:
            sampler = ChunkSampler()
            existing_chunks = sampler.load_samples()
            if len(existing_chunks) >= sample_size:
                click.echo(f"\n[1/5] Using cached {len(existing_chunks)} chunks")
                chunks = existing_chunks
            else:
                click.echo(f"\n[1/5] Re-sampling {sample_size} chunks (cache has {len(existing_chunks)})...")
                sampler = ChunkSampler(db_path)
                sampler.print_statistics()
                chunks = sampler.sample_chunks(sample_size=sample_size)
                sampler.save_samples(chunks)
        else:
            click.echo(f"\n[1/5] Sampling {sample_size} chunks from database...")
            sampler = ChunkSampler(db_path)
            sampler.print_statistics()
            chunks = sampler.sample_chunks(sample_size=sample_size)
            sampler.save_samples(chunks)

    # Step 2: Handle quality filtering (use cached if available)
    if cached_quality:
        click.echo(f"\n[2/5] ✅ Using cached {len(cached_quality)} quality chunks")
        suitable = cached_quality
    else:
        if quality_file.exists() and not reset:
            click.echo(f"\n[2/5] Using cached quality-filtered chunks...")
            filter = ChunkQualityFilter(model=llm_model)
            suitable = filter.load_filtered_chunks()
            click.echo(f"  Loaded {len(suitable)} cached quality chunks")
        else:
            if skip_quality:
                click.echo(f"\n[2/5] Applying heuristic quality filters only (skipping LLM)...")
                filter = ChunkQualityFilter(model=llm_model)
                suitable_chunks, rejected = filter.quick_filter(chunks)
                click.echo(f"  Heuristic filter: {len(suitable_chunks)} passed, {len(rejected)} rejected")

                # Show rejection summary for heuristic-only filtering
                filter.print_rejection_summary(len(chunks))

                # Convert to QualityChunkSample format for compatibility
                from chunk_quality_filter import QualityChunkSample
                suitable = []
                for chunk in suitable_chunks:
                    suitable.append(QualityChunkSample(
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
                        quality_status='SUITABLE',
                        quality_reason='Passed heuristic filters',
                        confidence_score=0.8
                    ))
                unsuitable = []
                filter.save_filtered_chunks(suitable, unsuitable)
            else:
                click.echo(f"\n[2/5] Filtering chunks for quality using {llm_model}...")
                filter = ChunkQualityFilter(model=llm_model)

                # Apply quick heuristics first
                chunks, quick_rejected = filter.quick_filter(chunks)
                click.echo(f"  Quick filter: {len(chunks)} passed, {len(quick_rejected)} rejected")

                suitable, unsuitable = filter.filter_chunks(chunks)
                filter.save_filtered_chunks(suitable, unsuitable)

                # Show rejection summary
                filter.print_rejection_summary(len(chunks))

    # Step 3: Handle search terms (use cached if available)
    if cached_terms:
        click.echo(f"\n[3/5] ✅ Using cached {len(cached_terms)} search term sets")
        terms_dict = cached_terms
    else:
        terms_file = Path('data/chunk_terms.json')
        if terms_file.exists() and not reset:
            click.echo(f"\n[3/5] Using cached search terms...")
            generator = ChunkQueryGenerator(model=llm_model)
            terms_dict = generator.load_search_terms()

            # Verify terms match current suitable chunks
            suitable_chunk_ids = {chunk.chunk_id for chunk in suitable}
            cached_chunk_ids = set(terms_dict.keys())

            if suitable_chunk_ids == cached_chunk_ids:
                click.echo(f"  Loaded {len(terms_dict)} cached search term sets")
            else:
                click.echo(f"  Cache mismatch, regenerating search terms...")
                terms_dict = generator.batch_generate(suitable, batch_size=5)
                generator.save_search_terms(terms_dict)
        else:
            click.echo(f"\n[3/5] Generating search terms for {len(suitable)} chunks...")
            generator = ChunkQueryGenerator(model=llm_model)
            terms_dict = generator.batch_generate(suitable, batch_size=5)
            generator.save_search_terms(terms_dict)

    # Step 4: Index chunks
    click.echo(f"\n[4/5] Indexing chunks with {embedding_model}...")
    evaluator = ChunkVectorEvaluator(
        embedding_model=embedding_model,
        use_ollama=ollama,
        use_lmstudio=lmstudio
    )
    evaluator.index_chunks(suitable)

    # Step 5: Evaluate
    click.echo(f"\n[5/5] Evaluating search performance (top-{top_k})...")
    report = evaluator.evaluate_all_chunks(suitable, terms_dict, top_k=top_k)
    evaluator.save_report(report)

    # Save CSV for model comparison
    evaluator.save_csv_summary(report)
    evaluator.save_detailed_csv(report)

    # Final summary
    click.echo("\n" + "="*60)
    click.echo("PIPELINE COMPLETE!")
    click.echo("="*60)
    click.echo(f"[OK] Processed {len(suitable)} quality chunks")
    click.echo(f"[OK] Generated {sum(len(t.search_terms) for t in terms_dict.values())} search terms")
    click.echo(f"[OK] Mean Hit Rate: {report.overall_metrics['mean_hit_rate']:.2%}")
    click.echo(f"[OK] Results saved to results/chunk_evaluation.json")
    click.echo(f"[OK] CSV comparison saved to results/model_comparison.csv")

@cli.command()
@click.option('--reports', multiple=True, help='Evaluation report files to compare')
def compare_chunk_models(reports):
    """Compare multiple chunk evaluation reports"""

    if len(reports) < 2:
        click.echo("Please provide at least 2 report files to compare.")
        return

    evaluator = ChunkVectorEvaluator()
    loaded_reports = []

    for report_path in reports:
        report = evaluator.load_report(report_path)
        if report:
            loaded_reports.append(report)

    if len(loaded_reports) < 2:
        click.echo("Could not load enough reports for comparison.")
        return

    evaluator.compare_models(loaded_reports)

@cli.command()
@click.option('--report', default='results/chunk_evaluation.json', help='Evaluation report to analyze')
def analyze_chunk_results(report):
    """Analyze chunk evaluation results in detail"""

    evaluator = ChunkVectorEvaluator()
    loaded_report = evaluator.load_report(report)

    if not loaded_report:
        click.echo(f"Could not load report from {report}")
        return

    # Print metrics
    evaluator.print_metrics(loaded_report)

    # Analyze failures
    evaluator.analyze_failures(loaded_report, top_n=10)

if __name__ == '__main__':
    cli()