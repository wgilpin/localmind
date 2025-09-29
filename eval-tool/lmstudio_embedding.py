"""LM Studio embedding client for the eval tool"""
import requests
import json
from typing import List, Union
import time

class LMStudioEmbedding:
    """LM Studio embedding client that mimics SentenceTransformer interface"""

    def __init__(self, model_name: str, base_url: str = "http://localhost:1234"):
        self.model_name = model_name
        self.base_url = base_url.rstrip('/')

        # Test connection
        self._test_connection()

    def _test_connection(self):
        """Test if LM Studio is running and has embedding models"""
        try:
            # Check if LM Studio is running
            response = requests.get(f"{self.base_url}/v1/models", timeout=5)
            response.raise_for_status()

            models = response.json().get('data', [])
            model_ids = [model['id'] for model in models]

            print(f"LM Studio connection successful. Available models: {model_ids}")

            # Check if we have an embedding model
            embedding_models = [m for m in model_ids if 'embed' in m.lower() or 'nomic' in m.lower()]
            if embedding_models:
                print(f"Found embedding models: {embedding_models}")
                # Use the first embedding model if no specific one provided
                if self.model_name == "auto":
                    self.model_name = embedding_models[0]
                    print(f"Using embedding model: {self.model_name}")
            else:
                print(f"Warning: No embedding models found in LM Studio. Make sure to load an embedding model.")

        except requests.RequestException as e:
            raise ConnectionError(f"Cannot connect to LM Studio at {self.base_url}. "
                                f"Make sure LM Studio server is running. Error: {e}")

    def encode(self, texts: Union[str, List[str]], batch_size: int = 100, **kwargs) -> Union[List[float], List[List[float]]]:
        """
        Generate embeddings for text(s) using LM Studio's OpenAI-compatible API.

        Args:
            texts: Single string or list of strings to encode
            batch_size: Number of texts to process in a single API call
            **kwargs: Additional arguments (ignored for compatibility)

        Returns:
            Single embedding (for single string) or list of embeddings (for list of strings)
        """
        if isinstance(texts, str):
            # Single text
            embedding = self._get_embedding_batch([texts])[0]
            return embedding
        else:
            # Process in batches for efficiency
            all_embeddings = []
            for i in range(0, len(texts), batch_size):
                batch = texts[i:i + batch_size]
                batch_embeddings = self._get_embedding_batch(batch)
                all_embeddings.extend(batch_embeddings)
            return all_embeddings

    def _get_embedding_batch(self, texts: List[str], max_retries: int = 3) -> List[List[float]]:
        """Get embeddings for a batch of texts with retry logic"""
        # Log the batch embedding call (only for large batches)
        batch_size = len(texts)
        if batch_size > 10:
            total_chars = sum(len(t) for t in texts)
            print(f"[EMBEDDING] Sending batch of {batch_size} texts to LM Studio (total {total_chars} chars)")

        for attempt in range(max_retries):
            try:
                # OpenAI API supports batch embedding with array input
                payload = {
                    "input": texts,  # Array of strings for batch processing
                    "model": self.model_name
                }

                response = requests.post(
                    f"{self.base_url}/v1/embeddings",
                    json=payload,
                    headers={"Content-Type": "application/json"},
                    timeout=60  # Increased timeout for batch processing
                )
                response.raise_for_status()

                result = response.json()

                if 'data' not in result or len(result['data']) == 0:
                    raise ValueError(f"No embeddings in response: {result}")

                # Extract embeddings from OpenAI-format response
                # The API returns embeddings in the same order as input
                embeddings = [item['embedding'] for item in sorted(result['data'], key=lambda x: x['index'])]

                if len(embeddings) != batch_size:
                    raise ValueError(f"Expected {batch_size} embeddings, got {len(embeddings)}")

                if batch_size > 10:
                    print(f"[EMBEDDING] Success: got {batch_size} embeddings, each {len(embeddings[0])}-dim")
                return embeddings

            except requests.RequestException as e:
                if attempt == max_retries - 1:
                    print(f"[EMBEDDING] FAILED after {max_retries} attempts: {e}")
                    raise RuntimeError(f"Failed to get embeddings after {max_retries} attempts: {e}")

                print(f"[EMBEDDING] Attempt {attempt + 1} failed, retrying in 1 second...{e}")
                time.sleep(1)

        raise RuntimeError(f"Failed to get embeddings after {max_retries} attempts")

    def _get_embedding(self, text: str, max_retries: int = 3) -> List[float]:
        """Get embedding for a single text with retry logic"""
        # Log the embedding call
        text_preview = text[:100].replace('\n', '\\n').replace('\r', '\\r')
        if len(text) > 100:
            text_preview += "..."
        print(f"[EMBEDDING] Sending to LM Studio: '{text_preview}' (length: {len(text)} chars)")

        for attempt in range(max_retries):
            try:
                payload = {
                    "input": text,
                    "model": self.model_name
                }

                response = requests.post(
                    f"{self.base_url}/v1/embeddings",
                    json=payload,
                    headers={"Content-Type": "application/json"},
                    timeout=30
                )
                response.raise_for_status()

                result = response.json()

                if 'data' not in result or len(result['data']) == 0:
                    raise ValueError(f"No embedding in response: {result}")

                # Extract embedding from OpenAI-format response
                embedding = result['data'][0]['embedding']
                print(f"[EMBEDDING] Success: got {len(embedding)}-dim vector for text length {len(text)}")
                return embedding

            except requests.RequestException as e:
                if attempt == max_retries - 1:
                    print(f"[EMBEDDING] FAILED after {max_retries} attempts: {e}")
                    raise RuntimeError(f"Failed to get embedding after {max_retries} attempts: {e}")

                print(f"[EMBEDDING] Attempt {attempt + 1} failed, retrying in 1 second...{e}")
                time.sleep(1)

        raise RuntimeError(f"Failed to get embedding after {max_retries} attempts")

    def __repr__(self):
        return f"LMStudioEmbedding(model='{self.model_name}', base_url='{self.base_url}')"