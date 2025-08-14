import ollama
import json
from pathlib import Path
from typing import List, Dict, Any
from tqdm import tqdm


class QueryGenerator:
    def __init__(self, model: str = "qwen3:4b"):
        self.model = model
        self.client = ollama.Client()

    def _extract_short_query(self, title: str, url: str) -> str:
        """Extract a 1-2 word search query from title or URL"""
        import re
        from urllib.parse import urlparse

        # Common words to skip (expanded list)
        stop_words = {
            "the",
            "a",
            "an",
            "and",
            "or",
            "but",
            "in",
            "on",
            "at",
            "to",
            "for",
            "of",
            "with",
            "by",
            "from",
            "up",
            "about",
            "into",
            "through",
            "during",
            "how",
            "when",
            "where",
            "why",
            "what",
            "which",
            "who",
            "whom",
            "whose",
            "is",
            "are",
            "was",
            "were",
            "be",
            "been",
            "being",
            "have",
            "has",
            "had",
            "do",
            "does",
            "did",
            "will",
            "would",
            "could",
            "should",
            "may",
            "might",
            "must",
            "can",
            "need",
            "dare",
            "ought",
            "shall",
            "uses",
            "using",
            "used",
            "get",
            "gets",
            "getting",
            "got",
            "gotten",
            "make",
            "made",
            "making",
            "your",
            "our",
            "my",
            "his",
            "her",
            "its",
            "their",
            "this",
            "that",
            "these",
            "those",
            "all",
            "any",
            "some",
            "no",
            "not",
            "only",
            "just",
            "very",
            "too",
        }

        # Try to extract from title first
        if title:
            # Remove special characters but keep meaningful ones like version numbers
            words = re.findall(r"\b[A-Za-z0-9]+(?:\.[0-9]+)?\b", title)

            # Score words by importance
            word_scores = []
            for word in words:
                if word.lower() in stop_words or len(word) < 2:
                    continue

                score = 0
                # Prefer proper nouns (capitalized)
                if word and word[0].isupper():
                    score += 3
                # Prefer longer words (more specific)
                score += min(len(word) / 3, 2)
                # Prefer words with numbers (versions, models)
                if any(c.isdigit() for c in word):
                    score += 2
                # Prefer technical/brand-like terms (all caps or mixed case)
                if word.isupper() and len(word) > 1:
                    score += 4
                if sum(1 for c in word if c.isupper()) > 1:
                    score += 2

                word_scores.append((word, score))

            # Sort by score and get top words
            word_scores.sort(key=lambda x: x[1], reverse=True)

            if word_scores:
                # Get the highest scoring word(s)
                if len(word_scores) >= 2:
                    # Check if two words together are short enough
                    first = word_scores[0][0]
                    second = word_scores[1][0]
                    # Prefer a single distinctive word if the combo is too long
                    if len(first) + len(second) < 15:
                        return f"{first} {second}"
                    else:
                        return first
                else:
                    return word_scores[0][0]

        # Fallback to domain name from URL
        if url:
            try:
                domain = urlparse(url).netloc
                # Remove www. and common TLDs
                domain = re.sub(r"^www\.", "", domain)
                domain = re.sub(
                    r"\.(com|org|net|io|edu|gov|co|uk|ai|app|dev).*$", "", domain
                )

                # If domain has meaningful parts, use them
                parts = re.split(r"[.-]", domain)
                # Filter out generic parts
                meaningful = [
                    p
                    for p in parts
                    if len(p) > 2 and p not in {"www", "blog", "docs", "api"}
                ]
                if meaningful:
                    return meaningful[0]
            except:
                pass

        # Last fallback - just return a generic term
        return "search"

    def generate_queries_for_bookmark(
        self, bookmark: Dict[str, Any], max_queries: int = 5
    ) -> List[str]:
        """Generate search queries that should return this bookmark"""

        content = bookmark.get("content", "")
        title = bookmark.get("name", "")
        url = bookmark.get("url", "")

        if not content:
            return []

        # Always generate the requested number of queries (default 5)
        num_queries = max_queries

        # Extract short query programmatically from title/URL - don't use LLM
        short_query = self._extract_short_query(title, url)

        # Debug: ensure we always have a short query
        if not short_query and title:
            # Emergency fallback - just take first word that's not a stop word
            import re

            words = re.findall(r"\b[A-Za-z0-9]+\b", title)
            for w in words:
                if len(w) > 2:
                    short_query = w
                    break
        if not short_query:
            short_query = "info"  # absolute last resort

        # Then generate longer queries if needed
        longer_prompt = (
            f"""
Generate {num_queries - 1} different search queries (6 words maximum length for each) for this webpage.

Title: {title}
Content: {content[:500]}

List {num_queries - 1} queries only:"""
            if num_queries > 1
            else None
        )

        try:
            all_queries = []

            # Add the programmatically extracted short query
            if short_query:
                all_queries.append(short_query)
            else:
                # Fallback if extraction failed - just use first significant word from title
                import re

                words = re.findall(r"\b[A-Za-z0-9]+\b", title) if title else []
                if words:
                    all_queries.append(words[0])

            # Generate longer queries if needed
            if longer_prompt and num_queries > 1:
                response = self.client.generate(
                    model=self.model,
                    prompt=longer_prompt,
                    options={
                        "temperature": 0.7,
                        "top_p": 0.9,
                    },
                    keep_alive="1h",
                )
                longer_queries_text = response["response"].strip()
                longer_queries = [
                    q.strip() for q in longer_queries_text.split("\n") if q.strip()
                ]
                all_queries.extend(longer_queries)

            # Clean up ALL queries (remove numbering and parenthetical text)
            import re

            cleaned_queries = []

            # Clean all queries
            for q in all_queries:
                # First normalize Unicode characters
                import unicodedata

                q = unicodedata.normalize("NFKD", q)

                # Remove common numbering patterns
                q = q.lstrip("0123456789.-) ")
                # Remove quotes if present
                q = q.strip("\"'")
                # Replace all Unicode punctuation with ASCII equivalents
                # Smart quotes and apostrophes
                q = q.replace("\u2019", "'")  # right single quote (apostrophe)
                q = q.replace("\u2018", "'")  # left single quote
                q = q.replace("\u201c", '"')  # left double quote
                q = q.replace("\u201d", '"')  # right double quote
                q = q.replace("\u201e", '"')  # double low quote
                q = q.replace("\u201a", "'")  # single low quote
                q = q.replace("\u201b", "'")  # single high-reversed quote
                # Dashes
                q = q.replace("\u2013", "-")  # en-dash
                q = q.replace("\u2014", "--")  # em-dash
                q = q.replace("\u2015", "--")  # horizontal bar
                # Other punctuation
                q = q.replace("\u2026", "...")  # ellipsis
                q = q.replace("\u00a0", " ")  # non-breaking space
                q = q.replace("\u202f", " ")  # narrow non-breaking space
                q = q.replace("\u2009", " ")  # thin space
                # Remove parenthetical text (anything in parentheses) - do this after other replacements
                q = re.sub(r"\s*\([^)]*\)", "", q).strip()
                # Also remove any trailing ? or ! that might have been part of a pattern
                q = q.rstrip("?!")

                # Skip instructional/introductory text that LLM generates
                if any(
                    skip_phrase in q.lower()
                    for skip_phrase in [
                        "here are",
                        "based on",
                        "search queries",
                        "different queries",
                        "webpage content",
                        "varying in",
                        "specificity:",
                        "generated queries",
                        "following are",
                        "below are",
                        "queries based on",
                        "search terms",
                        "keeping them to",
                        "word maximum",
                        "word limit",
                        "each within",
                        "limited to",
                        "words each",
                    ]
                ):
                    continue

                if q and len(q) > 2:
                    cleaned_queries.append(q)

            # CRITICAL: Ensure short query is first and we only return num_queries total
            # The short query should always be first in all_queries
            final_queries = []
            if cleaned_queries:
                # Always include the first query (our short query)
                final_queries.append(cleaned_queries[0])
                # Add the rest up to num_queries total
                for q in cleaned_queries[1:num_queries]:
                    final_queries.append(q)

            return final_queries

        except Exception as e:
            print(f"Error generating queries for bookmark {bookmark.get('id')}: {e}")
            return []

    def generate_queries_for_samples(
        self,
        samples: List[Dict[str, Any]],
        output_path: str = "data/generated_queries.json",
        resume: bool = True,
    ) -> Dict[str, List[str]]:
        """Generate queries for all sampled bookmarks with incremental saving"""

        output_file = Path(output_path)
        output_file.parent.mkdir(parents=True, exist_ok=True)

        # Load existing queries if resuming
        existing_queries = {}
        if resume and output_file.exists():
            try:
                with open(output_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    existing_queries = data.get("queries", {})
                    print(
                        f"Resuming from existing file with {len(existing_queries)} bookmarks already processed"
                    )
            except Exception as e:
                print(f"Could not load existing queries: {e}")

        queries_map = existing_queries.copy()

        # Filter out already processed bookmarks
        bookmarks_to_process = []
        for bookmark in samples:
            bookmark_id = bookmark.get(
                "id", bookmark.get("guid", str(hash(bookmark["url"])))
            )
            if bookmark_id not in queries_map:
                bookmarks_to_process.append(bookmark)

        if not bookmarks_to_process:
            print(f"All {len(samples)} bookmarks already have queries generated")
            return queries_map

        print(
            f"Generating queries for {len(bookmarks_to_process)} bookmarks (skipping {len(samples) - len(bookmarks_to_process)} already processed)..."
        )

        for bookmark in tqdm(bookmarks_to_process):
            bookmark_id = bookmark.get(
                "id", bookmark.get("guid", str(hash(bookmark["url"])))
            )
            queries = self.generate_queries_for_bookmark(bookmark)

            if queries:
                queries_map[bookmark_id] = queries

                # Save incrementally after each bookmark
                self._save_queries_incremental(queries_map, output_file)

                print(
                    f"Generated {len(queries)} queries for bookmark {bookmark_id} (total: {len(queries_map)}/{len(samples)})"
                )

        print(f"Generated queries for {len(queries_map)} bookmarks total")
        return queries_map

    def _save_queries_incremental(
        self, queries_map: Dict[str, List[str]], output_file: Path
    ):
        """Save queries incrementally to disk"""
        data = {
            "model": self.model,
            "total_bookmarks": len(queries_map),
            "total_queries": sum(len(queries) for queries in queries_map.values()),
            "queries": queries_map,
        }

        # Write to temporary file first, then rename (atomic operation)
        temp_file = output_file.with_suffix(".tmp")
        with open(temp_file, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2, ensure_ascii=False)

        # Rename temp file to actual file
        temp_file.replace(output_file)

    def save_queries(
        self,
        queries_map: Dict[str, List[str]],
        output_path: str = "data/generated_queries.json",
    ):
        """Save generated queries to file"""
        output_file = Path(output_path)
        output_file.parent.mkdir(parents=True, exist_ok=True)

        # Include metadata
        data = {
            "model": self.model,
            "total_bookmarks": len(queries_map),
            "total_queries": sum(len(queries) for queries in queries_map.values()),
            "queries": queries_map,
        }

        with open(output_file, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2, ensure_ascii=False)

        print(f"Saved queries to {output_file}")
        return output_file

    def load_queries(
        self, input_path: str = "data/generated_queries.json"
    ) -> Dict[str, List[str]]:
        """Load queries from file"""
        with open(input_path, "r", encoding="utf-8") as f:
            data = json.load(f)
            return data.get("queries", data)
