#!/usr/bin/env python3
import click
import json
import shutil
from pathlib import Path
from bookmark_sampler import BookmarkSampler
from query_generator import QueryGenerator
from chroma_evaluator import ChromaEvaluator
from integrated_sampler import IntegratedSampler

def reset_data():
    """Reset/delete all existing data"""
    directories_to_remove = ['data', 'chroma_db_eval', 'results']
    
    for dir_name in directories_to_remove:
        dir_path = Path(dir_name)
        if dir_path.exists():
            click.echo(f"Removing {dir_path}...")
            shutil.rmtree(dir_path)
    
    click.echo("All data has been reset!")

@click.group()
def cli():
    """ChromaDB Bookmark Search Evaluation Tool"""
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
@click.option('--embedding-model', default='all-MiniLM-L6-v2', help='Sentence transformer model for embeddings')
@click.option('--output', default='results/evaluation_results.json', help='Output file for results')
def evaluate(samples, queries, top_k, embedding_model, output):
    """Evaluate search performance"""
    
    # Load samples and queries
    sampler = BookmarkSampler()
    bookmarks = sampler.load_samples(samples)
    
    generator = QueryGenerator()
    queries_map = generator.load_queries(queries)
    
    # Initialize evaluator
    evaluator = ChromaEvaluator(
        persist_directory="./chroma_db_eval",
        embedding_model=embedding_model
    )
    
    # Create collection and index bookmarks
    evaluator.create_collection()
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
@click.option('--embedding-model', default='all-MiniLM-L6-v2', help='Sentence transformer model')
@click.option('--reset', is_flag=True, help='Reset/delete all existing data and start fresh')
def run_all(sample_size, model, top_k, embedding_model, reset):
    """Run the complete evaluation pipeline"""
    
    click.echo("="*60)
    click.echo("CHROMADB BOOKMARK EVALUATION PIPELINE")
    click.echo("="*60)
    
    if reset:
        reset_data()
    
    # Step 1 & 2: Sample bookmarks and generate queries (integrated)
    click.echo(f"\n[1/3] Checking/generating {sample_size} samples with queries using {model}...")
    sampler = IntegratedSampler(model=model)
    samples, queries_map = sampler.sample_and_generate(sample_size=sample_size)
    
    # Step 3: Index bookmarks
    click.echo("\n[2/3] Indexing bookmarks in ChromaDB...")
    evaluator = ChromaEvaluator(
        persist_directory="./chroma_db_eval",
        embedding_model=embedding_model
    )
    evaluator.create_collection()
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
    
    if click.confirm("This will delete all samples, queries, ChromaDB data, and results. Continue?"):
        reset_data()
    else:
        click.echo("Reset cancelled.")

@cli.command()
@click.option('--results', default='results/evaluation_results.json', help='Results file to analyze')
def analyze(results):
    """Analyze and display evaluation results"""
    
    with open(results, 'r') as f:
        data = json.load(f)
    
    evaluator = ChromaEvaluator()
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

if __name__ == '__main__':
    cli()